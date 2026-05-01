use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ennoia_kernel::{
    ActionRule, BehaviorContribution, CapabilityContribution, CommandContribution,
    ExtensionCapabilities, ExtensionConversationSpec, ExtensionDiagnostic, ExtensionHealth,
    ExtensionKind, ExtensionManifest, ExtensionPermissionSpec, ExtensionRegistryEntry,
    ExtensionRegistryFile, ExtensionRpcRequest, ExtensionRpcResponse, ExtensionRuntimeEvent,
    ExtensionRuntimeSpec, ExtensionSourceMode, ExtensionUiSpec, HookContribution,
    LocaleContribution, MemoryContribution, PageContribution, PanelContribution,
    ProviderContribution, ResolvedUiEntry, ResolvedWorkerEntry, ResourceTypeContribution,
    ScheduleActionContribution, SubscriptionContribution, SurfaceContribution, ThemeContribution,
};
use serde::Serialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedExtensionSnapshot {
    pub id: String,
    pub name: String,
    pub description: String,
    pub docs: Option<String>,
    pub conversation: ExtensionConversationSpec,
    pub kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub source_root: String,
    pub install_dir: String,
    pub generation: u64,
    pub health: ExtensionHealth,
    pub ui: Option<ResolvedUiEntry>,
    pub worker: Option<ResolvedWorkerEntry>,
    pub permissions: ExtensionPermissionSpec,
    pub runtime: ExtensionRuntimeSpec,
    pub capabilities: ExtensionCapabilities,
    pub resource_types: Vec<ResourceTypeContribution>,
    pub capability_rows: Vec<CapabilityContribution>,
    pub surfaces: Vec<SurfaceContribution>,
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub locales: Vec<LocaleContribution>,
    pub commands: Vec<CommandContribution>,
    pub providers: Vec<ProviderContribution>,
    pub behaviors: Vec<BehaviorContribution>,
    pub memories: Vec<MemoryContribution>,
    pub hooks: Vec<HookContribution>,
    pub actions: Vec<ActionRule>,
    pub schedule_actions: Vec<ScheduleActionContribution>,
    pub subscriptions: Vec<SubscriptionContribution>,
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredResourceTypeContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub resource_type: ResourceTypeContribution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegisteredCapabilityContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub capability: CapabilityContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredSurfaceContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub surface: SurfaceContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredSubscriptionContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub subscription: SubscriptionContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPageContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub page: PageContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPanelContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub panel: PanelContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredThemeContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub theme: ThemeContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredLocaleContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub locale: LocaleContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredCommandContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub command: CommandContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredProviderContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub provider: ProviderContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredBehaviorContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub behavior: BehaviorContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredMemoryContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub memory: MemoryContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredHookContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub hook: HookContribution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegisteredActionRuleContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub action: ActionRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredScheduleActionContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub schedule_action: ScheduleActionContribution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtensionRuntimeSnapshot {
    pub generation: u64,
    pub updated_at: String,
    pub extensions: Vec<ResolvedExtensionSnapshot>,
    pub resource_types: Vec<RegisteredResourceTypeContribution>,
    pub capabilities: Vec<RegisteredCapabilityContribution>,
    pub surfaces: Vec<RegisteredSurfaceContribution>,
    pub subscriptions: Vec<RegisteredSubscriptionContribution>,
    pub pages: Vec<RegisteredPageContribution>,
    pub panels: Vec<RegisteredPanelContribution>,
    pub themes: Vec<RegisteredThemeContribution>,
    pub locales: Vec<RegisteredLocaleContribution>,
    pub commands: Vec<RegisteredCommandContribution>,
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
    ) -> io::Result<ExtensionRpcResponse> {
        let extension = self.get(extension_id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("extension {extension_id} not found"),
            )
        })?;
        self.worker_runtime.dispatch(&extension, method, request)
    }

    pub fn hooks_for_event(&self, event: &str) -> Vec<RegisteredHookContribution> {
        self.snapshot()
            .hooks
            .into_iter()
            .filter(|item| item.hook.event == event)
            .collect()
    }

    pub fn refresh_from_disk(&self, summary: &str) -> io::Result<Option<ExtensionRuntimeSnapshot>> {
        let current = self.snapshot();
        let next = build_snapshot(&self.config, current.generation + 1)?;
        let mut state = self.state.write().expect("extension runtime write lock");
        if equivalent_snapshots(&current, &next) {
            return Ok(None);
        }

        self.worker_runtime
            .invalidate_missing_or_changed(&next.extensions);
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
        registry
            .extensions
            .retain(|item| !(item.id == manifest.id && item.source == "dev"));
        registry.extensions.push(ExtensionRegistryEntry {
            id: manifest.id.clone(),
            source: "dev".to_string(),
            enabled: true,
            removed: false,
            path: normalize_display_path(&path),
        });
        sort_registry_entries(&mut registry.extensions);
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
        let original_len = registry.extensions.len();
        registry
            .extensions
            .retain(|item| !(item.id == extension_id && item.source == "dev"));
        if registry.extensions.len() == original_len {
            return Ok(false);
        }

        sort_registry_entries(&mut registry.extensions);
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
            .filter(|item| item.id == extension_id && !item.removed)
        {
            entry.enabled = enabled;
            updated = true;
        }
        if !updated {
            return Ok(false);
        }
        sort_registry_entries(&mut registry.extensions);
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
    fn resource_type_rows(&self) -> Vec<RegisteredResourceTypeContribution> {
        self.resource_types
            .iter()
            .cloned()
            .map(|resource_type| RegisteredResourceTypeContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                resource_type,
            })
            .collect()
    }

    fn capability_rows(&self) -> Vec<RegisteredCapabilityContribution> {
        self.capability_rows
            .iter()
            .cloned()
            .map(|capability| RegisteredCapabilityContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                capability,
            })
            .collect()
    }

    fn surface_rows(&self) -> Vec<RegisteredSurfaceContribution> {
        self.surfaces
            .iter()
            .cloned()
            .map(|surface| RegisteredSurfaceContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                surface,
            })
            .collect()
    }

    fn subscription_rows(&self) -> Vec<RegisteredSubscriptionContribution> {
        self.subscriptions
            .iter()
            .cloned()
            .map(|subscription| RegisteredSubscriptionContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                subscription,
            })
            .collect()
    }

    fn page_rows(&self) -> Vec<RegisteredPageContribution> {
        self.pages
            .iter()
            .cloned()
            .map(|page| RegisteredPageContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
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
                extension_kind: self.kind.clone(),
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
                extension_kind: self.kind.clone(),
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
                extension_kind: self.kind.clone(),
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
                extension_kind: self.kind.clone(),
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
                extension_kind: self.kind.clone(),
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

    let mut resource_types = Vec::new();
    let mut capabilities = Vec::new();
    let mut surfaces = Vec::new();
    let mut subscriptions = Vec::new();
    let mut pages = Vec::new();
    let mut panels = Vec::new();
    let mut themes = Vec::new();
    let mut locales = Vec::new();
    let mut commands = Vec::new();
    let mut providers = Vec::new();
    let mut behaviors = Vec::new();
    let mut memories = Vec::new();
    let mut hooks = Vec::new();
    let mut actions = Vec::new();
    let mut schedule_actions = Vec::new();

    for extension in &extensions {
        resource_types.extend(extension.resource_type_rows());
        capabilities.extend(extension.capability_rows());
        surfaces.extend(extension.surface_rows());
        subscriptions.extend(extension.subscription_rows());
        pages.extend(extension.page_rows());
        panels.extend(extension.panel_rows());
        themes.extend(extension.theme_rows());
        locales.extend(extension.locale_rows());
        commands.extend(extension.command_rows());
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
        resource_types,
        capabilities,
        surfaces,
        subscriptions,
        pages,
        panels,
        themes,
        locales,
        commands,
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
    for item in registry
        .extensions
        .into_iter()
        .filter(|entry| entry.enabled && !entry.removed)
    {
        let root = PathBuf::from(&item.path);
        ordered.insert(
            normalize_display_path(&root),
            RuntimeSource {
                root,
                source_mode: registry_source_mode(&item),
            },
        );
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
    let resource_types = manifest.resource_types.clone();
    let capability_rows = manifest.capabilities.clone();
    let surfaces = manifest.surfaces.clone();
    let pages = derive_pages(&surfaces);
    let panels = derive_panels(&surfaces);
    let providers = derive_providers(&capability_rows, &manifest.id);
    let behaviors = derive_behaviors(&capability_rows, &manifest.id);
    let memories = derive_memories(&capability_rows, &manifest.id);
    let actions = derive_actions(&capability_rows);
    let schedule_actions = derive_schedule_actions(&capability_rows);
    let subscriptions = manifest.subscriptions.clone();
    let hooks = derive_hooks(&capability_rows, &subscriptions);
    let mut diagnostics = Vec::new();
    let ui = resolve_ui(
        &source.root,
        &source.source_mode,
        &manifest.ui,
        &manifest,
        generation,
    )
    .map_err(|error| {
        diagnostics.push(diagnostic(
            "warn",
            "ui resolution failed",
            Some(error.to_string()),
        ));
    })
    .ok()
    .flatten();
    let worker = resolve_worker(&source.root, &manifest)
        .map_err(|error| {
            diagnostics.push(diagnostic(
                "warn",
                "worker resolution failed",
                Some(error.to_string()),
            ));
        })
        .ok()
        .flatten();

    if ui.is_none() && worker.is_none() && capabilities == ExtensionCapabilities::default() {
        diagnostics.push(diagnostic(
            "warn",
            "extension has no resolved ui or worker entry",
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
        description: manifest.display_description(),
        docs: manifest.docs,
        conversation: manifest.conversation,
        kind: manifest.kind,
        source_mode: source.source_mode,
        source_root,
        install_dir,
        generation,
        health,
        ui,
        worker,
        permissions: manifest.permissions,
        runtime: manifest.runtime,
        capabilities,
        resource_types,
        capability_rows,
        surfaces,
        pages,
        panels,
        themes: manifest.themes,
        locales: manifest.locales,
        commands: manifest.commands,
        providers,
        behaviors,
        memories,
        hooks,
        actions,
        schedule_actions,
        subscriptions,
        diagnostics,
    }
}

fn derive_pages(surfaces: &[SurfaceContribution]) -> Vec<PageContribution> {
    surfaces
        .iter()
        .filter(|surface| surface.kind == "page")
        .filter_map(|surface| {
            Some(PageContribution {
                id: surface.id.clone(),
                title: surface.title.clone()?,
                route: surface.route.clone()?,
                mount: surface.mount.clone(),
                icon: surface.icon.clone(),
                nav: surface.nav.clone(),
            })
        })
        .collect()
}

fn derive_panels(surfaces: &[SurfaceContribution]) -> Vec<PanelContribution> {
    surfaces
        .iter()
        .filter(|surface| surface.kind == "panel")
        .filter_map(|surface| {
            Some(PanelContribution {
                id: surface.id.clone(),
                title: surface.title.clone()?,
                mount: surface.mount.clone(),
                slot: surface.slot.clone()?,
                icon: surface.icon.clone(),
            })
        })
        .collect()
}

fn derive_providers(
    capabilities: &[CapabilityContribution],
    extension_id: &str,
) -> Vec<ProviderContribution> {
    capabilities
        .iter()
        .filter_map(|capability| {
            let provider = capability.metadata.get("provider")?;
            Some(ProviderContribution {
                id: json_string(provider, "id").unwrap_or_else(|| capability.id.clone()),
                kind: json_string(provider, "kind").unwrap_or_else(|| capability.contract.clone()),
                entry: capability.entry.clone(),
                extension_id: Some(
                    json_string(provider, "extension_id")
                        .unwrap_or_else(|| extension_id.to_string()),
                ),
                interfaces: json_string_array(provider, "interfaces"),
                model_discovery: json_bool(provider, "model_discovery"),
                manual_model: json_bool_default(provider, "manual_model", true),
                generation_options: provider_generation_options(provider),
            })
        })
        .collect()
}

fn derive_behaviors(
    capabilities: &[CapabilityContribution],
    extension_id: &str,
) -> Vec<BehaviorContribution> {
    capabilities
        .iter()
        .filter_map(|capability| {
            let behavior = capability.metadata.get("behavior")?;
            Some(BehaviorContribution {
                id: json_string(behavior, "id").unwrap_or_else(|| capability.id.clone()),
                extension_id: Some(
                    json_string(behavior, "extension_id")
                        .unwrap_or_else(|| extension_id.to_string()),
                ),
                interfaces: json_string_array(behavior, "interfaces"),
                entry: capability.entry.clone(),
            })
        })
        .collect()
}

fn derive_memories(
    capabilities: &[CapabilityContribution],
    extension_id: &str,
) -> Vec<MemoryContribution> {
    capabilities
        .iter()
        .filter_map(|capability| {
            let memory = capability.metadata.get("memory")?;
            Some(MemoryContribution {
                id: json_string(memory, "id").unwrap_or_else(|| capability.id.clone()),
                extension_id: Some(
                    json_string(memory, "extension_id").unwrap_or_else(|| extension_id.to_string()),
                ),
                interfaces: json_string_array(memory, "interfaces"),
                entry: capability.entry.clone(),
            })
        })
        .collect()
}

fn derive_actions(capabilities: &[CapabilityContribution]) -> Vec<ActionRule> {
    capabilities
        .iter()
        .filter_map(|capability| {
            let action = capability.metadata.get("action")?;
            let action_key =
                json_string(action, "key").unwrap_or_else(|| capability.contract.clone());
            let method = capability.entry.clone()?;
            Some(ActionRule {
                action: action_key,
                capability_id: capability.id.clone(),
                method,
                phase: serde_json::from_value(
                    action
                        .get("phase")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!("execute")),
                )
                .unwrap_or_default(),
                priority: json_i32(action, "priority").unwrap_or(100),
                enabled: json_bool_default(action, "enabled", true),
                result_mode: serde_json::from_value(
                    action
                        .get("result_mode")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!("last")),
                )
                .unwrap_or_default(),
                when: action.get("when").cloned().unwrap_or(JsonValue::Null),
                schema: json_string(action, "schema"),
            })
        })
        .collect()
}

fn derive_schedule_actions(
    capabilities: &[CapabilityContribution],
) -> Vec<ScheduleActionContribution> {
    capabilities
        .iter()
        .filter_map(|capability| {
            let schedule_action = capability.metadata.get("schedule_action")?;
            Some(ScheduleActionContribution {
                id: json_string(schedule_action, "id").unwrap_or_else(|| capability.id.clone()),
                method: capability.entry.clone()?,
                title: capability.title.clone(),
                schema: json_string(schedule_action, "schema"),
            })
        })
        .collect()
}

fn derive_hooks(
    capabilities: &[CapabilityContribution],
    subscriptions: &[SubscriptionContribution],
) -> Vec<HookContribution> {
    subscriptions
        .iter()
        .filter_map(|subscription| {
            let capability = capabilities
                .iter()
                .find(|item| item.id == subscription.capability)?;
            Some(HookContribution {
                event: subscription.event.clone(),
                handler: capability.entry.clone(),
            })
        })
        .collect()
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}

fn json_bool(value: &serde_json::Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn json_bool_default(value: &serde_json::Value, key: &str, default_value: bool) -> bool {
    value
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default_value)
}

fn json_i32(value: &serde_json::Value, key: &str) -> Option<i32> {
    value
        .get(key)?
        .as_i64()
        .and_then(|item| i32::try_from(item).ok())
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn provider_generation_options(
    value: &serde_json::Value,
) -> Vec<ennoia_kernel::ProviderGenerationOption> {
    value
        .get("generation_options")
        .and_then(|item| serde_json::from_value(item.clone()).ok())
        .unwrap_or_default()
}

fn resolve_ui(
    root: &Path,
    source_mode: &ExtensionSourceMode,
    ui: &ExtensionUiSpec,
    manifest: &ExtensionManifest,
    generation: u64,
) -> io::Result<Option<ResolvedUiEntry>> {
    if *source_mode == ExtensionSourceMode::Dev {
        if let Some(dev_url) = ui.dev_url.clone() {
            return Ok(Some(ResolvedUiEntry {
                kind: "url".to_string(),
                entry: dev_url,
                hmr: ui.hmr,
                version: generation.to_string(),
            }));
        }
        if let Some(entry) = ui.entry.clone() {
            let path = root.join(entry);
            let version = regular_file_version(&path)?;
            return Ok(Some(ResolvedUiEntry {
                kind: "module".to_string(),
                entry: normalize_display_path(&path),
                hmr: ui.hmr,
                version,
            }));
        }
    }

    if let Some(bundle) = manifest.build.ui_bundle.clone() {
        let path = root.join(bundle);
        let version = regular_file_version(&path)?;
        return Ok(Some(ResolvedUiEntry {
            kind: "file".to_string(),
            entry: normalize_display_path(&path),
            hmr: ui.hmr,
            version,
        }));
    }

    Ok(None)
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

fn resolve_worker(
    root: &Path,
    manifest: &ExtensionManifest,
) -> io::Result<Option<ResolvedWorkerEntry>> {
    let Some(entry) = manifest
        .worker
        .entry
        .clone()
        .or_else(|| manifest.build.worker_bundle.clone())
    else {
        return Ok(None);
    };

    let kind = manifest
        .worker
        .kind
        .clone()
        .unwrap_or_else(|| "wasm".to_string());
    let protocol = manifest.worker.protocol.clone();
    match kind.as_str() {
        "wasm" => {
            if protocol.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "wasm worker must not declare a protocol",
                ));
            }
        }
        "process" => {
            let protocol = protocol.as_deref().unwrap_or("jsonrpc-stdio");
            if protocol != "jsonrpc-stdio" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsupported process worker protocol '{protocol}'"),
                ));
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported worker kind '{kind}'"),
            ));
        }
    }

    let entry_path = resolve_worker_entry_path(root, &entry, &kind)?;
    if !entry_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("worker entry not found: {}", entry_path.display()),
        ));
    }

    Ok(Some(ResolvedWorkerEntry {
        kind,
        entry: normalize_display_path(&entry_path),
        abi: manifest.worker.abi.clone().unwrap_or_else(|| {
            if manifest.worker.kind.as_deref() == Some("process") {
                String::new()
            } else {
                "ennoia.worker".to_string()
            }
        }),
        protocol: protocol
            .or_else(|| Some("jsonrpc-stdio".to_string()))
            .filter(|_| manifest.worker.kind.as_deref() == Some("process")),
        status: "ready".to_string(),
    }))
}

