use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::app::AppState;

/// Builds the HTTP router for the Ennoia server skeleton.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/extensions", get(extensions))
        .route("/api/v1/agents", get(agents))
        .route("/api/v1/spaces", get(spaces))
        .route("/api/v1/runs", get(runs))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    app: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "Ennoia",
    })
}

async fn overview(State(state): State<AppState>) -> Json<ennoia_kernel::PlatformOverview> {
    Json(state.overview)
}

async fn extensions(State(state): State<AppState>) -> Json<Vec<String>> {
    Json(
        state
            .extensions
            .items()
            .iter()
            .map(|item| item.manifest.id.clone())
            .collect(),
    )
}

async fn agents(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::AgentConfig>> {
    Json(state.agents)
}

async fn spaces(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::SpaceSpec>> {
    Json(state.spaces)
}

async fn runs(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::RunSpec>> {
    Json(state.runs)
}
