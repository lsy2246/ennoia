use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ennoia_kernel::{
    CommandContribution, ExtensionCapabilities, ExtensionDiagnostic, ExtensionFrontendSpec,
    ExtensionHealth, ExtensionKind, ExtensionManifest, ExtensionRuntimeEvent, ExtensionSourceMode,
    HookContribution, LocaleContribution, PageContribution, PanelContribution,
    ProviderContribution, ResolvedBackendEntry, ResolvedFrontendEntry, ThemeContribution,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedExtensionSnapshot {
    pub id: String,
    pub name: String,
    pub kind: ExtensionKind,
    pub version: String,
    pub source_mode: ExtensionSourceMode,
    pub source_root: String,
    pub install_dir: String,
    pub generation: u64,
    pub health: ExtensionHealth,
    pub frontend: Option<ResolvedFrontendEntry>,
    pub backend: Option<ResolvedBackendEntry>,
    pub capabilities: ExtensionCapabilities,
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub locales: Vec<LocaleContribution>,
    pub commands: Vec<CommandContribution>,
    pub providers: Vec<ProviderContribution>,
    pub hooks: Vec<HookContribution>,
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPageContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub page: PageContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPanelContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub panel: PanelContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredThemeContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub theme: ThemeContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredLocaleContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub locale: LocaleContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredCommandContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub command: CommandContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredProviderContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub provider: ProviderContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredHookContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub hook: HookContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionRuntimeSnapshot {
    pub generation: u64,
    pub updated_at: String,
    pub extensions: Vec<ResolvedExtensionSnapshot>,
    pub pages: Vec<RegisteredPageContribution>,
    pub panels: Vec<RegisteredPanelContribution>,
    pub themes: Vec<RegisteredThemeContribution>,
    pub locales: Vec<RegisteredLocaleContribution>,
    pub commands: Vec<RegisteredCommandContribution>,
    pub providers: Vec<RegisteredProviderContribution>,
    pub hooks: Vec<RegisteredHookContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRuntimeConfig {
    pub attached_workspaces_file: PathBuf,
    pub package_extensions_dir: PathBuf,
    pub legacy_extensions_config_dir: PathBuf,
    pub logs_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedWorkspaceRecord {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct AttachedWorkspacesFile {
    #[serde(default)]
    workspaces: Vec<AttachedWorkspaceRecord>,
}

#[derive(Debug, Clone)]
pub struct ExtensionRuntime {
    config: ExtensionRuntimeConfig,
    state: Arc<RwLock<ExtensionRuntimeState>>,
}

#[derive(Debug)]
struct ExtensionRuntimeState {
    snapshot: ExtensionRuntimeSnapshot,
    events: Vec<ExtensionRuntimeEvent>,
    runners: BTreeMap<String, RunnerProcess>,
}

#[derive(Debug, Clone)]
struct RuntimeSource {
    root: PathBuf,
    source_mode: ExtensionSourceMode,
}

#[derive(Debug)]
struct RunnerProcess {
    command: String,
    child: Child,
}

#[derive(Debug, Serialize, Deserialize)]
struct LegacyExtensionConfigFile {
    #[serde(default)]
    enabled: bool,
    install_dir: String,
}

impl ExtensionRuntime {
    pub fn bootstrap(config: ExtensionRuntimeConfig) -> io::Result<Self> {
        ensure_parent_dir(&config.attached_workspaces_file)?;
        if !config.attached_workspaces_file.exists() {
            fs::write(&config.attached_workspaces_file, "workspaces = []\n")?;
        }
        fs::create_dir_all(&config.package_extensions_dir)?;
        fs::create_dir_all(&config.legacy_extensions_config_dir)?;

        let mut state = ExtensionRuntimeState {
            snapshot: empty_snapshot(),
            events: Vec::new(),
            runners: BTreeMap::new(),
        };
        let mut next = build_snapshot(&config, state.snapshot.generation + 1)?;
        reconcile_runners(&config, &mut state.runners, &mut next);
        state.push_replace(next, "runtime bootstrap");

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub fn snapshot(&self) -> ExtensionRuntimeSnapshot {
        self.state
            .read()
            .expect("extension runtime read lock")
            .snapshot
            .clone()
    }

    pub fn events(&self, limit: usize) -> Vec<ExtensionRuntimeEvent> {
        let state = self.state.read().expect("extension runtime read lock");
        let count = state.events.len();
        state.events[count.saturating_sub(limit)..].to_vec()
    }

    pub fn get(&self, extension_id: &str) -> Option<ResolvedExtensionSnapshot> {
        self.snapshot()
            .extensions
            .into_iter()
            .find(|item| item.id == extension_id)
    }

    pub fn diagnostics(&self, extension_id: &str) -> Vec<ExtensionDiagnostic> {
        self.get(extension_id)
            .map(|item| item.diagnostics)
            .unwrap_or_default()
    }

    pub fn refresh_from_disk(&self, summary: &str) -> io::Result<Option<ExtensionRuntimeSnapshot>> {
        let current = self.snapshot();
        let mut next = build_snapshot(&self.config, current.generation + 1)?;
        let mut state = self.state.write().expect("extension runtime write lock");
        reconcile_runners(&self.config, &mut state.runners, &mut next);
        if equivalent_snapshots(&current, &next) {
            return Ok(None);
        }

        state.push_replace(next.clone(), summary);
        Ok(Some(next))
    }

    pub fn reload_extension(
        &self,
        extension_id: &str,
    ) -> io::Result<Option<ResolvedExtensionSnapshot>> {
        let summary = format!("extension {extension_id} reloaded");
        let _ = self.refresh_from_disk(&summary)?;
        Ok(self.get(extension_id))
    }

    pub fn restart_extension(
        &self,
        extension_id: &str,
    ) -> io::Result<Option<ResolvedExtensionSnapshot>> {
        let summary = format!("extension {extension_id} restarted");
        let _ = self.refresh_from_disk(&summary)?;
        Ok(self.get(extension_id))
    }

    pub fn attach_workspace(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<ResolvedExtensionSnapshot> {
        let path = canonicalize_or_original(path.as_ref());
        let manifest = read_manifest_from_root(&path)?;
        let attached = read_attached_workspaces(&self.config.attached_workspaces_file)?;
        let mut workspaces = attached
            .workspaces
            .into_iter()
            .filter(|item| item.id != manifest.id)
            .collect::<Vec<_>>();
        workspaces.push(AttachedWorkspaceRecord {
            id: manifest.id.clone(),
            path: normalize_display_path(&path),
        });
        write_attached_workspaces(
            &self.config.attached_workspaces_file,
            &AttachedWorkspacesFile { workspaces },
        )?;

        self.refresh_from_disk(&format!("workspace {} attached", manifest.id))?;
        self.get(&manifest.id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("extension {} missing after attach", manifest.id),
            )
        })
    }

    pub fn detach_workspace(&self, extension_id: &str) -> io::Result<bool> {
        let attached = read_attached_workspaces(&self.config.attached_workspaces_file)?;
        let original_len = attached.workspaces.len();
        let workspaces = attached
            .workspaces
            .into_iter()
            .filter(|item| item.id != extension_id)
            .collect::<Vec<_>>();
        if workspaces.len() == original_len {
            return Ok(false);
        }

        write_attached_workspaces(
            &self.config.attached_workspaces_file,
            &AttachedWorkspacesFile { workspaces },
        )?;
        let _ = self.refresh_from_disk(&format!("workspace {extension_id} detached"))?;
        Ok(true)
    }

    pub fn set_legacy_extension_enabled(
        &self,
        extension_id: &str,
        install_dir: &str,
        enabled: bool,
    ) -> io::Result<Option<ResolvedExtensionSnapshot>> {
        let config_path = self
            .config
            .legacy_extensions_config_dir
            .join(format!("{extension_id}.toml"));
        let payload = LegacyExtensionConfigFile {
            enabled,
            install_dir: install_dir.to_string(),
        };
        fs::write(
            config_path,
            toml::to_string_pretty(&payload).map_err(io::Error::other)?,
        )?;
        let summary = if enabled {
            format!("extension {extension_id} enabled")
        } else {
            format!("extension {extension_id} disabled")
        };
        let _ = self.refresh_from_disk(&summary)?;
        Ok(self.get(extension_id))
    }
}

impl ExtensionRuntimeState {
    fn push_replace(&mut self, snapshot: ExtensionRuntimeSnapshot, summary: &str) {
        for extension in &snapshot.extensions {
            self.events.push(ExtensionRuntimeEvent {
                event_id: format!("evt-{}", unique_suffix()),
                extension_id: Some(extension.id.clone()),
                generation: snapshot.generation,
                event: match extension.health {
                    ExtensionHealth::Ready => "extension.ready",
                    ExtensionHealth::Failed => "extension.failed",
                    ExtensionHealth::Degraded => "extension.degraded",
                    ExtensionHealth::Stopped => "extension.stopped",
                    ExtensionHealth::Discovering => "extension.discovering",
                    ExtensionHealth::Resolving => "extension.resolving",
                }
                .to_string(),
                health: Some(extension.health.clone()),
                summary: format!("{} ({})", extension.id, extension.name),
                diagnostics: extension.diagnostics.clone(),
                occurred_at: now_string(),
            });
        }
        self.events.push(ExtensionRuntimeEvent {
            event_id: format!("evt-{}", unique_suffix()),
            extension_id: None,
            generation: snapshot.generation,
            event: "extension.graph_swapped".to_string(),
            health: None,
            summary: summary.to_string(),
            diagnostics: Vec::new(),
            occurred_at: now_string(),
        });
        if self.events.len() > 256 {
            let drain = self.events.len() - 256;
            self.events.drain(0..drain);
        }
        self.snapshot = snapshot;
    }
}

impl ResolvedExtensionSnapshot {
    fn page_rows(&self) -> Vec<RegisteredPageContribution> {
        self.pages
            .iter()
            .cloned()
            .map(|page| RegisteredPageContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                page,
            })
            .collect()
    }

    fn panel_rows(&self) -> Vec<RegisteredPanelContribution> {
        self.panels
            .iter()
            .cloned()
            .map(|panel| RegisteredPanelContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                panel,
            })
            .collect()
    }

    fn theme_rows(&self) -> Vec<RegisteredThemeContribution> {
        self.themes
            .iter()
            .cloned()
            .map(|theme| RegisteredThemeContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                theme,
            })
            .collect()
    }

    fn locale_rows(&self) -> Vec<RegisteredLocaleContribution> {
        self.locales
            .iter()
            .cloned()
            .map(|locale| RegisteredLocaleContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                locale,
            })
            .collect()
    }

    fn command_rows(&self) -> Vec<RegisteredCommandContribution> {
        self.commands
            .iter()
            .cloned()
            .map(|command| RegisteredCommandContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                command,
            })
            .collect()
    }

    fn provider_rows(&self) -> Vec<RegisteredProviderContribution> {
        self.providers
            .iter()
            .cloned()
            .map(|provider| RegisteredProviderContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                provider,
            })
            .collect()
    }

    fn hook_rows(&self) -> Vec<RegisteredHookContribution> {
        self.hooks
            .iter()
            .cloned()
            .map(|hook| RegisteredHookContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                hook,
            })
            .collect()
    }
}

