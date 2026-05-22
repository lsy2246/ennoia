use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use ennoia_kernel::{
    ExtensionSettingFieldSpec, ExtensionSettingFieldType, ExtensionSettingValue, SkillCheckAction,
    SkillCheckCategory, SkillCheckItem, SkillCheckItemStatus, SkillCheckResult, SkillConfig,
    SkillManifest, SkillReadinessSummary, SkillRegistryFile, SkillRuntimeStatus,
    SkillSettingsPayload, SkillSettingsRecord,
};
use ennoia_paths::RuntimePaths;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

const DEFAULT_SKILL_CHECK_TIMEOUT_MS: u64 = 15_000;
const SKILL_STATUS_FILE: &str = "status.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct SkillSettingsFile {
    #[serde(default)]
    values: BTreeMap<String, ExtensionSettingValue>,
}

pub fn load_skill_manifest(
    paths: &RuntimePaths,
    skill_id: &str,
    allow_dev_sources: bool,
) -> io::Result<SkillManifest> {
    let descriptor_path =
        resolve_skill_root(paths, skill_id, allow_dev_sources)?.join("skill.toml");
    let contents = fs::read_to_string(descriptor_path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

pub fn load_skill_settings(paths: &RuntimePaths, manifest: &SkillManifest) -> SkillSettingsRecord {
    SkillSettingsRecord {
        skill_id: manifest.id.clone(),
        values: load_effective_skill_settings(paths, manifest),
    }
}

pub fn validate_skill_settings_payload(
    manifest: &SkillManifest,
    payload: &SkillSettingsPayload,
) -> Result<(), String> {
    let declared = manifest
        .settings
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let declared_keys = declared.keys().copied().collect::<BTreeSet<_>>();

    for key in payload.values.keys() {
        if !declared_keys.contains(key.as_str()) {
            return Err(format!(
                "setting '{key}' is not declared by skill '{}'",
                manifest.id
            ));
        }
    }

    for field in &manifest.settings {
        if field.required
            && !payload.values.contains_key(&field.key)
            && field.default_value.is_none()
        {
            return Err(format!("required setting '{}' is missing", field.key));
        }
    }

    for (key, value) in &payload.values {
        let Some(field) = declared.get(key.as_str()) else {
            continue;
        };
        validate_setting_value(field, value)?;
    }

    Ok(())
}

pub fn save_skill_settings(
    paths: &RuntimePaths,
    manifest: &SkillManifest,
    payload: &SkillSettingsPayload,
) -> io::Result<SkillSettingsRecord> {
    let mut stored = read_skill_settings_file(paths.skill_config_file(&manifest.id))
        .unwrap_or_default()
        .values;
    for (key, value) in &payload.values {
        stored.insert(key.clone(), value.clone());
    }
    write_skill_settings_file(paths.skill_config_file(&manifest.id), &stored)?;
    Ok(load_skill_settings(paths, manifest))
}

pub fn load_skill_status(paths: &RuntimePaths, manifest: &SkillManifest) -> SkillCheckResult {
    if let Some(result) =
        config_missing_result(manifest, &load_effective_skill_settings(paths, manifest))
    {
        return result;
    }

    if let Some(mut result) = read_skill_status_file(skill_status_path(paths, &manifest.id)) {
        if manifest.diagnostics.manual_check {
            ensure_manual_check_action(&mut result);
        }
        return result;
    }

    if manifest.diagnostics.check.is_some() {
        let mut result = SkillCheckResult {
            status: SkillRuntimeStatus::Unknown,
            summary: if manifest.settings.is_empty() {
                "尚未执行检测。".to_string()
            } else {
                "配置已完成，等待检测。".to_string()
            },
            checked_at: None,
            items: Vec::new(),
            actions: Vec::new(),
        };
        if manifest.diagnostics.manual_check {
            ensure_manual_check_action(&mut result);
        }
        return result;
    }

    if manifest.settings.is_empty() {
        SkillCheckResult {
            status: SkillRuntimeStatus::Unknown,
            summary: "未定义配置或检测。".to_string(),
            checked_at: None,
            items: Vec::new(),
            actions: Vec::new(),
        }
    } else {
        SkillCheckResult {
            status: SkillRuntimeStatus::Ready,
            summary: "配置已完成。".to_string(),
            checked_at: None,
            items: Vec::new(),
            actions: Vec::new(),
        }
    }
}

pub fn load_skill_readiness_summary(
    paths: &RuntimePaths,
    manifest: &SkillManifest,
) -> SkillReadinessSummary {
    let result = load_skill_status(paths, manifest);
    SkillReadinessSummary {
        status: result.status,
        summary: result.summary,
        checked_at: result.checked_at,
    }
}

pub async fn run_skill_check(
    paths: &RuntimePaths,
    manifest: &SkillManifest,
    allow_dev_sources: bool,
) -> io::Result<SkillCheckResult> {
    let values = load_effective_skill_settings(paths, manifest);
    if let Some(mut result) = config_missing_result(manifest, &values) {
        result.checked_at = Some(now_iso());
        if manifest.diagnostics.manual_check {
            ensure_manual_check_action(&mut result);
        }
        write_skill_status_file(skill_status_path(paths, &manifest.id), &result)?;
        return Ok(result);
    }

    let Some(command) = manifest.diagnostics.check.as_ref() else {
        let result = SkillCheckResult {
            status: if manifest.settings.is_empty() {
                SkillRuntimeStatus::Unknown
            } else {
                SkillRuntimeStatus::Ready
            },
            summary: if manifest.settings.is_empty() {
                "未定义检测命令。".to_string()
            } else {
                "配置已完成。".to_string()
            },
            checked_at: Some(now_iso()),
            items: Vec::new(),
            actions: Vec::new(),
        };
        write_skill_status_file(skill_status_path(paths, &manifest.id), &result)?;
        return Ok(result);
    };

    let mut process =
        build_skill_check_command(paths, manifest, &values, command, allow_dev_sources)?;
    let timeout_ms = command.timeout_ms.unwrap_or(DEFAULT_SKILL_CHECK_TIMEOUT_MS);
    let output = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        process.output(),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "skill check timed out"))??;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let mut result = parse_skill_check_output(&stdout, &stderr, output.status.success())?;
    if result.checked_at.is_none() {
        result.checked_at = Some(now_iso());
    }
    if manifest.diagnostics.manual_check {
        ensure_manual_check_action(&mut result);
    }
    write_skill_status_file(skill_status_path(paths, &manifest.id), &result)?;
    Ok(result)
}

