//! Policy crate owns configuration for memory and runtime behavior.

pub mod glob;
pub mod memory;
pub mod runtime;

use std::fs;
use std::path::{Path, PathBuf};

pub use glob::GlobPattern;
pub use memory::MemoryPolicy;
pub use runtime::{RuntimeRule, RuntimeRuleCondition, RuntimeRuleTarget, StagePolicy};

/// PolicySet is the bundle loaded from `<home>/policies/`.
#[derive(Debug, Clone, Default)]
pub struct PolicySet {
    pub memory: MemoryPolicy,
    pub stage: StagePolicy,
}

impl PolicySet {
    /// Loads policies from the given directory. Falls back to defaults per file when absent.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, PolicyError> {
        let dir = dir.as_ref();

        let memory = load_toml_or_default::<MemoryPolicy>(dir.join("memory.toml"))?;
        let stage = load_toml_or_default::<StagePolicy>(dir.join("stage.toml"))?;

        Ok(Self { memory, stage })
    }

    pub fn builtin() -> Self {
        Self {
            memory: MemoryPolicy::builtin(),
            stage: StagePolicy::builtin(),
        }
    }
}

#[derive(Debug)]
pub enum PolicyError {
    Io(std::io::Error),
    Toml(toml::de::Error),
}

impl From<std::io::Error> for PolicyError {
    fn from(error: std::io::Error) -> Self {
        PolicyError::Io(error)
    }
}

impl From<toml::de::Error> for PolicyError {
    fn from(error: toml::de::Error) -> Self {
        PolicyError::Toml(error)
    }
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::Io(error) => write!(f, "policy io error: {error}"),
            PolicyError::Toml(error) => write!(f, "policy toml error: {error}"),
        }
    }
}

impl std::error::Error for PolicyError {}

fn load_toml_or_default<T>(path: PathBuf) -> Result<T, PolicyError>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}