fn resolve_worker_entry_path(root: &Path, entry: &str, kind: &str) -> io::Result<PathBuf> {
    let direct = root.join(entry);
    if direct.exists() {
        return Ok(direct);
    }

    if kind == "process" && cfg!(windows) && Path::new(entry).extension().is_none() {
        let fallback = root.join(format!("{entry}.exe"));
        if fallback.exists() {
            return Ok(fallback);
        }
    }

    Ok(direct)
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
        description: String::new(),
        docs: None,
        conversation: ExtensionConversationSpec::default(),
        kind: ExtensionKind::SystemExtension,
        source_mode: source.source_mode,
        source_root: source_root.clone(),
        install_dir: source_root,
        generation,
        health: ExtensionHealth::Failed,
        ui: None,
        worker: None,
        permissions: ExtensionPermissionSpec::default(),
        runtime: ExtensionRuntimeSpec::default(),
        capabilities: ExtensionCapabilities::default(),
        resource_types: Vec::new(),
        capability_rows: Vec::new(),
        surfaces: Vec::new(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        commands: Vec::new(),
        providers: Vec::new(),
        behaviors: Vec::new(),
        memories: Vec::new(),
        hooks: Vec::new(),
        actions: Vec::new(),
        schedule_actions: Vec::new(),
        subscriptions: Vec::new(),
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

fn registry_source_mode(entry: &ExtensionRegistryEntry) -> ExtensionSourceMode {
    match entry.source.as_str() {
        "dev" => ExtensionSourceMode::Dev,
        _ => ExtensionSourceMode::Package,
    }
}

fn sort_registry_entries(entries: &mut [ExtensionRegistryEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn equivalent_snapshots(
    current: &ExtensionRuntimeSnapshot,
    next: &ExtensionRuntimeSnapshot,
) -> bool {
    normalize_extensions(&current.extensions) == normalize_extensions(&next.extensions)
        && current.resource_types == next.resource_types
        && current.capabilities == next.capabilities
        && current.surfaces == next.surfaces
        && current.subscriptions == next.subscriptions
        && current.pages == next.pages
        && current.panels == next.panels
        && current.themes == next.themes
        && current.locales == next.locales
        && current.commands == next.commands
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
        resource_types: Vec::new(),
        capabilities: Vec::new(),
        surfaces: Vec::new(),
        subscriptions: Vec::new(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        commands: Vec::new(),
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
        fs::create_dir_all(ext_dir.join("worker")).expect("create worker dir");
        fs::write(ext_dir.join("extension.toml"), sample_descriptor()).expect("write descriptor");
        fs::write(ext_dir.join("worker/plugin.wasm"), b"test").expect("write worker");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                extensions: vec![ExtensionRegistryEntry {
                    id: "sample".to_string(),
                    source: "dev".to_string(),
                    enabled: true,
                    removed: false,
                    path: normalize_display_path(&ext_dir),
                }],
            },
        )
        .expect("write registry");

        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.extensions.len(), 1);
        assert_eq!(snapshot.resource_types.len(), 1);
        assert_eq!(snapshot.capabilities.len(), 2);
        assert_eq!(snapshot.surfaces.len(), 2);
        assert_eq!(snapshot.subscriptions.len(), 1);
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
    fn attach_dev_source_updates_runtime_snapshot() {
        let root = unique_test_dir("runtime-attach");
        let ext_dir = root.join("foo");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::create_dir_all(ext_dir.join("worker")).expect("create worker dir");
        fs::write(ext_dir.join("extension.toml"), sample_descriptor_for("foo"))
            .expect("write descriptor");
        fs::write(ext_dir.join("worker/plugin.wasm"), b"test").expect("write worker");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
        };
        let runtime = ExtensionRuntime::bootstrap(config).expect("bootstrap runtime");
        let attached = runtime
            .attach_dev_source(&ext_dir)
            .expect("attach dev source");
        assert_eq!(attached.id, "foo");
        assert_eq!(runtime.snapshot().extensions.len(), 1);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    fn sample_descriptor() -> String {
        sample_descriptor_for("sample")
    }

    fn sample_descriptor_for(id: &str) -> String {
        format!(
            r##"
id = "{id}"
name = "Observatory"
kind = "extension"
description = "Test extension"
docs = "docs/overview.md"

[source]
mode = "dev"
root = "."
dev = true

[conversation]
inject = true
resource_types = ["{id}.event"]
capabilities = ["{id}.feed"]

[ui]
runtime = "browser-esm"
entry = "./ui/index.ts"
dev_url = "http://127.0.0.1:4201/src/index.ts"
hmr = true

[worker]
kind = "wasm"
entry = "./worker/plugin.wasm"
abi = "ennoia.worker"

[[resource_types]]
id = "{id}.event"
title = {{ key = "ext.{id}.resource.event", fallback = "Event" }}
content_kind = "json"
operations = ["read"]
tags = ["activity"]

[[surfaces]]
id = "{id}.events"
kind = "page"
mount = "{id}.events.page"
title = {{ key = "ext.{id}.page.events", fallback = "Observatory" }}
route = "/{id}"
icon = "activity"

[[surfaces]]
id = "{id}.timeline"
kind = "panel"
mount = "{id}.timeline.panel"
title = {{ key = "ext.{id}.panel.timeline", fallback = "Event Timeline" }}
slot = "right"
icon = "panel-right"

[[themes]]
id = "{id}.daybreak"
label = {{ key = "ext.{id}.theme.daybreak", fallback = "Daybreak" }}
appearance = "Light"
tokens_entry = "ui/themes/daybreak.css"
preview_color = "#F4A261"
extends = "system"
category = "extension"

[[locales]]
locale = "zh-CN"
namespace = "ext.{id}"
entry = "ui/locales/zh-CN.json"

[[locales]]
locale = "en-US"
namespace = "ext.{id}"
entry = "ui/locales/en-US.json"

[[commands]]
id = "{id}.open"
title = {{ key = "ext.{id}.command.open", fallback = "Open Observatory" }}
action = "open-page"
shortcut = "Ctrl+Shift+O"

[[capabilities]]
id = "{id}.feed"
contract = "activity-feed"
kind = "query"
entry = "worker/providers/activity-feed.js"
metadata = {{ provider = {{ id = "{id}.feed", kind = "activity-feed" }} }}

[[capabilities]]
id = "{id}.run.completed"
contract = "hook.run.completed"
kind = "event_handler"
entry = "worker/hooks/run-completed.js"

[[subscriptions]]
event = "run.completed"
capability = "{id}.run.completed"
"##
        )
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ennoia-{prefix}-{}", unique_suffix()))
    }
}
