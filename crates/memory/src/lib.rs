//! SqliteMemoryStore: the canonical MemoryStore implementation backed by sqlx + sqlite.

pub mod sqlite;

pub use sqlite::SqliteMemoryStore;

pub fn module_name() -> &'static str {
    "memory"
}
