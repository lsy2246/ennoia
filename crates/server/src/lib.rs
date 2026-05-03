//! Server exposes Ennoia over HTTP and hosts the extension runtime.

pub mod agent_permissions;
pub mod app;
pub mod event_bus;
pub mod execution;
pub mod logs_store;
pub mod middleware;
pub mod pipeline;
pub mod routes;
pub mod runtime_bridge;

pub use app::{bootstrap_app_state, default_app_state, run_server, AppState};
pub use routes::build_router;

/// Returns the current server module name.
pub fn module_name() -> &'static str {
    "server"
}