fn build_snapshot(
    config: &ExtensionRuntimeConfig,
    generation: u64,
) -> io::Result<ExtensionRuntimeSnapshot> {
    let mut resolved_by_id = BTreeMap::<String, ResolvedExtensionSnapshot>::new();
    for source in discover_sources(config)? {
        let resolved = resolve_source(source, generation);
        match resolved_by_id.get(&resolved.id) {
            Some(existing) if source_priority(existing) >= source_priority(&resolved) => {}
            _ => {
                resolved_by_id.insert(resolved.id.clone(), resolved);
            }
        }
    }
    let mut extensions = resolved_by_id.into_values().collect::<Vec<_>>();
    extensions.sort_by(|left, right| left.id.cmp(&right.id));

    let mut pages = Vec::new();
    let mut panels = Vec::new();
    let mut themes = Vec::new();
    let mut locales = Vec::new();
    let mut commands = Vec::new();
    let mut providers = Vec::new();
    let mut hooks = Vec::new();

    for extension in &extensions {
        pages.extend(extension.page_rows());
        panels.extend(extension.panel_rows());
        themes.extend(extension.theme_rows());
        locales.extend(extension.locale_rows());
        commands.extend(extension.command_rows());
        providers.extend(extension.provider_rows());
        hooks.extend(extension.hook_rows());
    }

    Ok(ExtensionRuntimeSnapshot {
        generation,
        updated_at: now_string(),
        extensions,
        pages,
        panels,
        themes,
        locales,
        commands,
        providers,
        hooks,
    })
}

