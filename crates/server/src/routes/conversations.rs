use super::*;

pub(super) async fn conversations_list(
    State(state): State<AppState>,
) -> Json<Vec<ConversationSpec>> {
    Json(
        db::list_conversations(&state.pool)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn conversations_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<CreateConversationPayload>,
) -> ApiResult<ConversationCreateResponse> {
    let requested_topology =
        conversation_topology_from_value(&payload.topology).ok_or_else(|| {
            scoped(
                ApiError::bad_request("invalid conversation topology"),
                &request,
            )
        })?;
    let agent_ids = normalize_agent_ids(&state.runtime_paths, &payload.agent_ids);
    if agent_ids.is_empty() {
        return Err(scoped(
            ApiError::bad_request("at least one agent is required"),
            &request,
        ));
    }
    let topology = infer_topology_from_agent_count(requested_topology, &agent_ids);

    let now = now_iso();
    let participants = build_participants(&agent_ids);
    let owner = resolve_owner(&topology, payload.space_id.as_deref(), &agent_ids);
    let conversation_id = format!("conv-{}", Uuid::new_v4());
    let lane_id = format!("lane-{}", Uuid::new_v4());
    let conversation = ConversationSpec {
        id: conversation_id.clone(),
        topology,
        owner,
        space_id: payload.space_id.clone(),
        title: payload
            .title
            .unwrap_or_else(|| default_conversation_title(&state.runtime_paths, &agent_ids)),
        participants: participants.clone(),
        default_lane_id: Some(lane_id.clone()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let lane = LaneSpec {
        id: lane_id,
        conversation_id: conversation_id,
        space_id: payload.space_id,
        name: payload
            .lane_name
            .unwrap_or_else(|| build_default_lane_name(&agent_ids)),
        lane_type: payload.lane_type.unwrap_or_else(|| "primary".to_string()),
        status: "active".to_string(),
        goal: payload
            .lane_goal
            .unwrap_or_else(|| build_default_lane_goal(&agent_ids)),
        participants,
        created_at: now.clone(),
        updated_at: now,
    };

    db::upsert_conversation(&state.pool, &conversation)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    db::insert_lane(&state.pool, &lane)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    Ok(Json(ConversationCreateResponse {
        conversation,
        default_lane: lane,
    }))
}

pub(super) async fn conversation_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<ConversationDetailResponse> {
    let conversation = db::get_conversation(&state.pool, &conversation_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| scoped(ApiError::not_found("conversation not found"), &request))?;
    let lanes = db::list_lanes_for_conversation(&state.pool, &conversation_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(ConversationDetailResponse {
        conversation,
        lanes,
    }))
}

pub(super) async fn conversation_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = db::delete_conversation(&state.pool, &conversation_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    if !deleted {
        return Err(scoped(
            ApiError::not_found("conversation not found"),
            &request,
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn conversation_messages(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<MessageSpec>> {
    Json(
        db::list_messages_for_conversation(&state.pool, &conversation_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn conversation_messages_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<ConversationMessagePayload>,
) -> ApiResult<ConversationEnvelope> {
    let conversation = db::get_conversation(&state.pool, &conversation_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| scoped(ApiError::not_found("conversation not found"), &request))?;
    let lanes = db::list_lanes_for_conversation(&state.pool, &conversation_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let lane = select_lane(&lanes, payload.lane_id.as_deref())
        .ok_or_else(|| scoped(ApiError::bad_request("lane not found"), &request))?;
    let goal = payload.goal.clone().unwrap_or_else(|| payload.body.clone());
    drive_run(
        &state,
        conversation,
        lane,
        &payload.body,
        &goal,
        payload.addressed_agents,
    )
    .await
    .map(Json)
    .map_err(|error| scoped(ApiError::bad_request(error), &request))
}

pub(super) async fn conversation_runs(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<RunSpec>> {
    Json(
        db::list_runs_for_conversation(&state.pool, &conversation_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn conversation_lanes(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<LaneSpec>> {
    Json(
        db::list_lanes_for_conversation(&state.pool, &conversation_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn lane_handoffs(
    State(state): State<AppState>,
    Path(lane_id): Path<String>,
) -> Json<Vec<HandoffSpec>> {
    Json(
        db::list_handoffs_for_lane(&state.pool, &lane_id)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn lane_handoffs_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(lane_id): Path<String>,
    Json(payload): Json<HandoffPayload>,
) -> ApiResult<HandoffSpec> {
    let handoff = HandoffSpec {
        id: format!("handoff-{}", Uuid::new_v4()),
        from_lane_id: lane_id,
        to_lane_id: payload.to_lane_id,
        from_agent_id: payload.from_agent_id,
        to_agent_id: payload.to_agent_id,
        summary: payload.summary,
        instructions: payload.instructions,
        status: payload.status.unwrap_or_else(|| "open".to_string()),
        created_at: now_iso(),
    };
    db::insert_handoff(&state.pool, &handoff)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(handoff))
}
