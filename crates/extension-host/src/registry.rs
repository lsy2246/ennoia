use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ennoia_kernel::{
    ActionRule, BehaviorContribution, ExtensionCompatSpec, ExtensionConversationSpec,
    ExtensionDevSourceEntry, ExtensionDiagnostic, ExtensionEventSpec, ExtensionHealth,
    ExtensionManifest, ExtensionOperationSpec, ExtensionRegistryEntry, ExtensionRegistryFile,
    ExtensionRpcRequest, ExtensionRpcResponse, ExtensionRuntimeEvent, ExtensionRuntimeSpec,
    ExtensionSettingFieldSpec, ExtensionSourceMode, ExtensionViewSpec, HookContribution,
    LocaleContribution, MemoryContribution, MessageRendererContribution, PageContribution,
    PageNavContribution, PanelContribution, ProviderContribution, ResolvedUiEntry,
    ResolvedWorkerEntry, ScheduleActionContribution, ThemeContribution,
};
use serde::Serialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedExtensionSnapshot {
    pub id: String,
    pub version: Option<String>,
    pub name: String,
    pub description: String,
    pub docs: Option<String>,
    pub compat: ExtensionCompatSpec,
    pub conversation: ExtensionConversationSpec,
    pub source_mode: ExtensionSourceMode,
    pub source_root: String,
    pub install_dir: String,
    pub generation: u64,
    pub health: ExtensionHealth,
    pub views: Vec<ExtensionViewSpec>,
    pub operations: Vec<ExtensionOperationSpec>,
    pub events: Vec<ExtensionEventSpec>,
    #[serde(skip_serializing)]
    pub ui: Option<ResolvedUiEntry>,
    #[serde(skip_serializing)]
    pub worker: Option<ResolvedWorkerEntry>,
    #[serde(skip_serializing)]
    pub runtime: ExtensionRuntimeSpec,
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub locales: Vec<LocaleContribution>,
    pub message_renderers: Vec<MessageRendererContribution>,
    pub settings: Vec<ExtensionSettingFieldSpec>,
    pub providers: Vec<ProviderContribution>,
    pub behaviors: Vec<BehaviorContribution>,
    pub memories: Vec<MemoryContribution>,
    pub hooks: Vec<HookContribution>,
    pub actions: Vec<ActionRule>,
    pub schedule_actions: Vec<ScheduleActionContribution>,
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredExtensionViewContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub view: ExtensionViewSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredExtensionOperationContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub operation: ExtensionOperationSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredExtensionEventContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub event: ExtensionEventSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPageContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub page: PageContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPanelContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub panel: PanelContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredThemeContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub theme: ThemeContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredLocaleContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub locale: LocaleContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredMessageRendererContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub renderer: MessageRendererContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredProviderContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub provider: ProviderContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredBehaviorContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub behavior: BehaviorContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredMemoryContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub memory: MemoryContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredHookContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub hook: HookContribution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegisteredActionRuleContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub action: ActionRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredScheduleActionContribution {
    pub extension_id: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub schedule_action: ScheduleActionContribution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtensionRuntimeSnapshot {
    pub generation: u64,
    pub updated_at: String,
    pub extensions: Vec<ResolvedExtensionSnapshot>,
    pub views: Vec<RegisteredExtensionViewContribution>,
    pub operations: Vec<RegisteredExtensionOperationContribution>,
    pub events: Vec<RegisteredExtensionEventContribution>,
    pub pages: Vec<RegisteredPageContribution>,
    pub panels: Vec<RegisteredPanelContribution>,
    pub themes: Vec<RegisteredThemeContribution>,
    pub locales: Vec<RegisteredLocaleContribution>,
    pub message_renderers: Vec<RegisteredMessageRendererContribution>,
    pub providers: Vec<RegisteredProviderContribution>,
    pub behaviors: Vec<RegisteredBehaviorContribution>,
    pub memories: Vec<RegisteredMemoryContribution>,
    pub hooks: Vec<RegisteredHookContribution>,
    pub actions: Vec<RegisteredActionRuleContribution>,
    pub schedule_actions: Vec<RegisteredScheduleActionContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRuntimeConfig {
    pub registry_file: PathBuf,
    pub logs_dir: PathBuf,
    pub home_dir: PathBuf,
    pub allow_dev_sources: bool,
    pub runtime_defaults: ExtensionRuntimeSpec,
}

#[derive(Debug, Clone)]
pub struct ExtensionRuntime {
    config: ExtensionRuntimeConfig,
    state: Arc<RwLock<ExtensionRuntimeState>>,
    worker_runtime: Arc<crate::worker::WorkerRuntime>,
}

#[derive(Debug)]
struct ExtensionRuntimeState {
    snapshot: ExtensionRuntimeSnapshot,
    events: Vec<ExtensionRuntimeEvent>,
}

#[derive(Debug, Clone)]
struct RuntimeSource {
    root: PathBuf,
    source_mode: ExtensionSourceMode,
}

impl ExtensionRuntime {
    pub fn bootstrap(config: ExtensionRuntimeConfig) -> io::Result<Self> {
        ensure_parent_dir(&config.registry_file)?;
        if !config.registry_file.exists() {
            write_registry_file(&config.registry_file, &ExtensionRegistryFile::default())?;
        }
        let home_dir = config.home_dir.clone();
        let logs_dir = config.logs_dir.clone();

        let mut state = ExtensionRuntimeState {
            snapshot: empty_snapshot(),
            events: Vec::new(),
        };
        let next = build_snapshot(&config, state.snapshot.generation + 1)?;
        state.push_replace(next, "runtime bootstrap");

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
            worker_runtime: Arc::new(crate::worker::WorkerRuntime::new(home_dir, logs_dir)?),
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

    pub fn dispatch_rpc(
        &self,
        extension_id: &str,
        method: &str,
        request: ExtensionRpcRequest,
        host_dispatcher: Option<&dyn crate::worker::HostCapabilityDispatcher>,
    ) -> io::Result<ExtensionRpcResponse> {
        let extension = self.get(extension_id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("extension {extension_id} not found"),
            )
        })?;
        self.worker_runtime
            .dispatch(&extension, method, request, host_dispatcher)
    }

    pub fn terminate_worker(&self, extension_id: &str) {
        self.worker_runtime.terminate_extension(extension_id);
    }

    pub fn hooks_for_event(&self, event: &str) -> Vec<RegisteredHookContribution> {
        let mut hooks = self
            .snapshot()
            .hooks
            .into_iter()
            .filter(|item| item.hook.event == event)
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            right
                .hook
                .priority
                .cmp(&left.hook.priority)
                .then_with(|| left.extension_id.cmp(&right.extension_id))
                .then_with(|| left.hook.handler.cmp(&right.hook.handler))
        });
        hooks
    }

    pub fn refresh_from_disk(&self, summary: &str) -> io::Result<Option<ExtensionRuntimeSnapshot>> {
        let current = self.snapshot();
        let next = build_snapshot(&self.config, current.generation + 1)?;
        if equivalent_snapshots(&current, &next) {
            return Ok(None);
        }

        // Worker invalidation may need to wait for an in-flight process worker call.
        // Keep that wait outside the runtime write lock so snapshot readers do not stall.
        self.worker_runtime
            .invalidate_missing_or_changed(&next.extensions);
        let mut state = self.state.write().expect("extension runtime write lock");
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

    pub fn attach_dev_source(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<ResolvedExtensionSnapshot> {
        let path = canonicalize_or_original(path.as_ref());
        let manifest = read_manifest_from_root(&path)?;
        let mut registry = read_registry_file(&self.config.registry_file)?;
        registry.dev_sources.retain(|item| item.id != manifest.id);
        registry.dev_sources.push(ExtensionDevSourceEntry {
            id: manifest.id.clone(),
            path: normalize_display_path(&path),
            enabled: true,
        });
        sort_dev_sources(&mut registry.dev_sources);
        write_registry_file(&self.config.registry_file, &registry)?;

        self.refresh_from_disk(&format!("dev source {} attached", manifest.id))?;
        self.get(&manifest.id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("extension {} missing after attach", manifest.id),
            )
        })
    }

    pub fn detach_dev_source(&self, extension_id: &str) -> io::Result<bool> {
        let mut registry = read_registry_file(&self.config.registry_file)?;
        let original_len = registry.dev_sources.len();
        registry.dev_sources.retain(|item| item.id != extension_id);
        if registry.dev_sources.len() == original_len {
            return Ok(false);
        }

        sort_dev_sources(&mut registry.dev_sources);
        write_registry_file(&self.config.registry_file, &registry)?;
        let _ = self.refresh_from_disk(&format!("dev source {extension_id} detached"))?;
        Ok(true)
    }

    pub fn set_extension_enabled(&self, extension_id: &str, enabled: bool) -> io::Result<bool> {
        let mut registry = read_registry_file(&self.config.registry_file)?;
        let mut updated = false;
        for entry in registry
            .extensions
            .iter_mut()
            .filter(|item| item.id == extension_id)
        {
            entry.enabled = enabled;
            updated = true;
        }
        for entry in registry
            .dev_sources
            .iter_mut()
            .filter(|item| item.id == extension_id)
        {
            entry.enabled = enabled;
            updated = true;
        }
        if !updated {
            return Ok(false);
        }
        sort_registry_entries(&mut registry.extensions);
        sort_dev_sources(&mut registry.dev_sources);
        write_registry_file(&self.config.registry_file, &registry)?;
        let summary = if enabled {
            format!("extension {extension_id} enabled")
        } else {
            format!("extension {extension_id} disabled")
        };
        let _ = self.refresh_from_disk(&summary)?;
        Ok(true)
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
    fn view_rows(&self) -> Vec<RegisteredExtensionViewContribution> {
        self.views
            .iter()
            .cloned()
            .map(|view| RegisteredExtensionViewContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                view,
            })
            .collect()
    }

    fn operation_rows(&self) -> Vec<RegisteredExtensionOperationContribution> {
        self.operations
            .iter()
            .cloned()
            .map(|operation| RegisteredExtensionOperationContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                operation,
            })
            .collect()
    }

    fn event_rows(&self) -> Vec<RegisteredExtensionEventContribution> {
        self.events
            .iter()
            .cloned()
            .map(|event| RegisteredExtensionEventContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                event,
            })
            .collect()
    }

    fn page_rows(&self) -> Vec<RegisteredPageContribution> {
        self.pages
            .iter()
            .cloned()
            .map(|page| RegisteredPageContribution {
                extension_id: self.id.clone(),
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
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                locale,
            })
            .collect()
    }

    fn message_renderer_rows(&self) -> Vec<RegisteredMessageRendererContribution> {
        self.message_renderers
            .iter()
            .cloned()
            .map(|renderer| RegisteredMessageRendererContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                renderer,
            })
            .collect()
    }

    fn provider_rows(&self) -> Vec<RegisteredProviderContribution> {
        self.providers
            .iter()
            .cloned()
            .map(|provider| RegisteredProviderContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                provider,
            })
            .collect()
    }

    fn behavior_rows(&self) -> Vec<RegisteredBehaviorContribution> {
        self.behaviors
            .iter()
            .cloned()
            .map(|behavior| RegisteredBehaviorContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                behavior,
            })
            .collect()
    }

    fn memory_rows(&self) -> Vec<RegisteredMemoryContribution> {
        self.memories
            .iter()
            .cloned()
            .map(|memory| RegisteredMemoryContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                memory,
            })
            .collect()
    }

    fn hook_rows(&self) -> Vec<RegisteredHookContribution> {
        self.hooks
            .iter()
            .cloned()
            .map(|hook| RegisteredHookContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                hook,
            })
            .collect()
    }

    fn action_rows(&self) -> Vec<RegisteredActionRuleContribution> {
        self.actions
            .iter()
            .cloned()
            .map(|action| RegisteredActionRuleContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                action,
            })
            .collect()
    }

    fn schedule_action_rows(&self) -> Vec<RegisteredScheduleActionContribution> {
        self.schedule_actions
            .iter()
            .cloned()
            .map(|schedule_action| RegisteredScheduleActionContribution {
                extension_id: self.id.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                schedule_action,
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
        let resolved = resolve_source(source, generation, config);
        match resolved_by_id.get(&resolved.id) {
            Some(existing) if source_priority(existing) >= source_priority(&resolved) => {}
            _ => {
                resolved_by_id.insert(resolved.id.clone(), resolved);
            }
        }
    }
    let mut extensions = resolved_by_id.into_values().collect::<Vec<_>>();
    extensions.sort_by(|left, right| left.id.cmp(&right.id));

    let mut views = Vec::new();
    let mut operations = Vec::new();
    let mut events = Vec::new();
    let mut pages = Vec::new();
    let mut panels = Vec::new();
    let mut themes = Vec::new();
    let mut locales = Vec::new();
    let mut message_renderers = Vec::new();
    let mut providers = Vec::new();
    let mut behaviors = Vec::new();
    let mut memories = Vec::new();
    let mut hooks = Vec::new();
    let mut actions = Vec::new();
    let mut schedule_actions = Vec::new();

    for extension in &extensions {
        views.extend(extension.view_rows());
        operations.extend(extension.operation_rows());
        events.extend(extension.event_rows());
        pages.extend(extension.page_rows());
        panels.extend(extension.panel_rows());
        themes.extend(extension.theme_rows());
        locales.extend(extension.locale_rows());
        message_renderers.extend(extension.message_renderer_rows());
        providers.extend(extension.provider_rows());
        behaviors.extend(extension.behavior_rows());
        memories.extend(extension.memory_rows());
        hooks.extend(extension.hook_rows());
        actions.extend(extension.action_rows());
        schedule_actions.extend(extension.schedule_action_rows());
    }

    Ok(ExtensionRuntimeSnapshot {
        generation,
        updated_at: now_string(),
        extensions,
        views,
        operations,
        events,
        pages,
        panels,
        themes,
        locales,
        message_renderers,
        providers,
        behaviors,
        memories,
        hooks,
        actions,
        schedule_actions,
    })
}

fn source_priority(extension: &ResolvedExtensionSnapshot) -> u8 {
    match extension.source_mode {
        ExtensionSourceMode::Dev => 2,
        ExtensionSourceMode::Package => 1,
    }
}

fn discover_sources(config: &ExtensionRuntimeConfig) -> io::Result<Vec<RuntimeSource>> {
    let mut ordered = BTreeMap::<String, RuntimeSource>::new();

    let registry = read_registry_file(&config.registry_file)?;
    let enabled_by_id = registry
        .extensions
        .into_iter()
        .map(|entry| (entry.id, entry.enabled))
        .collect::<BTreeMap<_, _>>();
    let blocked_builtin_sync = registry.blocked_builtin_sync;

    let installed_dir = config.home_dir.join("extensions");
    if installed_dir.exists() {
        for entry in fs::read_dir(installed_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let root = entry.path();
            if !root.join("extension.toml").exists() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            if blocked_builtin_sync.iter().any(|item| item == &id) {
                continue;
            }
            if !enabled_by_id.get(&id).copied().unwrap_or(true) {
                continue;
            }
            ordered.insert(
                normalize_display_path(&root),
                RuntimeSource {
                    root,
                    source_mode: ExtensionSourceMode::Package,
                },
            );
        }
    }

    if config.allow_dev_sources {
        for item in registry
            .dev_sources
            .into_iter()
            .filter(|entry| entry.enabled)
        {
            let root = PathBuf::from(&item.path);
            ordered.insert(
                normalize_display_path(&root),
                RuntimeSource {
                    root,
                    source_mode: ExtensionSourceMode::Dev,
                },
            );
        }
    }

    Ok(ordered.into_values().collect())
}

fn resolve_source(
    source: RuntimeSource,
    generation: u64,
    config: &ExtensionRuntimeConfig,
) -> ResolvedExtensionSnapshot {
    match read_manifest_from_root(&source.root) {
        Ok(manifest) => resolve_manifest(source, manifest, generation, config),
        Err(error) => failed_extension_snapshot(source, generation, error),
    }
}

fn resolve_manifest(
    source: RuntimeSource,
    manifest: ExtensionManifest,
    generation: u64,
    config: &ExtensionRuntimeConfig,
) -> ResolvedExtensionSnapshot {
    let install_dir = normalize_display_path(&source.root);
    let source_root = install_dir.clone();
    let views = manifest.views.clone();
    let operations = manifest.operations.clone();
    let events = manifest.events.clone();
    let pages = derive_pages(&views);
    let panels = derive_panels(&views);
    let settings = manifest.settings.clone();
    let message_renderers = manifest.message_renderers.clone();
    let providers = derive_providers(&operations, &manifest.id, &source.root);
    let behaviors = derive_behaviors(&operations, &manifest.id);
    let memories = derive_memories(&operations, &manifest.id);
    let actions = derive_actions(&operations);
    let schedule_actions = derive_schedule_actions(&operations);
    let hooks = derive_hooks(&events);
    let mut diagnostics = Vec::new();
    let ui = resolve_ui(&source.root, &source.source_mode, generation)
        .map_err(|error| {
            diagnostics.push(diagnostic(
                "warn",
                "ui entry discovery failed",
                Some(error.to_string()),
            ));
        })
        .ok()
        .flatten();
    let worker = resolve_worker(&source.root, &manifest.id)
        .map_err(|error| {
            diagnostics.push(diagnostic(
                "warn",
                "service entry discovery failed",
                Some(error.to_string()),
            ));
        })
        .ok()
        .flatten();

    if ui.is_none() && worker.is_none() && views.is_empty() && operations.is_empty() {
        diagnostics.push(diagnostic(
            "warn",
            "extension has no visible views or callable operations",
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

    let runtime = config.runtime_defaults.clone();

    ResolvedExtensionSnapshot {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        name: manifest.display_name(),
        description: manifest.display_description(),
        docs: manifest.docs.clone(),
        compat: manifest.compat.clone(),
        conversation: manifest.conversation.clone(),
        source_mode: source.source_mode,
        source_root,
        install_dir,
        generation,
        health,
        views,
        operations,
        events,
        ui,
        worker,
        runtime,
        pages,
        panels,
        themes: discover_themes(&source.root),
        locales: discover_locales(&source.root, &manifest.id),
        message_renderers,
        settings,
        providers,
        behaviors,
        memories,
        hooks,
        actions,
        schedule_actions,
        diagnostics,
    }
}

fn derive_pages(views: &[ExtensionViewSpec]) -> Vec<PageContribution> {
    views
        .iter()
        .filter(|view| view.view_type == "page")
        .map(|view| {
            let nav = view.nav.as_deref().and_then(|nav| match nav {
                "sidebar" | "primary" | "main" => Some(PageNavContribution {
                    default_pinned: true,
                    order: view.order,
                }),
                "none" | "" => None,
                _ => Some(PageNavContribution {
                    default_pinned: false,
                    order: view.order,
                }),
            });
            PageContribution {
                id: view.name.clone(),
                title: view.title.clone(),
                route: view
                    .route
                    .clone()
                    .unwrap_or_else(|| format!("/{}", view.name.replace('.', "/"))),
                mount: view.name.clone(),
                icon: view.icon.clone(),
                nav,
            }
        })
        .collect()
}

fn derive_panels(views: &[ExtensionViewSpec]) -> Vec<PanelContribution> {
    views
        .iter()
        .filter(|view| view.view_type == "panel")
        .map(|view| PanelContribution {
            id: view.name.clone(),
            title: view.title.clone(),
            mount: view.name.clone(),
            slot: view.slot.clone().unwrap_or_else(|| "right".to_string()),
            icon: view.icon.clone(),
        })
        .collect()
}

fn derive_providers(
    operations: &[ExtensionOperationSpec],
    extension_id: &str,
    root: &Path,
) -> Vec<ProviderContribution> {
    operations
        .iter()
        .filter_map(|operation| {
            let provider = operation.provider.as_ref()?;
            Some(ProviderContribution {
                id: operation.name.clone(),
                kind: provider.kind.clone(),
                entry: discover_provider_entry(root),
                extension_id: Some(extension_id.to_string()),
                interfaces: provider.interfaces.clone(),
                model_discovery: provider.model_discovery,
                manual_model: provider.manual_model,
                generation_options: provider.generation_options.clone(),
            })
        })
        .collect()
}

fn derive_behaviors(
    operations: &[ExtensionOperationSpec],
    extension_id: &str,
) -> Vec<BehaviorContribution> {
    operations
        .iter()
        .filter(|operation| operation.name == "workflow.default")
        .map(|operation| {
            Some(BehaviorContribution {
                id: "default".to_string(),
                extension_id: Some(extension_id.to_string()),
                interfaces: vec![
                    "runs".to_string(),
                    "tasks".to_string(),
                    "artifacts".to_string(),
                    "handoffs".to_string(),
                    "status".to_string(),
                ],
                entry: Some(operation.name.clone()),
            })
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn derive_memories(
    operations: &[ExtensionOperationSpec],
    extension_id: &str,
) -> Vec<MemoryContribution> {
    if !operations
        .iter()
        .any(|operation| operation.name.starts_with("memory."))
    {
        return Vec::new();
    }
    vec![MemoryContribution {
        id: "memory".to_string(),
        extension_id: Some(extension_id.to_string()),
        interfaces: operations
            .iter()
            .filter(|operation| operation.name.starts_with("memory."))
            .map(|operation| operation.name.clone())
            .collect(),
        entry: None,
    }]
}

fn derive_actions(operations: &[ExtensionOperationSpec]) -> Vec<ActionRule> {
    operations
        .iter()
        .filter(|operation| operation.provider.is_none())
        .map(|operation| ActionRule {
            action: operation.name.clone(),
            operation: operation.name.clone(),
            method: operation.name.clone(),
            phase: ennoia_kernel::ActionPhase::Execute,
            priority: 100,
            enabled: true,
            result_mode: ennoia_kernel::ActionResultMode::Last,
            when: JsonValue::Null,
            schema: operation.input.clone(),
        })
        .collect()
}

fn derive_schedule_actions(
    operations: &[ExtensionOperationSpec],
) -> Vec<ScheduleActionContribution> {
    operations
        .iter()
        .filter(|operation| operation.schedule)
        .map(|operation| ScheduleActionContribution {
            id: operation.name.clone(),
            method: operation.name.clone(),
            title: operation.title.clone(),
            schema: operation.input.clone(),
        })
        .collect()
}

fn derive_hooks(events: &[ExtensionEventSpec]) -> Vec<HookContribution> {
    events
        .iter()
        .map(|event| HookContribution {
            event: event.on.clone(),
            handler: Some(event.operation.clone()),
            priority: event.priority,
        })
        .collect()
}

fn discover_locales(root: &Path, extension_id: &str) -> Vec<LocaleContribution> {
    let locales_dir = root.join("ui").join("locales");
    let Ok(entries) = fs::read_dir(locales_dir) else {
        return Vec::new();
    };
    let mut locales = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|item| item.to_str()) != Some("json") {
                return None;
            }
            let locale = path.file_stem()?.to_string_lossy().to_string();
            Some(LocaleContribution {
                locale,
                namespace: format!("ext.{extension_id}"),
                entry: normalize_display_path(&path.strip_prefix(root).ok()?.to_path_buf()),
            })
        })
        .collect::<Vec<_>>();
    locales.sort_by(|left, right| left.locale.cmp(&right.locale));
    locales
}

fn resolve_ui(
    root: &Path,
    source_mode: &ExtensionSourceMode,
    _generation: u64,
) -> io::Result<Option<ResolvedUiEntry>> {
    if *source_mode == ExtensionSourceMode::Dev {
        if let Some(path) = discover_dev_ui_entry(root) {
            let version = regular_file_version(&path)?;
            return Ok(Some(ResolvedUiEntry {
                kind: "module".to_string(),
                entry: normalize_display_path(&path),
                hmr: true,
                version,
            }));
        }
    }

    let path = root.join("ui").join("dist").join("entry.js");
    if path.exists() {
        let version = regular_file_version(&path)?;
        return Ok(Some(ResolvedUiEntry {
            kind: "file".to_string(),
            entry: normalize_display_path(&path),
            hmr: false,
            version,
        }));
    }

    Ok(None)
}

fn discover_dev_ui_entry(root: &Path) -> Option<PathBuf> {
    ["ui/entry.tsx", "ui/entry.ts", "ui/entry.jsx", "ui/entry.js"]
        .into_iter()
        .map(|candidate| root.join(candidate))
        .find(|path| path.is_file())
}

fn regular_file_version(path: &Path) -> io::Result<String> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("extension ui entry must be a file: {}", path.display()),
        ));
    }
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Ok(format!("{modified}-{}", metadata.len()))
}

