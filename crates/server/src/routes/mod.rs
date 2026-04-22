use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::header,
    http::StatusCode,
    middleware as axum_middleware,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{any, delete, get, post, put},
    Extension, Json, Router,
};
use chrono::Utc;
use ennoia_contract::ApiError;
use ennoia_extension_host::{
    read_registry_file, ExtensionRuntimeSnapshot, RegisteredCommandContribution,
    RegisteredHookContribution, RegisteredLocaleContribution, RegisteredPageContribution,
    RegisteredPanelContribution, RegisteredProviderContribution, RegisteredThemeContribution,
    ResolvedExtensionSnapshot,
};
use ennoia_kernel::{
    AgentConfig, AppConfig, BootstrapState, ExtensionDiagnostic, ExtensionRuntimeEvent,
    HookEventEnvelope, LocalizedText, ProviderConfig, RuntimeProfile, ServerConfig, SkillConfig,
    UiConfig, UiPreference, UiPreferenceRecord,
};
use ennoia_observability::RequestContext;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::app::{
    delete_config_from_dir, delete_skill_package, load_agent_configs, load_provider_configs,
    load_skill_configs, normalize_app_config, upsert_skill_package, write_config_to_dir, AppState,
};
use crate::middleware::{
    body_limit_middleware, cors_middleware, logging_middleware, rate_limit_middleware,
    request_context_middleware, timeout_middleware,
};

mod extensions;
mod logs;
mod resources;
mod runtime;

use extensions::*;
use logs::*;
use resources::*;
use runtime::*;

type ApiResult<T> = Result<Json<T>, ApiError>;

fn scoped(error: ApiError, request: &RequestContext) -> ApiError {
    error.with_request_id(&request.request_id)
}

pub fn build_router(state: AppState) -> Router {
    let bootstrap = Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route("/api/v1/bootstrap/setup", post(bootstrap_setup));

    let runtime = Router::new()
        .route(
            "/api/v1/runtime/profile",
            get(runtime_profile).put(runtime_profile_put),
        )
        .route(
            "/api/v1/runtime/preferences",
            get(runtime_preferences).put(runtime_preferences_put),
        )
        .route(
            "/api/v1/runtime/app-config",
            get(runtime_app_config).put(runtime_app_config_put),
        )
        .route(
            "/api/v1/runtime/server-config",
            get(runtime_server_config).put(runtime_server_config_put),
        );

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/ui/runtime", get(ui_runtime))
        .route("/api/v1/ui/messages", get(ui_messages))
        .route(
            "/api/v1/spaces/{space_id}/ui-preferences",
            get(space_ui_preferences).put(space_ui_preferences_put),
        )
        .route("/api/v1/extensions", get(extensions))
        .route(
            "/api/v1/extensions/{extension_id}/enabled",
            put(extension_enabled_put),
        )
        .route("/api/v1/extensions/runtime", get(extensions_runtime))
        .route("/api/v1/extensions/events", get(extension_events))
        .route(
            "/api/v1/extensions/events/stream",
            get(extension_events_stream),
        )
        .route("/api/v1/extensions/registry", get(extensions_runtime))
        .route("/api/v1/extensions/pages", get(extension_pages))
        .route("/api/v1/extensions/panels", get(extension_panels))
        .route("/api/v1/extensions/commands", get(extension_commands))
        .route("/api/v1/extensions/providers", get(extension_providers))
        .route("/api/v1/extensions/hooks", get(extension_hooks))
        .route("/api/v1/extensions/attach", post(extension_attach))
        .route("/api/v1/extensions/{extension_id}", get(extension_detail))
        .route(
            "/api/v1/extensions/{extension_id}/diagnostics",
            get(extension_diagnostics),
        )
        .route(
            "/api/v1/extensions/{extension_id}/frontend/module",
            get(extension_frontend_module),
        )
        .route(
            "/api/v1/extensions/{extension_id}/themes/{theme_id}/stylesheet",
            get(extension_theme_stylesheet),
        )
        .route(
            "/api/v1/extensions/{extension_id}/logs",
            get(extension_logs),
        )
        .route(
            "/api/v1/extensions/{extension_id}/reload",
            post(extension_reload),
        )
        .route("/api/ext/{extension_id}/{*path}", any(extension_api_proxy))
        .route(
            "/api/v1/extensions/{extension_id}/restart",
            post(extension_restart),
        )
        .route(
            "/api/v1/extensions/attach/{extension_id}",
            delete(extension_detach),
        )
        .route("/api/v1/agents", get(agents).post(agent_create))
        .route(
            "/api/v1/agents/{agent_id}",
            get(agent_detail).put(agent_update).delete(agent_delete),
        )
        .route("/api/v1/skills", get(skills).post(skill_create))
        .route(
            "/api/v1/skills/{skill_id}",
            get(skill_detail).put(skill_update).delete(skill_delete),
        )
        .route("/api/v1/providers", get(providers).post(provider_create))
        .route(
            "/api/v1/providers/{provider_id}",
            get(provider_detail)
                .put(provider_update)
                .delete(provider_delete),
        )
        .route(
            "/api/v1/providers/{provider_id}/models",
            get(provider_models),
        )
        .route("/api/v1/spaces", get(spaces))
        .route("/api/v1/logs", get(logs_list))
        .route("/api/v1/logs/frontend", post(frontend_log_create))
        .merge(bootstrap)
        .merge(runtime)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            body_limit_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timeout_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            cors_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            logging_middleware,
        ))
        .layer(axum_middleware::from_fn(request_context_middleware))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    app: &'static str,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    app_name: String,
    web_title: LocalizedText,
    default_theme: String,
    modules: Vec<String>,
    counts: JsonValue,
}