fn reconcile_runners(
    config: &ExtensionRuntimeConfig,
    runners: &mut BTreeMap<String, RunnerProcess>,
    snapshot: &mut ExtensionRuntimeSnapshot,
) {
    let desired = snapshot
        .extensions
        .iter()
        .filter_map(|extension| {
            extension.backend.as_ref().and_then(|backend| {
                backend
                    .command
                    .clone()
                    .map(|command| (extension.id.clone(), command))
            })
        })
        .collect::<BTreeMap<_, _>>();

    let stale = runners
        .keys()
        .filter(|id| !desired.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    for id in stale {
        if let Some(mut runner) = runners.remove(&id) {
            let _ = runner.child.kill();
            let _ = runner.child.wait();
        }
    }

    for extension in &mut snapshot.extensions {
        let Some(backend) = extension.backend.as_mut() else {
            continue;
        };
        let Some(command) = backend.command.clone() else {
            continue;
        };

        let restart_needed = match runners.get_mut(&extension.id) {
            Some(runner) if runner.command == command => match runner.child.try_wait() {
                Ok(Some(_)) => true,
                Ok(None) => {
                    backend.pid = Some(runner.child.id());
                    backend.status = "ready".to_string();
                    false
                }
                Err(error) => {
                    extension.diagnostics.push(diagnostic(
                        "warn",
                        "runner state check failed",
                        Some(error.to_string()),
                    ));
                    true
                }
            },
            Some(_) => true,
            None => true,
        };

        if restart_needed {
            if let Some(mut old) = runners.remove(&extension.id) {
                let _ = old.child.kill();
                let _ = old.child.wait();
            }
            match spawn_runner(
                config,
                &extension.id,
                &command,
                Path::new(&extension.source_root),
            ) {
                Ok(runner) => {
                    backend.pid = Some(runner.child.id());
                    backend.status = "ready".to_string();
                    runners.insert(extension.id.clone(), runner);
                }
                Err(error) => {
                    backend.status = "failed".to_string();
                    extension.health = ExtensionHealth::Failed;
                    extension.diagnostics.push(diagnostic(
                        "error",
                        "runner start failed",
                        Some(error.to_string()),
                    ));
                }
            }
        }
    }
}

fn spawn_runner(
    config: &ExtensionRuntimeConfig,
    extension_id: &str,
    command: &str,
    cwd: &Path,
) -> io::Result<RunnerProcess> {
    fs::create_dir_all(&config.logs_dir)?;
    let log_path = config.logs_dir.join(format!("{extension_id}.log"));
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let mut process = if cfg!(windows) {
        let mut item = Command::new("powershell.exe");
        item.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(command);
        item
    } else {
        let mut item = Command::new("sh");
        item.arg("-lc").arg(command);
        item
    };

    let child = process
        .current_dir(cwd)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;

    Ok(RunnerProcess {
        command: command.to_string(),
        child,
    })
}

impl Drop for ExtensionRuntimeState {
    fn drop(&mut self) {
        for (_, mut runner) in std::mem::take(&mut self.runners) {
            let _ = runner.child.kill();
            let _ = runner.child.wait();
        }
    }
}

fn source_priority(extension: &ResolvedExtensionSnapshot) -> u8 {
    match extension.source_mode {
        ExtensionSourceMode::Workspace => 2,
        ExtensionSourceMode::Package => 1,
    }
}

fn discover_sources(config: &ExtensionRuntimeConfig) -> io::Result<Vec<RuntimeSource>> {
    let mut ordered = BTreeMap::<String, RuntimeSource>::new();

    let attached = read_attached_workspaces(&config.attached_workspaces_file)?;
    for workspace in attached.workspaces {
        let root = PathBuf::from(&workspace.path);
        ordered.insert(
            normalize_display_path(&root),
            RuntimeSource {
                root,
                source_mode: ExtensionSourceMode::Workspace,
            },
        );
    }

    if config.package_extensions_dir.exists() {
        for entry in fs::read_dir(&config.package_extensions_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let root = entry.path();
            ordered.insert(
                normalize_display_path(&root),
                RuntimeSource {
                    root,
                    source_mode: ExtensionSourceMode::Package,
                },
            );
        }
    }

    if config.legacy_extensions_config_dir.exists() {
        for entry in fs::read_dir(&config.legacy_extensions_config_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let contents = fs::read_to_string(entry.path())?;
            let item: LegacyExtensionConfigFile =
                toml::from_str(&contents).map_err(io::Error::other)?;
            if !item.enabled {
                continue;
            }
            let root = PathBuf::from(item.install_dir.replace("~/.ennoia", ""));
            let root = if root.is_absolute() {
                root
            } else {
                config
                    .legacy_extensions_config_dir
                    .parent()
                    .and_then(Path::parent)
                    .unwrap_or_else(|| Path::new("."))
                    .join(root)
            };
            let normalized = normalize_display_path(&root);
            ordered.entry(normalized).or_insert(RuntimeSource {
                root,
                source_mode: ExtensionSourceMode::Package,
            });
        }
    }

    Ok(ordered.into_values().collect())
}

fn resolve_source(source: RuntimeSource, generation: u64) -> ResolvedExtensionSnapshot {
    match read_manifest_from_root(&source.root) {
        Ok(manifest) => resolve_manifest(source, manifest, generation),
        Err(error) => failed_extension_snapshot(source, generation, error),
    }
}

fn resolve_manifest(
    source: RuntimeSource,
    manifest: ExtensionManifest,
    generation: u64,
) -> ResolvedExtensionSnapshot {
    let install_dir = normalize_display_path(&source.root);
    let source_root = install_dir.clone();
    let capabilities = manifest.effective_capabilities();
    let mut diagnostics = Vec::new();
    let frontend = resolve_frontend(
        &source.root,
        &source.source_mode,
        &manifest.frontend,
        &manifest,
    )
    .map_err(|error| {
        diagnostics.push(diagnostic(
            "warn",
            "frontend resolution failed",
            Some(error.to_string()),
        ));
    })
    .ok()
    .flatten();
    let backend = resolve_backend(&source.root, &source.source_mode, &manifest)
        .map_err(|error| {
            diagnostics.push(diagnostic(
                "warn",
                "backend resolution failed",
                Some(error.to_string()),
            ));
        })
        .ok()
        .flatten();

    if frontend.is_none() && backend.is_none() {
        diagnostics.push(diagnostic(
            "warn",
            "extension has no resolved frontend or backend entry",
            None,
        ));
    }

    let health = if diagnostics.iter().any(|item| item.level == "error") {
        ExtensionHealth::Failed
    } else if diagnostics.iter().any(|item| item.level == "warn") {
        ExtensionHealth::Degraded
    } else {
        ExtensionHealth::Ready
    };

    ResolvedExtensionSnapshot {
        id: manifest.id.clone(),
        name: manifest.display_name(),
        kind: manifest.kind,
        version: manifest.version,
        source_mode: source.source_mode,
        source_root,
        install_dir,
        generation,
        health,
        frontend,
        backend,
        capabilities,
        pages: manifest.contributes.pages,
        panels: manifest.contributes.panels,
        themes: manifest.contributes.themes,
        locales: manifest.contributes.locales,
        commands: manifest.contributes.commands,
        providers: manifest.contributes.providers,
        hooks: manifest.contributes.hooks,
        diagnostics,
    }
}

fn resolve_frontend(
    root: &Path,
    source_mode: &ExtensionSourceMode,
    frontend: &ExtensionFrontendSpec,
    manifest: &ExtensionManifest,
) -> io::Result<Option<ResolvedFrontendEntry>> {
    if *source_mode == ExtensionSourceMode::Workspace {
        if let Some(dev_url) = frontend.dev_url.clone() {
            return Ok(Some(ResolvedFrontendEntry {
                kind: "url".to_string(),
                entry: dev_url,
                hmr: frontend.hmr,
            }));
        }
        if let Some(entry) = frontend.entry.clone() {
            return Ok(Some(ResolvedFrontendEntry {
                kind: "module".to_string(),
                entry: normalize_display_path(&root.join(entry)),
                hmr: frontend.hmr,
            }));
        }
    }

    if let Some(bundle) = manifest
        .build
        .frontend_bundle
        .clone()
        .or_else(|| manifest.frontend_bundle.clone())
    {
        return Ok(Some(ResolvedFrontendEntry {
            kind: "file".to_string(),
            entry: normalize_display_path(&root.join(bundle)),
            hmr: frontend.hmr,
        }));
    }

    Ok(None)
}

fn resolve_backend(
    root: &Path,
    source_mode: &ExtensionSourceMode,
    manifest: &ExtensionManifest,
) -> io::Result<Option<ResolvedBackendEntry>> {
    if *source_mode == ExtensionSourceMode::Workspace {
        if let Some(command) = manifest.backend.dev_command.clone() {
            let entry = manifest
                .backend
                .entry
                .clone()
                .map(|item| normalize_display_path(&root.join(item)))
                .unwrap_or_else(|| normalize_display_path(root));
            return Ok(Some(ResolvedBackendEntry {
                kind: "process".to_string(),
                runtime: manifest
                    .backend
                    .runtime
                    .clone()
                    .unwrap_or_else(|| "node".to_string()),
                entry,
                command: Some(command),
                healthcheck: manifest.backend.healthcheck.clone(),
                status: "ready".to_string(),
                pid: None,
            }));
        }
        if let Some(entry) = manifest.backend.entry.clone() {
            return Ok(Some(ResolvedBackendEntry {
                kind: "module".to_string(),
                runtime: manifest
                    .backend
                    .runtime
                    .clone()
                    .unwrap_or_else(|| "node".to_string()),
                entry: normalize_display_path(&root.join(entry)),
                command: None,
                healthcheck: manifest.backend.healthcheck.clone(),
                status: "ready".to_string(),
                pid: None,
            }));
        }
    }

    if let Some(bundle) = manifest
        .build
        .backend_bundle
        .clone()
        .or_else(|| manifest.backend_entry.clone())
    {
        let resolved_entry = normalize_display_path(&root.join(bundle));
        let runtime = manifest
            .backend
            .runtime
            .clone()
            .unwrap_or_else(|| "node".to_string());
        return Ok(Some(ResolvedBackendEntry {
            kind: "file".to_string(),
            runtime: runtime.clone(),
            entry: resolved_entry.clone(),
            command: Some(match runtime.as_str() {
                "bun" => format!("bun \"{}\"", resolved_entry),
                "deno" => format!("deno run \"{}\"", resolved_entry),
                _ => format!("node \"{}\"", resolved_entry),
            }),
            healthcheck: manifest.backend.healthcheck.clone(),
            status: "ready".to_string(),
            pid: None,
        }));
    }

    Ok(None)
}

fn failed_extension_snapshot(
    source: RuntimeSource,
    generation: u64,
    error: io::Error,
) -> ResolvedExtensionSnapshot {
    let source_root = normalize_display_path(&source.root);
    let id = source
        .root
        .file_name()
        .map(|item| item.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    ResolvedExtensionSnapshot {
        id: id.clone(),
        name: id,
        kind: ExtensionKind::SystemExtension,
        version: "0.0.0".to_string(),
        source_mode: source.source_mode,
        source_root: source_root.clone(),
        install_dir: source_root,
        generation,
        health: ExtensionHealth::Failed,
        frontend: None,
        backend: None,
        capabilities: ExtensionCapabilities::default(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        commands: Vec::new(),
        providers: Vec::new(),
        hooks: Vec::new(),
        diagnostics: vec![diagnostic(
            "error",
            "descriptor resolution failed",
            Some(error.to_string()),
        )],
    }
}

fn read_manifest_from_root(root: &Path) -> io::Result<ExtensionManifest> {
    let descriptor_path = descriptor_path(root).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no extension descriptor found under {}", root.display()),
        )
    })?;
    let contents = fs::read_to_string(descriptor_path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

fn descriptor_path(root: &Path) -> Option<PathBuf> {
    [
        root.join("ennoia.extension.toml"),
        root.join("manifest.toml"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn read_attached_workspaces(path: &Path) -> io::Result<AttachedWorkspacesFile> {
    if !path.exists() {
        return Ok(AttachedWorkspacesFile::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

fn write_attached_workspaces(path: &Path, file: &AttachedWorkspacesFile) -> io::Result<()> {
    ensure_parent_dir(path)?;
    fs::write(
        path,
        toml::to_string_pretty(file).map_err(io::Error::other)?,
    )
}

fn equivalent_snapshots(
    current: &ExtensionRuntimeSnapshot,
    next: &ExtensionRuntimeSnapshot,
) -> bool {
    normalize_extensions(&current.extensions) == normalize_extensions(&next.extensions)
        && current.pages == next.pages
        && current.panels == next.panels
        && current.themes == next.themes
        && current.locales == next.locales
        && current.commands == next.commands
        && current.providers == next.providers
        && current.hooks == next.hooks
}

fn normalize_extensions(
    extensions: &[ResolvedExtensionSnapshot],
) -> Vec<ResolvedExtensionSnapshot> {
    extensions
        .iter()
        .cloned()
        .map(|mut extension| {
            extension.generation = 0;
            extension
        })
        .collect()
}

fn empty_snapshot() -> ExtensionRuntimeSnapshot {
    ExtensionRuntimeSnapshot {
        generation: 0,
        updated_at: now_string(),
        extensions: Vec::new(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        commands: Vec::new(),
        providers: Vec::new(),
        hooks: Vec::new(),
    }
}

fn diagnostic(level: &str, summary: &str, detail: Option<String>) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        level: level.to_string(),
        summary: summary.to_string(),
        detail,
        at: now_string(),
    }
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|item| item.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|item| item.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn canonicalize_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn normalize_display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_snapshot_flattens_contributions() {
        let root = unique_test_dir("runtime-snapshot");
        let ext_dir = root.join("observatory");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::write(ext_dir.join("ennoia.extension.toml"), sample_descriptor())
            .expect("write descriptor");

        let config = ExtensionRuntimeConfig {
            attached_workspaces_file: root.join("attached/workspaces.toml"),
            package_extensions_dir: root.join("packages"),
            legacy_extensions_config_dir: root.join("legacy"),
            logs_dir: root.join("logs"),
        };
        fs::create_dir_all(&config.package_extensions_dir).expect("create packages");
        fs::create_dir_all(
            config
                .attached_workspaces_file
                .parent()
                .expect("attached parent"),
        )
        .expect("create attached parent");
        fs::write(
            &config.attached_workspaces_file,
            format!(
                "workspaces = [{{ id = \"observatory\", path = \"{}\" }}]\n",
                normalize_display_path(&ext_dir)
            ),
        )
        .expect("write attached");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.extensions.len(), 1);
        assert_eq!(snapshot.pages.len(), 1);
        assert_eq!(snapshot.panels.len(), 1);
        assert_eq!(snapshot.locales.len(), 2);
        assert_eq!(snapshot.commands.len(), 1);
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.hooks.len(), 1);
        assert_eq!(snapshot.extensions[0].health, ExtensionHealth::Ready);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn attach_workspace_updates_runtime_snapshot() {
        let root = unique_test_dir("runtime-attach");
        let ext_dir = root.join("foo");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::write(
            ext_dir.join("ennoia.extension.toml"),
            sample_descriptor_for("foo"),
        )
        .expect("write descriptor");

        let config = ExtensionRuntimeConfig {
            attached_workspaces_file: root.join("attached/workspaces.toml"),
            package_extensions_dir: root.join("packages"),
            legacy_extensions_config_dir: root.join("legacy"),
            logs_dir: root.join("logs"),
        };
        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let attached = runtime
            .attach_workspace(&ext_dir)
            .expect("attach workspace");
        assert_eq!(attached.id, "foo");
        assert_eq!(runtime.snapshot().extensions.len(), 1);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    fn sample_descriptor() -> String {
        sample_descriptor_for("observatory")
    }

    fn sample_descriptor_for(id: &str) -> String {
        format!(
            r##"
id = "{id}"
name = "Observatory"
kind = "extension"
version = "0.1.0"

[source]
mode = "workspace"
root = "."
workspace = true

[frontend]
runtime = "browser-esm"
entry = "./src/frontend/index.ts"
dev_url = "http://127.0.0.1:4201/src/index.ts"
hmr = true

[backend]
runtime = "node"
entry = "./src/backend/index.ts"
healthcheck = "/health"
restart = "hot"

[capabilities]
pages = true
panels = true
themes = true
locales = true
commands = true
providers = true
hooks = true

[contributes]
pages = [{{ id = "{id}.events", title = {{ key = "ext.{id}.page.events", fallback = "Observatory" }}, route = "/{id}", mount = "{id}.events.page", icon = "activity" }}]
panels = [{{ id = "{id}.timeline", title = {{ key = "ext.{id}.panel.timeline", fallback = "Event Timeline" }}, mount = "{id}.timeline.panel", slot = "right", icon = "panel-right" }}]
themes = [{{ id = "{id}.daybreak", label = {{ key = "ext.{id}.theme.daybreak", fallback = "Daybreak" }}, appearance = "Light", tokens_entry = "frontend/themes/daybreak.css", preview_color = "#F4A261", extends = "system", category = "extension" }}]
locales = [{{ locale = "zh-CN", namespace = "ext.{id}", entry = "frontend/locales/zh-CN.json", version = "1" }}, {{ locale = "en-US", namespace = "ext.{id}", entry = "frontend/locales/en-US.json", version = "1" }}]
commands = [{{ id = "{id}.open", title = {{ key = "ext.{id}.command.open", fallback = "Open Observatory" }}, action = "open-page", shortcut = "Ctrl+Shift+O" }}]
providers = [{{ id = "{id}.feed", kind = "activity-feed", entry = "backend/providers/activity-feed.js" }}]
hooks = [{{ event = "run.completed", handler = "backend/hooks/run-completed.js" }}]
"##
        )
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ennoia-{prefix}-{}", unique_suffix()))
    }
}