fn resolve_worker(root: &Path, extension_id: &str) -> io::Result<Option<ResolvedWorkerEntry>> {
    let Some(entry_path) = discover_service_entry(root, extension_id) else {
        return Ok(None);
    };
    if !entry_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "extension service entry not found: {}",
                entry_path.display()
            ),
        ));
    }

    Ok(Some(ResolvedWorkerEntry {
        kind: "process".to_string(),
        entry: normalize_display_path(&entry_path),
        abi: String::new(),
        protocol: Some("jsonrpc-stdio".to_string()),
        status: "ready".to_string(),
    }))
}

fn discover_provider_entry(root: &Path) -> Option<String> {
    ["plugins/provider/provider.js", "provider/index.js"]
        .into_iter()
        .find(|candidate| root.join(candidate).is_file())
        .map(str::to_string)
}

fn discover_service_entry(root: &Path, extension_id: &str) -> Option<PathBuf> {
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let candidates = [
        format!("runtime/service{executable_suffix}"),
        format!("bin/{extension_id}-service{executable_suffix}"),
        format!("bin/{extension_id}{executable_suffix}"),
        format!("bin/service{executable_suffix}"),
    ];
    candidates
        .into_iter()
        .map(|candidate| root.join(candidate))
        .find(|path| path.is_file())
}