#[derive(Debug, Serialize)]
struct UiRuntimeRegistryResponse {
    pages: Vec<RegisteredPageContribution>,
    panels: Vec<RegisteredPanelContribution>,
    themes: Vec<RegisteredThemeContribution>,
    locales: Vec<RegisteredLocaleContribution>,
    providers: Vec<RegisteredProviderContribution>,
}

#[derive(Debug, Serialize)]
struct UiRuntimeVersionsResponse {
    registry: u64,
    preferences: u64,
}

#[derive(Debug, Serialize)]
struct UiRuntimeResponse {
    ui_config: UiConfig,
    registry: UiRuntimeRegistryResponse,
    instance_preference: Option<UiPreferenceRecord>,
    space_preferences: Vec<UiPreferenceRecord>,
    versions: UiRuntimeVersionsResponse,
}

#[derive(Debug, Serialize)]
struct UiMessageBundleResponse {
    locale: String,
    resolved_locale: String,
    namespace: String,
    messages: HashMap<String, String>,
    source: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct UiMessagesResponse {
    locale: String,
    fallback_locale: String,
    bundles: Vec<UiMessageBundleResponse>,
}

#[derive(Debug, Deserialize)]
struct UiMessagesQuery {
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    namespaces: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UiPreferencePayload {
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    theme_id: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    date_style: Option<String>,
    #[serde(default)]
    density: Option<String>,
    #[serde(default)]
    motion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapSetupPayload {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    default_space_id: Option<String>,
    #[serde(default)]
    theme_id: Option<String>,
    #[serde(default)]
    date_style: Option<String>,
    #[serde(default)]
    density: Option<String>,
    #[serde(default)]
    motion: Option<String>,
}

#[derive(Debug, Serialize)]
struct BootstrapSetupResponse {
    bootstrap: BootstrapState,
    profile: RuntimeProfile,
    preference: UiPreferenceRecord,
}

#[derive(Debug, Deserialize)]
struct RuntimeProfilePayload {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    default_space_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionEnabledPayload {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ExtensionWorkbenchRecord {
    id: String,
    name: String,
    enabled: bool,
    status: String,
    version: String,
    kind: String,
    source_mode: String,
    install_dir: String,
    source_root: String,
    diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FrontendLogPayload {
    level: String,
    title: String,
    summary: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionEventsQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ExtensionAttachPayload {
    path: String,
}

#[derive(Debug, Serialize)]
struct ProviderModelsResponse {
    provider_id: String,
    source: String,
    models: Vec<String>,
    recommended_model: Option<String>,
    manual_allowed: bool,
    generation_options: Vec<ennoia_kernel::ProviderGenerationOption>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "Ennoia",
    })
}

async fn overview(State(state): State<AppState>) -> Json<OverviewResponse> {
    let extension_snapshot = state.extensions.snapshot();
    let agent_count = load_agent_configs(&state.runtime_paths)
        .map(|items| items.len())
        .unwrap_or(state.agents.len());

    Json(OverviewResponse {
        app_name: state.overview.app_name,
        web_title: state.ui_config.web_title.clone(),
        default_theme: state.ui_config.default_theme.clone(),
        modules: state.overview.modules,
        counts: serde_json::json!({
            "agents": agent_count,
            "spaces": state.spaces.len(),
            "extensions": extension_snapshot.extensions.len()
        }),
    })
}

async fn ui_runtime(State(state): State<AppState>) -> Json<UiRuntimeResponse> {
    let snapshot = state.extensions.snapshot();
    let instance_preference = read_instance_ui_preference_from_disk(&state);
    let space_preferences = list_space_ui_preferences_from_disk(&state);
    let registry_version = (snapshot.pages.len()
        + snapshot.panels.len()
        + snapshot.themes.len()
        + snapshot.locales.len()
        + snapshot.providers.len()) as u64;
    let preference_version = ui_preference_version_from_disk(&state);

    Json(UiRuntimeResponse {
        ui_config: state.ui_config.clone(),
        registry: UiRuntimeRegistryResponse {
            pages: snapshot.pages,
            panels: snapshot.panels,
            themes: snapshot.themes,
            locales: snapshot.locales,
            providers: snapshot.providers,
        },
        instance_preference,
        space_preferences,
        versions: UiRuntimeVersionsResponse {
            registry: registry_version,
            preferences: preference_version,
        },
    })
}

async fn ui_messages(
    State(state): State<AppState>,
    Query(query): Query<UiMessagesQuery>,
) -> Json<UiMessagesResponse> {
    let locale = query
        .locale
        .unwrap_or_else(|| state.ui_config.default_locale.clone());
    let namespaces = query
        .namespaces
        .as_deref()
        .map(parse_namespaces)
        .filter(|items| !items.is_empty())
        .unwrap_or_else(builtin_message_namespaces);

    let snapshot = state.extensions.snapshot();
    let bundles = namespaces
        .iter()
        .filter_map(|namespace| {
            extension_message_bundle(
                &snapshot.locales,
                &locale,
                &state.ui_config.fallback_locale,
                namespace,
            )
            .or_else(|| {
                builtin_message_bundle(&locale, &state.ui_config.fallback_locale, namespace)
            })
        })
        .collect::<Vec<_>>();

    Json(UiMessagesResponse {
        locale,
        fallback_locale: state.ui_config.fallback_locale.clone(),
        bundles,
    })
}

type StaticMessages = &'static [(&'static str, &'static str)];
type StaticCatalog = &'static [(&'static str, StaticMessages)];

fn parse_namespaces(value: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter_map(|item| {
            let namespace = item.to_string();
            if seen.insert(namespace.clone()) {
                Some(namespace)
            } else {
                None
            }
        })
        .collect()
}

fn builtin_message_namespaces() -> Vec<String> {
    vec![
        "web".to_string(),
        "settings".to_string(),
        "ext.observatory".to_string(),
    ]
}

fn builtin_message_bundle(
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<UiMessageBundleResponse> {
    const WEB_ZH_CN: StaticMessages = &[("web.title", "Ennoia")];
    const WEB_EN_US: StaticMessages = &[("web.title", "Ennoia")];
    const SETTINGS_ZH_CN: StaticMessages = &[("settings.personal.saved", "偏好已保存。")];
    const SETTINGS_EN_US: StaticMessages = &[("settings.personal.saved", "Preferences saved.")];
    const OBSERVATORY_ZH_CN: StaticMessages = &[
        ("ext.observatory.page.events", "观测台"),
        ("ext.observatory.panel.timeline", "事件时间线"),
        ("ext.observatory.theme.daybreak", "Daybreak"),
        ("ext.observatory.command.open", "打开观测台"),
    ];
    const OBSERVATORY_EN_US: StaticMessages = &[
        ("ext.observatory.page.events", "Observatory"),
        ("ext.observatory.panel.timeline", "Event Timeline"),
        ("ext.observatory.theme.daybreak", "Daybreak"),
        ("ext.observatory.command.open", "Open Observatory"),
    ];

    let (source, version, catalogs): (&str, &str, StaticCatalog) = match namespace {
        "web" => (
            "builtin:web",
            "1",
            &[("zh-CN", WEB_ZH_CN), ("en-US", WEB_EN_US)],
        ),
        "settings" => (
            "builtin:settings",
            "1",
            &[("zh-CN", SETTINGS_ZH_CN), ("en-US", SETTINGS_EN_US)],
        ),
        "ext.observatory" => (
            "builtin:ext.observatory",
            "1",
            &[("zh-CN", OBSERVATORY_ZH_CN), ("en-US", OBSERVATORY_EN_US)],
        ),
        _ => return None,
    };

    let (resolved_locale, messages) = select_messages_for_locale(locale, fallback_locale, catalogs);

    Some(UiMessageBundleResponse {
        locale: locale.to_string(),
        resolved_locale: resolved_locale.to_string(),
        namespace: namespace.to_string(),
        messages: messages
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        source: source.to_string(),
        version: version.to_string(),
    })
}

fn extension_message_bundle(
    locales: &[RegisteredLocaleContribution],
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<UiMessageBundleResponse> {
    let contribution =
        select_registered_locale_contribution(locales, locale, fallback_locale, namespace)?;
    let source_root = PathBuf::from(&contribution.install_dir);
    let message_path =
        resolve_safe_extension_asset(&source_root, &contribution.locale.entry).ok()?;
    let messages = fs::read_to_string(message_path).ok()?;
    let parsed = serde_json::from_str::<HashMap<String, String>>(&messages).ok()?;

    Some(UiMessageBundleResponse {
        locale: locale.to_string(),
        resolved_locale: contribution.locale.locale.clone(),
        namespace: namespace.to_string(),
        messages: parsed,
        source: format!("extension:{}", contribution.extension_id),
        version: contribution.locale.version.clone(),
    })
}

fn select_registered_locale_contribution<'a>(
    locales: &'a [RegisteredLocaleContribution],
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<&'a RegisteredLocaleContribution> {
    let candidates = locales
        .iter()
        .filter(|item| item.locale.namespace == namespace)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    find_registered_locale_match(&candidates, locale)
        .or_else(|| find_registered_locale_match(&candidates, fallback_locale))
}

fn find_registered_locale_match<'a>(
    candidates: &[&'a RegisteredLocaleContribution],
    locale: &str,
) -> Option<&'a RegisteredLocaleContribution> {
    let normalized = locale.to_lowercase();
    let language = normalized.split('-').next().unwrap_or_default();

    candidates
        .iter()
        .copied()
        .find(|item| item.locale.locale.to_lowercase() == normalized)
        .or_else(|| {
            candidates.iter().copied().find(|item| {
                item.locale
                    .locale
                    .to_lowercase()
                    .split('-')
                    .next()
                    .unwrap_or_default()
                    == language
            })
        })
}

fn select_messages_for_locale(
    locale: &str,
    fallback_locale: &str,
    catalogs: StaticCatalog,
) -> (&'static str, StaticMessages) {
    let normalized = locale.to_lowercase();
    let language = normalized.split('-').next().unwrap_or_default();
    let fallback_normalized = fallback_locale.to_lowercase();
    let fallback_language = fallback_normalized.split('-').next().unwrap_or_default();

    catalogs
        .iter()
        .find(|(candidate, _)| candidate.to_lowercase() == normalized)
        .copied()
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| {
                    candidate
                        .to_lowercase()
                        .split('-')
                        .next()
                        .unwrap_or_default()
                        == language
                })
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| candidate.to_lowercase() == fallback_normalized)
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| {
                    candidate
                        .to_lowercase()
                        .split('-')
                        .next()
                        .unwrap_or_default()
                        == fallback_language
                })
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| candidate.eq_ignore_ascii_case("en-US"))
                .copied()
        })
        .or_else(|| catalogs.first().copied())
        .unwrap_or(("en-US", &[]))
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

