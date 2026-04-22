//! Server exposes Ennoia over HTTP and hosts the extension runtime.

pub mod app;
pub mod middleware;
pub mod routes;

pub use app::{bootstrap_app_state, default_app_state, run_server, AppState};
pub use routes::build_router;

/// Returns the current server module name.
pub fn module_name() -> &'static str {
    "server"
}
