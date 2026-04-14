//! Server exposes Ennoia over HTTP, WebSocket and static asset hosting.

pub mod app;
pub mod routes;

pub use app::{default_app_state, AppState};
pub use routes::build_router;

/// Returns the current server module name.
pub fn module_name() -> &'static str {
    "server"
}