const BUILTIN_THEME_IDS: &[&str] = &[
    "system",
    "ennoia.midnight",
    "ennoia.paper",
    "observatory.daybreak",
];

fn ensure_supported_locale(
    state: &AppState,
    request: &RequestContext,
    locale: String,
) -> Result<String, ApiError> {
    if state
        .ui_config
        .available_locales
        .iter()
        .any(|item| item == &locale)
    {
        return Ok(locale);
    }
    Err(scoped(
        ApiError::bad_request(format!("unsupported locale '{locale}'")),
        request,
    ))
}

fn ensure_supported_theme_id(
    state: &AppState,
    request: &RequestContext,
    theme_id: String,
) -> Result<String, ApiError> {
    if supported_theme_ids(state).contains(&theme_id) {
        return Ok(theme_id);
    }
    Err(scoped(
        ApiError::bad_request(format!("unsupported theme '{theme_id}'")),
        request,
    ))
}

fn validate_ui_preference_payload(
    state: &AppState,
    request: &RequestContext,
    payload: &UiPreferencePayload,
) -> Result<(), ApiError> {
    if let Some(locale) = &payload.locale {
        ensure_supported_locale(state, request, locale.clone())?;
    }
    if let Some(theme_id) = &payload.theme_id {
        ensure_supported_theme_id(state, request, theme_id.clone())?;
    }
    Ok(())
}

