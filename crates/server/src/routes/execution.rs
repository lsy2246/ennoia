use super::*;

pub(super) async fn runs(State(state): State<AppState>) -> Json<Vec<RunSpec>> {
    Json(db::list_runs(&state.pool).await.unwrap_or_default())
}

pub(super) async fn run_tasks(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<TaskSpec>> {
    Json(
        db::list_tasks_for_run(&state.pool, &run_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn run_artifacts(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ArtifactSpec>> {
    Json(
        db::list_artifacts_for_run(&state.pool, &run_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn run_stages(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::RunStageEvent>> {
    Json(
        state
            .runtime_store
            .list_stage_events_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn run_decisions(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::DecisionSnapshot>> {
    Json(
        state
            .runtime_store
            .list_decisions_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn run_gates(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::GateRecord>> {
    Json(
        state
            .runtime_store
            .list_gate_verdicts_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn tasks(State(state): State<AppState>) -> Json<Vec<TaskSpec>> {
    Json(db::list_tasks(&state.pool).await.unwrap_or_default())
}

pub(super) async fn artifacts(State(state): State<AppState>) -> Json<Vec<ArtifactSpec>> {
    Json(db::list_artifacts(&state.pool).await.unwrap_or_default())
}