pub fn skill_config_with_readiness(paths: &RuntimePaths, mut skill: SkillConfig) -> SkillConfig {
    skill.readiness = load_skill_readiness_summary(
        paths,
        &SkillManifest {
            id: skill.id.clone(),
            version: skill.version.clone(),
            description: skill.description.clone(),
            mount: skill.mount.clone(),
            actions: skill.actions.clone(),
            settings: skill.settings.clone(),
            diagnostics: skill.diagnostics.clone(),
        },
    );
    skill
}

fn build_skill_check_command(
    paths: &RuntimePaths,
    manifest: &SkillManifest,
    values: &BTreeMap<String, ExtensionSettingValue>,
    diagnostics: &ennoia_kernel::SkillCommandSpec,
    allow_dev_sources: bool,
) -> io::Result<Command> {
    let skill_root = resolve_skill_root(paths, &manifest.id, allow_dev_sources)?;
    let entry_path = resolve_skill_entry_path(&skill_root, &diagnostics.entry)?;
    let runner = diagnostics.runner.trim().to_lowercase();
    let mut command = match runner.as_str() {
        "node" => {
            let mut item = Command::new("node");
            item.arg(&entry_path);
            item
        }
        "bun" => {
            let mut item = Command::new("bun");
            item.arg(&entry_path);
            item
        }
        "python" => {
            let mut item = Command::new("python");
            item.arg(&entry_path);
            item
        }
        "direct" => Command::new(&entry_path),
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported skill diagnostics runner '{other}'"),
            ));
        }
    };

    for arg in &diagnostics.args {
        command.arg(arg);
    }

    command.current_dir(&skill_root);
    command.env("ENNOIA_HOME", paths.home());
    command.env("ENNOIA_SKILL_ID", &manifest.id);
    command.env("ENNOIA_SKILL_ROOT", &skill_root);
    command.env("ENNOIA_SKILL_DATA_DIR", paths.skill_state_dir(&manifest.id));
    command.env(
        "ENNOIA_SKILL_CONFIG_JSON",
        serde_json::to_string(values).map_err(io::Error::other)?,
    );
    for (key, value) in values {
        command.env(
            format!("ENNOIA_SKILL_SETTING_{}", env_key(key)),
            setting_value_as_env(value),
        );
    }
    Ok(command)
}