fn supported_theme_ids(state: &AppState) -> HashSet<String> {
    let mut ids = BUILTIN_THEME_IDS
        .iter()
        .map(|item| item.to_string())
        .collect::<HashSet<_>>();
    for theme in state.extensions.snapshot().themes {
        ids.insert(theme.theme.id);
    }
    ids
}

fn resolve_safe_extension_asset(root: &StdPath, entry: &str) -> std::io::Result<PathBuf> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_asset = fs::canonicalize(root.join(entry))?;
    if !canonical_asset.starts_with(&canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "extension asset must stay inside the extension root",
        ));
    }
    Ok(canonical_asset)
}

fn merge_ui_preference(
    current: Option<&UiPreference>,
    payload: UiPreferencePayload,
) -> UiPreference {
    UiPreference {
        locale: payload
            .locale
            .or_else(|| current.and_then(|item| item.locale.clone())),
        theme_id: payload
            .theme_id
            .or_else(|| current.and_then(|item| item.theme_id.clone())),
        time_zone: payload
            .time_zone
            .or_else(|| current.and_then(|item| item.time_zone.clone())),
        date_style: payload
            .date_style
            .or_else(|| current.and_then(|item| item.date_style.clone())),
        density: payload
            .density
            .or_else(|| current.and_then(|item| item.density.clone())),
        motion: payload
            .motion
            .or_else(|| current.and_then(|item| item.motion.clone())),
        version: current.map(|item| item.version + 1).unwrap_or(1),
        updated_at: now_iso(),
    }
}

