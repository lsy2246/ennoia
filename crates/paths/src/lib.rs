//! Runtime path resolver for Ennoia.
//!
//! All runtime directory and file locations must flow through `RuntimePaths`
//! instead of being assembled ad hoc inside product crates.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ennoia_kernel::{OwnerKind, OwnerRef};

pub const ENNOIA_HOME_ENV: &str = "ENNOIA_HOME";

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    home: PathBuf,
}

impl RuntimePaths {
    pub fn resolve(argument: Option<&str>) -> Self {
        let home = argument
            .map(PathBuf::from)
            .or_else(|| env::var_os(ENNOIA_HOME_ENV).map(PathBuf::from))
            .unwrap_or_else(default_home_dir);
        Self { home }
    }

    pub fn new(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self { home }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn config_dir(&self) -> PathBuf {
        self.home.join("config")
    }

    pub fn agents_config_dir(&self) -> PathBuf {
        self.config_dir().join("agents")
    }

    pub fn extensions_registry_file(&self) -> PathBuf {
        self.config_dir().join("extensions.toml")
    }

    pub fn skills_registry_file(&self) -> PathBuf {
        self.config_dir().join("skills.toml")
    }

    pub fn providers_config_dir(&self) -> PathBuf {
        self.config_dir().join("providers")
    }

    pub fn app_config_file(&self) -> PathBuf {
        self.config_dir().join("ennoia.toml")
    }

    pub fn server_config_file(&self) -> PathBuf {
        self.config_dir().join("server.toml")
    }

    pub fn ui_config_file(&self) -> PathBuf {
        self.config_dir().join("ui.toml")
    }

    pub fn profile_config_file(&self) -> PathBuf {
        self.config_dir().join("profile.toml")
    }

    pub fn interfaces_config_file(&self) -> PathBuf {
        self.config_dir().join("interfaces.toml")
    }

    pub fn preferences_dir(&self) -> PathBuf {
        self.config_dir().join("preferences")
    }

    pub fn instance_preference_file(&self) -> PathBuf {
        self.preferences_dir().join("instance.toml")
    }

    pub fn space_preferences_dir(&self) -> PathBuf {
        self.preferences_dir().join("spaces")
    }

    pub fn space_preference_file(&self, space_id: &str) -> PathBuf {
        self.space_preferences_dir()
            .join(format!("{space_id}.toml"))
    }

    pub fn policies_dir(&self) -> PathBuf {
        self.home.join("policies")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.home.join("data")
    }

    pub fn state_queue_dir(&self) -> PathBuf {
        self.state_dir().join("queue")
    }

    pub fn state_runs_dir(&self) -> PathBuf {
        self.state_dir().join("runs")
    }

    pub fn state_cache_dir(&self) -> PathBuf {
        self.state_dir().join("cache")
    }

    pub fn extensions_state_dir(&self) -> PathBuf {
        self.state_dir().join("extensions")
    }

    pub fn schedules_file(&self) -> PathBuf {
        self.system_state_dir().join("schedules.json")
    }

    pub fn system_state_dir(&self) -> PathBuf {
        self.state_dir().join("system")
    }

    pub fn system_sqlite_dir(&self) -> PathBuf {
        self.system_state_dir().join("sqlite")
    }

    pub fn system_log_db(&self) -> PathBuf {
        self.system_sqlite_dir().join("system-log.db")
    }

    pub fn system_events_db(&self) -> PathBuf {
        self.system_sqlite_dir().join("events.db")
    }

    pub fn extension_state_dir(&self, extension_id: &str) -> PathBuf {
        self.extensions_state_dir().join(extension_id)
    }

    pub fn extension_sqlite_dir(&self, extension_id: &str) -> PathBuf {
        self.extension_state_dir(extension_id).join("sqlite")
    }

    pub fn extension_sqlite_db(&self, extension_id: &str, file_name: &str) -> PathBuf {
        self.extension_sqlite_dir(extension_id).join(file_name)
    }

    pub fn global_dir(&self) -> PathBuf {
        self.home.join("global")
    }

    pub fn extensions_dir(&self) -> PathBuf {
        self.home.join("extensions")
    }

    pub fn extension_dir(&self, extension_id: &str) -> PathBuf {
        self.extensions_dir().join(extension_id)
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.home.join("skills")
    }

    pub fn skill_dir(&self, skill_id: &str) -> PathBuf {
        self.skills_dir().join(skill_id)
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.home.join("agents")
    }

    pub fn agent_dir(&self, agent_id: &str) -> PathBuf {
        self.agents_dir().join(agent_id)
    }

    pub fn agent_skills_dir(&self, agent_id: &str) -> PathBuf {
        self.agent_dir(agent_id).join("skills")
    }

    pub fn agent_working_dir(&self, agent_id: &str) -> PathBuf {
        self.agent_dir(agent_id).join("work")
    }

    pub fn agent_artifacts_dir(&self, agent_id: &str) -> PathBuf {
        self.agent_dir(agent_id).join("artifacts")
    }

    pub fn spaces_dir(&self) -> PathBuf {
        self.home.join("spaces")
    }

    pub fn space_dir(&self, space_id: &str) -> PathBuf {
        self.spaces_dir().join(space_id)
    }

    pub fn space_working_dir(&self, space_id: &str) -> PathBuf {
        self.space_dir(space_id).join("work")
    }

    pub fn space_artifacts_dir(&self, space_id: &str) -> PathBuf {
        self.space_dir(space_id).join("artifacts")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.home.join("logs")
    }

    pub fn server_logs_dir(&self) -> PathBuf {
        self.logs_dir().join("server")
    }

    pub fn agents_logs_dir(&self) -> PathBuf {
        self.logs_dir().join("agents")
    }

    pub fn spaces_logs_dir(&self) -> PathBuf {
        self.logs_dir().join("spaces")
    }

    pub fn extensions_logs_dir(&self) -> PathBuf {
        self.logs_dir().join("extensions")
    }

    pub fn owner_run_artifact_dir(&self, owner: &OwnerRef, run_id: &str) -> PathBuf {
        match owner.kind {
            OwnerKind::Agent => self
                .agent_artifacts_dir(&owner.id)
                .join(format!("runs/{run_id}")),
            OwnerKind::Space => self
                .space_artifacts_dir(&owner.id)
                .join(format!("runs/{run_id}")),
            OwnerKind::Global => self.home.join("global").join("runs").join(run_id),
        }
    }

    pub fn owner_run_artifact_relative_path(&self, owner: &OwnerRef, run_id: &str) -> String {
        match owner.kind {
            OwnerKind::Agent => format!("agents/{}/artifacts/runs/{run_id}/summary.json", owner.id),
            OwnerKind::Space => format!("spaces/{}/artifacts/runs/{run_id}/summary.json", owner.id),
            OwnerKind::Global => format!("global/runs/{run_id}/summary.json"),
        }
    }

    pub fn ensure_layout(&self) -> io::Result<()> {
        for dir in [
            self.agents_config_dir(),
            self.providers_config_dir(),
            self.preferences_dir(),
            self.space_preferences_dir(),
            self.extensions_dir(),
            self.skills_dir(),
            self.state_queue_dir(),
            self.state_runs_dir(),
            self.state_cache_dir(),
            self.extensions_state_dir(),
            self.system_sqlite_dir(),
            self.agents_dir(),
            self.server_logs_dir(),
            self.agents_logs_dir(),
            self.spaces_logs_dir(),
            self.extensions_logs_dir(),
        ] {
            fs::create_dir_all(dir)?;
        }

        Ok(())
    }

    pub fn expand_home_token(&self, value: &str) -> PathBuf {
        if let Some(rest) = value.strip_prefix("~/.ennoia") {
            return self.home.join(rest.trim_start_matches(['/', '\\']));
        }
        PathBuf::from(value)
    }

    pub fn display_with_home_token(&self, path: impl AsRef<Path>) -> String {
        let path = path.as_ref();
        if let Ok(stripped) = path.strip_prefix(&self.home) {
            let suffix = stripped.to_string_lossy().replace('\\', "/");
            if suffix.is_empty() {
                "~/.ennoia".to_string()
            } else {
                format!("~/.ennoia/{suffix}")
            }
        } else {
            path.to_string_lossy().replace('\\', "/")
        }
    }

    pub fn display_for_user(&self, path: impl AsRef<Path>) -> String {
        path.as_ref().to_string_lossy().replace('\\', "/")
    }
}

pub fn default_home_dir() -> PathBuf {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ennoia")
}