fn resolve_skill_root(
    paths: &RuntimePaths,
    skill_id: &str,
    allow_dev_sources: bool,
) -> io::Result<PathBuf> {
    let registry = read_skill_registry(paths)?;
    if allow_dev_sources {
        for source in registry
            .dev_sources
            .into_iter()
            .filter(|source| source.enabled && source.id == skill_id)
        {
            let root = PathBuf::from(source.path);
            if root.join("skill.toml").exists() {
                return Ok(root);
            }
        }
    }

    let installed_root = paths.skill_dir(skill_id);
    if installed_root.join("skill.toml").exists() {
        return Ok(installed_root);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("skill '{skill_id}' not found"),
    ))
}

fn read_skill_registry(paths: &RuntimePaths) -> io::Result<SkillRegistryFile> {
    let path = paths.skills_registry_file();
    if !path.exists() {
        return Ok(SkillRegistryFile::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

fn resolve_skill_entry_path(skill_root: &Path, entry: &str) -> io::Result<PathBuf> {
    let candidate = skill_root.join(entry);
    let canonical_root = fs::canonicalize(skill_root)?;
    let canonical_entry = fs::canonicalize(&candidate)?;
    if !canonical_entry.starts_with(&canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "skill diagnostics entry must stay inside the skill root",
        ));
    }
    Ok(canonical_entry)
}

fn parse_skill_check_output(
    stdout: &str,
    stderr: &str,
    success: bool,
) -> io::Result<SkillCheckResult> {
    if !stdout.is_empty() {
        if let Ok(result) = serde_json::from_str::<SkillCheckResult>(stdout) {
            return Ok(result);
        }
    }

    let detail = if !stdout.is_empty() { stdout } else { stderr };
    Ok(SkillCheckResult {
        status: if success {
            SkillRuntimeStatus::Ready
        } else {
            SkillRuntimeStatus::Error
        },
        summary: if detail.is_empty() {
            if success {
                "检测通过。".to_string()
            } else {
                "检测失败。".to_string()
            }
        } else {
            detail
                .lines()
                .next()
                .unwrap_or("检测失败。")
                .trim()
                .to_string()
        },
        checked_at: Some(now_iso()),
        items: Vec::new(),
        actions: Vec::new(),
    })
}

fn config_missing_result(
    manifest: &SkillManifest,
    values: &BTreeMap<String, ExtensionSettingValue>,
) -> Option<SkillCheckResult> {
    let items = missing_required_items(&manifest.settings, values);
    if items.is_empty() {
        return None;
    }
    let missing_count = items.len();
    Some(SkillCheckResult {
        status: SkillRuntimeStatus::MissingConfig,
        summary: format!("缺少 {missing_count} 项配置。"),
        checked_at: None,
        items,
        actions: Vec::new(),
    })
}

fn missing_required_items(
    fields: &[ExtensionSettingFieldSpec],
    values: &BTreeMap<String, ExtensionSettingValue>,
) -> Vec<SkillCheckItem> {
    fields
        .iter()
        .filter(|field| field.required && !setting_is_present(field, values.get(&field.key)))
        .map(|field| SkillCheckItem {
            key: field.key.clone(),
            category: SkillCheckCategory::Config,
            label: field.label.fallback.clone(),
            status: SkillCheckItemStatus::Missing,
            required: true,
            message: Some("必填配置尚未完成。".to_string()),
            fix_hint: field
                .description
                .as_ref()
                .map(|description| description.fallback.clone()),
        })
        .collect()
}

fn setting_is_present(
    field: &ExtensionSettingFieldSpec,
    value: Option<&ExtensionSettingValue>,
) -> bool {
    match (&field.field_type, value) {
        (
            ExtensionSettingFieldType::Text
            | ExtensionSettingFieldType::Textarea
            | ExtensionSettingFieldType::Select,
            Some(ExtensionSettingValue::String(text)),
        ) => !text.trim().is_empty(),
        (ExtensionSettingFieldType::Number, Some(ExtensionSettingValue::Integer(_))) => true,
        (ExtensionSettingFieldType::Boolean, Some(ExtensionSettingValue::Boolean(_))) => true,
        _ => false,
    }
}

fn validate_setting_value(
    field: &ExtensionSettingFieldSpec,
    value: &ExtensionSettingValue,
) -> Result<(), String> {
    match (&field.field_type, value) {
        (ExtensionSettingFieldType::Boolean, ExtensionSettingValue::Boolean(_)) => Ok(()),
        (ExtensionSettingFieldType::Number, ExtensionSettingValue::Integer(_)) => Ok(()),
        (
            ExtensionSettingFieldType::Text
            | ExtensionSettingFieldType::Textarea
            | ExtensionSettingFieldType::Select,
            ExtensionSettingValue::String(text),
        ) => {
            if field.required && text.trim().is_empty() {
                return Err(format!("setting '{}' cannot be empty", field.key));
            }
            if matches!(field.field_type, ExtensionSettingFieldType::Select)
                && !field.options.is_empty()
                && !field.options.iter().any(|option| option.value == *text)
            {
                return Err(format!(
                    "setting '{}' has unsupported value '{}'",
                    field.key, text
                ));
            }
            Ok(())
        }
        _ => Err(format!(
            "setting '{}' does not match declared type '{:?}'",
            field.key, field.field_type
        )),
    }
}

fn load_effective_skill_settings(
    paths: &RuntimePaths,
    manifest: &SkillManifest,
) -> BTreeMap<String, ExtensionSettingValue> {
    let stored = read_skill_settings_file(paths.skill_config_file(&manifest.id))
        .unwrap_or_default()
        .values;
    let mut values = manifest
        .settings
        .iter()
        .filter_map(|item| {
            item.default_value
                .clone()
                .map(|value| (item.key.clone(), value))
        })
        .collect::<BTreeMap<_, _>>();
    for field in &manifest.settings {
        if let Some(value) = stored.get(&field.key).cloned() {
            values.insert(field.key.clone(), value);
        }
    }
    values
}

fn read_skill_settings_file(path: PathBuf) -> Option<SkillSettingsFile> {
    let contents = fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}

fn write_skill_settings_file(
    path: PathBuf,
    values: &BTreeMap<String, ExtensionSettingValue>,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        toml::to_string_pretty(&SkillSettingsFile {
            values: values.clone(),
        })
        .map_err(io::Error::other)?,
    )
}