fn list_extension_workbench_records(state: &AppState) -> Vec<ExtensionWorkbenchRecord> {
    let mut by_id = state
        .extensions
        .snapshot()
        .extensions
        .into_iter()
        .map(|item| (item.id.clone(), map_extension_workbench_record(&item)))
        .collect::<HashMap<_, _>>();

    if let Ok(registry) = read_registry_file(&state.runtime_paths.extensions_registry_file()) {
        for record in registry.extensions.into_iter().filter(|item| !item.removed) {
            if by_id.contains_key(&record.id) {
                continue;
            }
            by_id.insert(
                record.id.clone(),
                ExtensionWorkbenchRecord {
                    id: record.id.clone(),
                    name: record.id.clone(),
                    enabled: record.enabled,
                    status: if record.enabled {
                        "ready".to_string()
                    } else {
                        "disabled".to_string()
                    },
                    version: "unknown".to_string(),
                    kind: "extension".to_string(),
                    source_mode: record.source,
                    install_dir: record.path.clone(),
                    source_root: record.path,
                    diagnostics: Vec::new(),
                },
            );
        }
    }

    let mut items = by_id.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn map_extension_workbench_record(
    extension: &ResolvedExtensionSnapshot,
) -> ExtensionWorkbenchRecord {
    ExtensionWorkbenchRecord {
        id: extension.id.clone(),
        name: extension.name.clone(),
        enabled: !matches!(extension.health, ennoia_kernel::ExtensionHealth::Stopped),
        status: match extension.health {
            ennoia_kernel::ExtensionHealth::Ready => "ready".to_string(),
            ennoia_kernel::ExtensionHealth::Failed => "failed".to_string(),
            ennoia_kernel::ExtensionHealth::Degraded => "degraded".to_string(),
            ennoia_kernel::ExtensionHealth::Stopped => "disabled".to_string(),
            ennoia_kernel::ExtensionHealth::Discovering => "discovering".to_string(),
            ennoia_kernel::ExtensionHealth::Resolving => "resolving".to_string(),
        },
        version: extension.version.clone(),
        kind: format!("{:?}", extension.kind),
        source_mode: format!("{:?}", extension.source_mode),
        install_dir: extension.install_dir.clone(),
        source_root: extension.source_root.clone(),
        diagnostics: extension.diagnostics.clone(),
    }
}