fn discover_themes(_root: &Path) -> Vec<ThemeContribution> {
    Vec::new()
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
        version: None,
        name: id,
        description: String::new(),
        docs: None,
        compat: ExtensionCompatSpec::default(),
        conversation: ExtensionConversationSpec::default(),
        source_mode: source.source_mode,
        source_root: source_root.clone(),
        install_dir: source_root,
        generation,
        health: ExtensionHealth::Failed,
        views: Vec::new(),
        operations: Vec::new(),
        events: Vec::new(),
        ui: None,
        worker: None,
        runtime: ExtensionRuntimeSpec::default(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        message_renderers: Vec::new(),
        settings: Vec::new(),
        providers: Vec::new(),
        behaviors: Vec::new(),
        memories: Vec::new(),
        hooks: Vec::new(),
        actions: Vec::new(),
        schedule_actions: Vec::new(),
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
    let path = root.join("extension.toml");
    path.exists().then_some(path)
}

pub fn read_registry_file(path: &Path) -> io::Result<ExtensionRegistryFile> {
    if !path.exists() {
        return Ok(ExtensionRegistryFile::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

pub fn write_registry_file(path: &Path, file: &ExtensionRegistryFile) -> io::Result<()> {
    ensure_parent_dir(path)?;
    fs::write(
        path,
        toml::to_string_pretty(file).map_err(io::Error::other)?,
    )
}

fn sort_registry_entries(entries: &mut [ExtensionRegistryEntry]) {
    entries.sort_by(|left, right| left.id.cmp(&right.id));
}

fn sort_dev_sources(entries: &mut [ExtensionDevSourceEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn equivalent_snapshots(
    current: &ExtensionRuntimeSnapshot,
    next: &ExtensionRuntimeSnapshot,
) -> bool {
    normalize_extensions(&current.extensions) == normalize_extensions(&next.extensions)
        && current.views == next.views
        && current.operations == next.operations
        && current.events == next.events
        && current.pages == next.pages
        && current.panels == next.panels
        && current.themes == next.themes
        && current.locales == next.locales
        && current.message_renderers == next.message_renderers
        && current.providers == next.providers
        && current.behaviors == next.behaviors
        && current.memories == next.memories
        && current.hooks == next.hooks
        && current.actions == next.actions
        && current.schedule_actions == next.schedule_actions
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
        views: Vec::new(),
        operations: Vec::new(),
        events: Vec::new(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        message_renderers: Vec::new(),
        providers: Vec::new(),
        behaviors: Vec::new(),
        memories: Vec::new(),
        hooks: Vec::new(),
        actions: Vec::new(),
        schedule_actions: Vec::new(),
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
        let ext_dir = root.join("sample");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::create_dir_all(ext_dir.join("runtime")).expect("create runtime dir");
        fs::write(ext_dir.join("extension.toml"), sample_descriptor()).expect("write descriptor");
        fs::write(service_entry_path(&ext_dir), b"test").expect("write service entry");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
            allow_dev_sources: true,
            runtime_defaults: ExtensionRuntimeSpec::default(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                dev_sources: vec![ExtensionDevSourceEntry {
                    id: "sample".to_string(),
                    path: normalize_display_path(&ext_dir),
                    enabled: true,
                }],
                ..ExtensionRegistryFile::default()
            },
        )
        .expect("write registry");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.extensions.len(), 1);
        assert_eq!(snapshot.extensions[0].version.as_deref(), Some("0.1.0"));
        assert_eq!(
            snapshot.extensions[0].compat.ennoia.as_deref(),
            Some(">=0.1.0")
        );
        assert!(snapshot.extensions[0].worker.is_some());
        assert_eq!(snapshot.views.len(), 2);
        assert_eq!(snapshot.operations.len(), 2);
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.pages.len(), 1);
        assert_eq!(snapshot.panels.len(), 1);
        assert_eq!(snapshot.message_renderers.len(), 1);
        assert_eq!(
            snapshot.message_renderers[0].renderer.format.as_str(),
            "markdown"
        );
        assert_eq!(
            snapshot.message_renderers[0].renderer.mount.as_str(),
            "sample.markdown"
        );
        assert_eq!(snapshot.message_renderers[0].renderer.priority, 100);
        assert_eq!(snapshot.locales.len(), 0);
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.hooks.len(), 1);
        assert_eq!(snapshot.actions.len(), 1);
        assert_eq!(snapshot.actions[0].action.operation, "sample.run.completed");
        assert_eq!(snapshot.extensions[0].health, ExtensionHealth::Ready);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn attach_dev_source_updates_runtime_snapshot() {
        let root = unique_test_dir("runtime-attach");
        let ext_dir = root.join("foo");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::create_dir_all(ext_dir.join("runtime")).expect("create runtime dir");
        fs::write(ext_dir.join("extension.toml"), sample_descriptor_for("foo"))
            .expect("write descriptor");
        fs::write(service_entry_path(&ext_dir), b"test").expect("write service entry");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
            allow_dev_sources: true,
            runtime_defaults: ExtensionRuntimeSpec::default(),
        };
        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let attached = runtime
            .attach_dev_source(&ext_dir)
            .expect("attach dev source");
        assert_eq!(attached.id, "foo");
        assert_eq!(runtime.snapshot().extensions.len(), 1);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn hooks_for_event_returns_matching_hooks_by_priority_then_stable_tiebreakers() {
        let root = unique_test_dir("runtime-hook-priority");
        let alpha_dir = root.join("alpha");
        let beta_dir = root.join("beta");
        let gamma_dir = root.join("gamma");
        for ext_dir in [&alpha_dir, &beta_dir, &gamma_dir] {
            fs::create_dir_all(ext_dir.join("runtime")).expect("create runtime dir");
            fs::write(service_entry_path(ext_dir), b"test").expect("write service entry");
        }
        fs::write(
            alpha_dir.join("extension.toml"),
            hook_descriptor_for("alpha", "conversation.message.created", "alpha.high", 100),
        )
        .expect("write alpha descriptor");
        fs::write(
            beta_dir.join("extension.toml"),
            hook_descriptor_for("beta", "conversation.message.created", "beta.low", 10),
        )
        .expect("write beta descriptor");
        fs::write(
            gamma_dir.join("extension.toml"),
            hook_descriptor_for("gamma", "conversation.message.created", "gamma.high", 100),
        )
        .expect("write gamma descriptor");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
            allow_dev_sources: true,
            runtime_defaults: ExtensionRuntimeSpec::default(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                dev_sources: vec![
                    ExtensionDevSourceEntry {
                        id: "beta".to_string(),
                        path: normalize_display_path(&beta_dir),
                        enabled: true,
                    },
                    ExtensionDevSourceEntry {
                        id: "gamma".to_string(),
                        path: normalize_display_path(&gamma_dir),
                        enabled: true,
                    },
                    ExtensionDevSourceEntry {
                        id: "alpha".to_string(),
                        path: normalize_display_path(&alpha_dir),
                        enabled: true,
                    },
                ],
                ..ExtensionRegistryFile::default()
            },
        )
        .expect("write registry");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let hooks = runtime.hooks_for_event("conversation.message.created");

        assert_eq!(
            hooks
                .iter()
                .map(|item| (
                    item.extension_id.as_str(),
                    item.hook.handler.as_deref(),
                    item.hook.priority
                ))
                .collect::<Vec<_>>(),
            vec![
                ("alpha", Some("alpha.high"), 100),
                ("gamma", Some("gamma.high"), 100),
                ("beta", Some("beta.low"), 10),
            ]
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn builtin_workflow_manifest_registers_task_orchestration_lifecycle_events() {
        let manifest_path = workspace_root()
            .join("assets")
            .join("extensions")
            .join("workflow")
            .join("extension.toml");
        let contents = fs::read_to_string(&manifest_path).expect("read workflow manifest");
        let manifest: ExtensionManifest =
            toml::from_str(&contents).expect("parse workflow manifest");
        let registered_events = manifest
            .events
            .iter()
            .map(|event| event.on.as_str())
            .collect::<Vec<_>>();

        for expected in [
            "conversation.message.created",
            "operation.updated",
            "permission.approval.resolved",
            "run.requested",
            "run.stage.changed",
            "artifact.created",
            "job.due",
        ] {
            assert!(
                registered_events.contains(&expected),
                "workflow manifest should register {expected}"
            );
        }
    }

    #[test]
    fn attached_dev_source_prefers_discovered_ui_entry_over_bundle() {
        let root = unique_test_dir("runtime-dev-ui-entry");
        let ext_dir = root.join("sample");
        fs::create_dir_all(ext_dir.join("runtime")).expect("create runtime dir");
        fs::create_dir_all(ext_dir.join("ui/dist")).expect("create bundle dir");
        fs::write(
            ext_dir.join("extension.toml"),
            sample_descriptor_without_ui_entry("sample"),
        )
        .expect("write descriptor");
        fs::write(service_entry_path(&ext_dir), b"test").expect("write service entry");
        fs::write(ext_dir.join("ui/entry.tsx"), "export default {};").expect("write ui entry");
        fs::write(ext_dir.join("ui/dist/entry.js"), "export default {};")
            .expect("write bundled ui entry");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
            allow_dev_sources: true,
            runtime_defaults: ExtensionRuntimeSpec::default(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                dev_sources: vec![ExtensionDevSourceEntry {
                    id: "sample".to_string(),
                    path: normalize_display_path(&ext_dir),
                    enabled: true,
                }],
                ..ExtensionRegistryFile::default()
            },
        )
        .expect("write registry");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let snapshot = runtime.snapshot();
        let extension = snapshot
            .extensions
            .into_iter()
            .find(|item| item.id == "sample")
            .expect("sample extension");
        let ui = extension.ui.expect("resolved ui");
        assert_eq!(ui.kind, "module");
        assert!(ui.entry.ends_with("ui/entry.tsx"));

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn runtime_ignores_dev_sources_when_disabled() {
        let root = unique_test_dir("runtime-ignore-dev-sources");
        let ext_dir = root.join("sample");
        fs::create_dir_all(ext_dir.join("runtime")).expect("create runtime dir");
        fs::write(
            ext_dir.join("extension.toml"),
            sample_descriptor_without_ui_entry("sample"),
        )
        .expect("write descriptor");
        fs::write(service_entry_path(&ext_dir), b"test").expect("write service entry");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
            allow_dev_sources: false,
            runtime_defaults: ExtensionRuntimeSpec::default(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                dev_sources: vec![ExtensionDevSourceEntry {
                    id: "sample".to_string(),
                    path: normalize_display_path(&ext_dir),
                    enabled: true,
                }],
                ..ExtensionRegistryFile::default()
            },
        )
        .expect("write registry");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        assert!(runtime.snapshot().extensions.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    fn sample_descriptor() -> String {
        sample_descriptor_for("sample")
    }

    fn sample_descriptor_without_ui_entry(id: &str) -> String {
        format!(
            r##"
id = "{id}"
name = "Logs"
description = "Test extension"
docs = "docs/overview.md"
"##
        )
    }

    fn sample_descriptor_for(id: &str) -> String {
        format!(
            r##"
id = "{id}"
version = "0.1.0"
name = "Logs"
description = "Test extension"
docs = "docs/overview.md"

[compat]
ennoia = ">=0.1.0"

[conversation]
visible = true
resources = ["{id}.event"]
operations = ["{id}.feed"]

[[views]]
name = "{id}.events"
type = "page"
title = {{ key = "ext.{id}.page.events", fallback = "Logs" }}
route = "/{id}"
nav = "sidebar"
order = 10
icon = "activity"

[[views]]
name = "{id}.timeline"
type = "panel"
title = {{ key = "ext.{id}.panel.timeline", fallback = "Event Timeline" }}
slot = "right"
icon = "panel-right"

[[operations]]
name = "{id}.feed"
provider = {{ kind = "activity-feed" }}

[[operations]]
name = "{id}.run.completed"

[[events]]
on = "run.completed"
operation = "{id}.run.completed"

[[message_renderers]]
id = "{id}.markdown"
format = "markdown"
mount = "{id}.markdown"
priority = 100
"##
        )
    }

    fn hook_descriptor_for(id: &str, event: &str, operation: &str, priority: i32) -> String {
        format!(
            r##"
id = "{id}"
name = "{id}"

[[operations]]
name = "{operation}"

[[events]]
on = "{event}"
operation = "{operation}"
priority = {priority}
"##
        )
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ennoia-{prefix}-{}", unique_suffix()))
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf()
    }

    fn service_entry_path(root: &Path) -> PathBuf {
        root.join("runtime").join(if cfg!(windows) {
            "service.exe"
        } else {
            "service"
        })
    }
}
