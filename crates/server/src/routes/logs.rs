use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LogRecord {
    id: String,
    kind: String,
    level: String,
    source: String,
    title: String,
    summary: String,
    #[serde(default)]
    details: Option<String>,
    at: String,
}

pub(super) async fn logs_list(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Json<Vec<LogRecord>> {
    let mut records = read_frontend_logs(&state);
    records.retain(|record| matches_log_query(record, &query));
    records.sort_by(|left, right| right.at.cmp(&left.at));
    records.truncate(query.limit.unwrap_or(50) as usize);
    Json(records)
}

pub(super) async fn frontend_log_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<FrontendLogPayload>,
) -> Result<StatusCode, ApiError> {
    let record = LogRecord {
        id: format!("flog-{}", Uuid::new_v4()),
        kind: "frontend".to_string(),
        level: payload.level,
        source: normalize_log_source(payload.source),
        title: payload.title,
        summary: payload.summary,
        details: payload.details,
        at: payload.at.unwrap_or_else(now_iso),
    };

    append_frontend_log(&state, &record)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(StatusCode::NO_CONTENT)
}

fn read_frontend_logs(state: &AppState) -> Vec<LogRecord> {
    let path = frontend_log_file(state);
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| serde_json::from_str::<LogRecord>(line).ok())
        .collect()
}

fn append_frontend_log(state: &AppState, record: &LogRecord) -> std::io::Result<()> {
    let path = frontend_log_file(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(record).map_err(std::io::Error::other)?;
    line.push('\n');
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    std::io::Write::write_all(&mut options.open(path)?, line.as_bytes())
}

fn frontend_log_file(state: &AppState) -> PathBuf {
    state.runtime_paths.logs_dir().join("frontend.jsonl")
}

fn normalize_log_source(source: Option<String>) -> String {
    let source = source.unwrap_or_else(|| "frontend".to_string());
    let trimmed = source.trim();
    if trimmed.is_empty() {
        "frontend".to_string()
    } else {
        trimmed.to_string()
    }
}

fn matches_log_query(record: &LogRecord, query: &LogsQuery) -> bool {
    query
        .level
        .as_deref()
        .is_none_or(|level| record.level.eq_ignore_ascii_case(level))
        && query
            .source
            .as_deref()
            .is_none_or(|source| record.source.eq_ignore_ascii_case(source))
        && query.q.as_deref().is_none_or(|needle| {
            let needle = needle.to_lowercase();
            record.title.to_lowercase().contains(&needle)
                || record.summary.to_lowercase().contains(&needle)
                || record
                    .details
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
        })
}
