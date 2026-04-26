use serde_json::Value as JsonValue;

pub fn normalize_error_message(message: impl AsRef<str>) -> String {
    let raw = message.as_ref().trim();
    if raw.is_empty() {
        return "unknown error".to_string();
    }

    if let Some(json_message) = extract_json_error_message(raw) {
        return json_message;
    }

    let lines = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return raw.to_string();
    }

    if let Some(line) = lines
        .iter()
        .rev()
        .copied()
        .find(|line| is_leaf_error_line(line))
    {
        return line.to_string();
    }

    if let Some(line) = lines
        .iter()
        .rev()
        .copied()
        .find(|line| !looks_like_stack_line(line) && !looks_like_noise_line(line))
    {
        return line.to_string();
    }

    lines.last().copied().unwrap_or(raw).to_string()
}

fn extract_json_error_message(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<JsonValue>(raw).ok()?;
    best_json_error_message(&value)
}

fn best_json_error_message(value: &JsonValue) -> Option<String> {
    string_field(value, &["message"])
        .or_else(|| string_field(value, &["error"]))
        .or_else(|| string_field(value, &["error", "message"]))
        .or_else(|| string_field(value, &["top_reason"]))
        .or_else(|| string_field(value, &["error", "code"]))
        .or_else(|| string_field(value, &["code"]))
        .or_else(|| first_array_string_field(value, "failures", &["error_message"]))
        .or_else(|| first_array_string_field(value, "failures", &["top_reason"]))
        .or_else(|| first_array_string_field(value, "failures", &["error_code"]))
}

fn string_field(value: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn first_array_string_field(
    value: &JsonValue,
    array_key: &str,
    item_path: &[&str],
) -> Option<String> {
    let items = value.get(array_key)?.as_array()?;
    for item in items {
        if let Some(found) = string_field(item, item_path) {
            return Some(found);
        }
    }
    None
}

fn is_leaf_error_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("error:")
        || lower.starts_with("panic:")
        || lower.starts_with("exception:")
        || lower.starts_with("caused by:")
}

fn looks_like_stack_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("at ")
        || lower.starts_with("file://")
        || lower.starts_with("node:")
        || lower.starts_with("node.js ")
        || lower.starts_with("stack backtrace:")
        || line.starts_with('^')
}

fn looks_like_noise_line(line: &str) -> bool {
    matches!(line, "{" | "}" | "[" | "]")
}

#[cfg(test)]
mod tests {
    use super::normalize_error_message;

    #[test]
    fn keeps_plain_error_messages() {
        assert_eq!(
            normalize_error_message("provider returned empty text"),
            "provider returned empty text",
        );
    }

    #[test]
    fn extracts_last_error_line_from_stack_trace() {
        let raw = r#"file:///tmp/provider.js:111
throw new Error("broken");
^

Error: OpenAI request failed: 503 upstream_http_404
    at openaiFetch (file:///tmp/provider.js:111:11)
    at async Module.generate (file:///tmp/provider.js:62:20)

Node.js v22.14.0"#;
        assert_eq!(
            normalize_error_message(raw),
            "Error: OpenAI request failed: 503 upstream_http_404",
        );
    }

    #[test]
    fn extracts_best_message_from_json_body() {
        let raw = r#"{"error":"proxy_all_attempts_failed","code":"proxy_all_attempts_failed","top_reason":"upstream_http_404"}"#;
        assert_eq!(normalize_error_message(raw), "proxy_all_attempts_failed");
    }

    #[test]
    fn extracts_error_field_from_runner_envelope() {
        let raw = r#"{"ok":false,"error":"Error: OpenAI request failed: 503 upstream_http_404"}"#;
        assert_eq!(
            normalize_error_message(raw),
            "Error: OpenAI request failed: 503 upstream_http_404",
        );
    }
}
