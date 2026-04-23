use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ennoia_kernel::{
    BehaviorContribution, CommandContribution, ExtensionCapabilities, ExtensionDiagnostic,
    ExtensionHealth, ExtensionKind, ExtensionManifest, ExtensionPermissionSpec,
    ExtensionRegistryEntry, ExtensionRegistryFile, ExtensionRpcRequest, ExtensionRpcResponse,
    ExtensionRuntimeEvent, ExtensionRuntimeSpec, ExtensionSourceMode, ExtensionUiSpec,
    HookContribution, InterfaceContribution, LocaleContribution, MemoryContribution,
    PageContribution, PanelContribution, ProviderContribution, ResolvedUiEntry,
    ResolvedWorkerEntry, ScheduleActionContribution, ThemeContribution,
};
use serde::Serialize;

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
    pub ui: Option<ResolvedUiEntry>,
    pub worker: Option<ResolvedWorkerEntry>,
    pub permissions: ExtensionPermissionSpec,
    pub runtime: ExtensionRuntimeSpec,
    pub capabilities: ExtensionCapabilities,
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub locales: Vec<LocaleContribution>,
    pub commands: Vec<CommandContribution>,
    pub providers: Vec<ProviderContribution>,
    pub behaviors: Vec<BehaviorContribution>,
    pub memories: Vec<MemoryContribution>,
    pub hooks: Vec<HookContribution>,
    pub interfaces: Vec<InterfaceContribution>,
    pub schedule_actions: Vec<ScheduleActionContribution>,
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
pub struct RegisteredBehaviorContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub behavior: BehaviorContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredMemoryContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub memory: MemoryContribution,
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
pub struct RegisteredInterfaceContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub interface: InterfaceContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredScheduleActionContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub source_mode: ExtensionSourceMode,
    pub install_dir: String,
    pub schedule_action: ScheduleActionContribution,
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
    pub behaviors: Vec<RegisteredBehaviorContribution>,
    pub memories: Vec<RegisteredMemoryContribution>,
    pub hooks: Vec<RegisteredHookContribution>,
    pub interfaces: Vec<RegisteredInterfaceContribution>,
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

        let mut state = ExtensionRuntimeState {
            snapshot: empty_snapshot(),
            events: Vec::new(),
        };
        let next = build_snapshot(&config, state.snapshot.generation + 1)?;
        state.push_replace(next, "runtime bootstrap");

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
            worker_runtime: Arc::new(crate::worker::WorkerRuntime::new()?),
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

    fn behavior_rows(&self) -> Vec<RegisteredBehaviorContribution> {
        self.behaviors
            .iter()
            .cloned()
            .map(|behavior| RegisteredBehaviorContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
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
                extension_version: self.version.clone(),
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
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                hook,
            })
            .collect()
    }

    fn interface_rows(&self) -> Vec<RegisteredInterfaceContribution> {
        self.interfaces
            .iter()
            .cloned()
            .map(|interface| RegisteredInterfaceContribution {
                extension_id: self.id.clone(),
                extension_kind: self.kind.clone(),
                extension_version: self.version.clone(),
                source_mode: self.source_mode.clone(),
                install_dir: self.install_dir.clone(),
                interface,
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
                extension_version: self.version.clone(),
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

    let mut pages = Vec::new();
    let mut panels = Vec::new();
    let mut themes = Vec::new();
    let mut locales = Vec::new();
    let mut commands = Vec::new();
    let mut providers = Vec::new();
    let mut behaviors = Vec::new();
    let mut memories = Vec::new();
    let mut hooks = Vec::new();
    let mut interfaces = Vec::new();
    let mut schedule_actions = Vec::new();

    for extension in &extensions {
        pages.extend(extension.page_rows());
        panels.extend(extension.panel_rows());
        themes.extend(extension.theme_rows());
        locales.extend(extension.locale_rows());
        commands.extend(extension.command_rows());
        providers.extend(extension.provider_rows());
        behaviors.extend(extension.behavior_rows());
        memories.extend(extension.memory_rows());
        hooks.extend(extension.hook_rows());
        interfaces.extend(extension.interface_rows());
        schedule_actions.extend(extension.schedule_action_rows());
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
        behaviors,
        memories,
        hooks,
        interfaces,
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
    let mut diagnostics = Vec::new();
    let ui = resolve_ui(&source.root, &source.source_mode, &manifest.ui, &manifest)
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
        kind: manifest.kind,
        version: manifest.version,
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
        pages: manifest.contributes.pages,
        panels: manifest.contributes.panels,
        themes: manifest.contributes.themes,
        locales: manifest.contributes.locales,
        commands: manifest.contributes.commands,
        providers: manifest.contributes.providers,
        behaviors: manifest.contributes.behaviors,
        memories: manifest.contributes.memories,
        hooks: manifest.contributes.hooks,
        interfaces: manifest.contributes.interfaces,
        schedule_actions: manifest.contributes.schedule_actions,
        diagnostics,
    }
}

fn resolve_ui(
    root: &Path,
    source_mode: &ExtensionSourceMode,
    ui: &ExtensionUiSpec,
    manifest: &ExtensionManifest,
) -> io::Result<Option<ResolvedUiEntry>> {
    if *source_mode == ExtensionSourceMode::Dev {
        if let Some(dev_url) = ui.dev_url.clone() {
            return Ok(Some(ResolvedUiEntry {
                kind: "url".to_string(),
                entry: dev_url,
                hmr: ui.hmr,
            }));
        }
        if let Some(entry) = ui.entry.clone() {
            return Ok(Some(ResolvedUiEntry {
                kind: "module".to_string(),
                entry: normalize_display_path(&root.join(entry)),
                hmr: ui.hmr,
            }));
        }
    }

    if let Some(bundle) = manifest
        .build
        .ui_bundle
        .clone()
        .or_else(|| manifest.ui_bundle.clone())
    {
        return Ok(Some(ResolvedUiEntry {
            kind: "file".to_string(),
            entry: normalize_display_path(&root.join(bundle)),
            hmr: ui.hmr,
        }));
    }

    Ok(None)
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
        .or_else(|| manifest.worker_entry.clone())
    else {
        return Ok(None);
    };

    let kind = manifest
        .worker
        .kind
        .clone()
        .unwrap_or_else(|| "wasm".to_string());
    if kind != "wasm" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported worker kind '{kind}'"),
        ));
    }

    Ok(Some(ResolvedWorkerEntry {
        kind,
        entry: normalize_display_path(&root.join(entry)),
        abi: manifest
            .worker
            .abi
            .clone()
            .unwrap_or_else(|| "ennoia.worker.v1".to_string()),
        status: "ready".to_string(),
    }))
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
        ui: None,
        worker: None,
        permissions: ExtensionPermissionSpec::default(),
        runtime: ExtensionRuntimeSpec::default(),
        capabilities: ExtensionCapabilities::default(),
        pages: Vec::new(),
        panels: Vec::new(),
        themes: Vec::new(),
        locales: Vec::new(),
        commands: Vec::new(),
        providers: Vec::new(),
        behaviors: Vec::new(),
        memories: Vec::new(),
        hooks: Vec::new(),
        interfaces: Vec::new(),
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
        && current.pages == next.pages
        && current.panels == next.panels
        && current.themes == next.themes
        && current.locales == next.locales
        && current.commands == next.commands
        && current.providers == next.providers
        && current.behaviors == next.behaviors
        && current.memories == next.memories
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
        behaviors: Vec::new(),
        memories: Vec::new(),
        hooks: Vec::new(),
        interfaces: Vec::new(),
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
        let ext_dir = root.join("observatory");
        fs::create_dir_all(&ext_dir).expect("create extension dir");
        fs::write(ext_dir.join("extension.toml"), sample_descriptor()).expect("write descriptor");

        let config = ExtensionRuntimeConfig {
            registry_file: root.join("config/extensions.toml"),
            logs_dir: root.join("logs"),
            home_dir: root.clone(),
        };
        write_registry_file(
            &config.registry_file,
            &ExtensionRegistryFile {
                extensions: vec![ExtensionRegistryEntry {
                    id: "observatory".to_string(),
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
        fs::write(ext_dir.join("extension.toml"), sample_descriptor_for("foo"))
            .expect("write descriptor");

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
mode = "dev"
root = "."
dev = true

[ui]
runtime = "browser-esm"
entry = "./ui/index.ts"
dev_url = "http://127.0.0.1:4201/src/index.ts"
hmr = true

[worker]
kind = "wasm"
entry = "./worker/plugin.wasm"
abi = "ennoia.worker.v1"

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
themes = [{{ id = "{id}.daybreak", label = {{ key = "ext.{id}.theme.daybreak", fallback = "Daybreak" }}, appearance = "Light", tokens_entry = "ui/themes/daybreak.css", preview_color = "#F4A261", extends = "system", category = "extension" }}]
locales = [{{ locale = "zh-CN", namespace = "ext.{id}", entry = "ui/locales/zh-CN.json", version = "1" }}, {{ locale = "en-US", namespace = "ext.{id}", entry = "ui/locales/en-US.json", version = "1" }}]
commands = [{{ id = "{id}.open", title = {{ key = "ext.{id}.command.open", fallback = "Open Observatory" }}, action = "open-page", shortcut = "Ctrl+Shift+O" }}]
providers = [{{ id = "{id}.feed", kind = "activity-feed", entry = "worker/providers/activity-feed.js" }}]
hooks = [{{ event = "run.completed", handler = "worker/hooks/run-completed.js" }}]
"##
        )
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ennoia-{prefix}-{}", unique_suffix()))
    }
}
