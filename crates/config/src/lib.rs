//! SqliteConfigStore: canonical ConfigStore implementation.

pub mod sqlite;

pub use sqlite::SqliteConfigStore;

pub fn module_name() -> &'static str {
    "config"
}
