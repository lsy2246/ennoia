//! Memory module owns its domain model, store contract and sqlite implementation.

pub mod model;
pub mod schema;
pub mod sqlite;

pub use model::*;
pub use schema::initialize_memory_schema;
pub use sqlite::SqliteMemoryStore;

pub fn module_name() -> &'static str {
    "memory"
}