fn read_skill_status_file(path: PathBuf) -> Option<SkillCheckResult> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_skill_status_file(path: PathBuf, result: &SkillCheckResult) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(result).map_err(io::Error::other)?,
    )
}

fn skill_status_path(paths: &RuntimePaths, skill_id: &str) -> PathBuf {
    paths.skill_state_dir(skill_id).join(SKILL_STATUS_FILE)
}

fn ensure_manual_check_action(result: &mut SkillCheckResult) {
    if result.actions.iter().any(|item| item.key == "recheck") {
        return;
    }
    result.actions.push(SkillCheckAction {
        key: "recheck".to_string(),
        label: "重新检测".to_string(),
        kind: "recheck".to_string(),
    });
}

fn env_key(value: &str) -> String {
    value
        .chars()
        .map(|item| {
            if item.is_ascii_alphanumeric() {
                item.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn setting_value_as_env(value: &ExtensionSettingValue) -> String {
    match value {
        ExtensionSettingValue::String(item) => item.clone(),
        ExtensionSettingValue::Integer(item) => item.to_string(),
        ExtensionSettingValue::Boolean(item) => item.to_string(),
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::{
        ExtensionSettingFieldType, LocalizedText, SkillActionConfig, SkillCommandSpec,
        SkillDiagnosticsSpec, SkillManifest, SkillMountConfig,
    };

    fn sample_manifest() -> SkillManifest {
        SkillManifest {
            id: "sample".to_string(),
            version: "1.0.0".to_string(),
            description: "sample".to_string(),
            mount: SkillMountConfig {
                mode: "auto".to_string(),
            },
            actions: vec![SkillActionConfig {
                id: "run".to_string(),
                description: String::new(),
                entry: "scripts/run.mjs".to_string(),
            }],
            settings: vec![ExtensionSettingFieldSpec {
                key: "api_key".to_string(),
                label: LocalizedText::new("skill.sample.api_key", "API Key"),
                description: None,
                field_type: ExtensionSettingFieldType::Text,
                placeholder: None,
                required: true,
                options: Vec::new(),
                default_value: None,
            }],
            diagnostics: SkillDiagnosticsSpec {
                manual_check: true,
                check: Some(SkillCommandSpec {
                    runner: "node".to_string(),
                    entry: "scripts/doctor.mjs".to_string(),
                    args: Vec::new(),
                    timeout_ms: None,
                }),
            },
        }
    }

    #[test]
    fn missing_config_result_marks_required_settings() {
        let manifest = sample_manifest();
        let result = config_missing_result(&manifest, &BTreeMap::new()).expect("missing result");
        assert_eq!(result.status, SkillRuntimeStatus::MissingConfig);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].key, "api_key");
    }

    #[test]
    fn validates_declared_skill_settings() {
        let manifest = sample_manifest();
        let mut payload = SkillSettingsPayload::default();
        payload.values.insert(
            "api_key".to_string(),
            ExtensionSettingValue::String("secret".to_string()),
        );
        assert!(validate_skill_settings_payload(&manifest, &payload).is_ok());
    }

    #[test]
    fn load_skill_manifest_prefers_enabled_dev_source() {
        let home = std::env::temp_dir().join(format!(
            "ennoia-skill-manifest-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let paths = RuntimePaths::new(&home);
        fs::create_dir_all(paths.skill_dir("sample")).expect("installed skill dir");
        fs::write(
            paths.skill_dir("sample").join("skill.toml"),
            r#"
id = "sample"
version = "1.0.0"
description = "installed"
"#,
        )
        .expect("write installed manifest");

        let dev_root = home.parent().unwrap_or(&home).join("sample-dev");
        fs::create_dir_all(&dev_root).expect("dev skill dir");
        fs::write(
            dev_root.join("skill.toml"),
            r#"
id = "sample"
version = "2.0.0"
description = "dev"
"#,
        )
        .expect("write dev manifest");
        fs::create_dir_all(paths.config_dir()).expect("config dir");
        fs::write(
            paths.skills_registry_file(),
            format!(
                r#"
[[dev_sources]]
id = "sample"
path = "{}"
enabled = true
"#,
                dev_root.to_string_lossy().replace('\\', "/")
            ),
        )
        .expect("write registry");

        let manifest = load_skill_manifest(&paths, "sample", true).expect("load manifest");

        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.description, "dev");

        let _ = fs::remove_dir_all(home);
        let _ = fs::remove_dir_all(dev_root);
    }
}
