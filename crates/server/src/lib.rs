//! Server exposes Ennoia over HTTP and hosts the persistence pool.

pub mod app;
pub mod db;
pub mod middleware;
pub mod routes;
pub mod system_config;

pub use app::{bootstrap_app_state, default_app_state, run_server, AppState};
pub use routes::build_router;
pub use system_config::SystemConfigRuntime;

/// Returns the current server module name.
pub fn module_name() -> &'static str {
    "server"
}
