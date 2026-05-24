use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtensionOutputBlock {
    pub kind: String,
    pub title: Option<String>,
    pub mime_type: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionOutputEnvelope {
    pub fallback: String,
    pub html_reply: Option<String>,
    pub artifact: Option<ExtensionOutputBlock>,
}

#[derive(Debug, Deserialize)]
struct RawExtensionOutputEnvelope {
    kind: String,
    profile: Option<String>,
    placement: Option<String>,
    content_type: Option<String>,
    title: Option<String>,
    fallback: Option<String>,
    body: Option<String>,
}

pub fn parse_extension_output_reply(body: &str) -> Option<ExtensionOutputEnvelope> {
    let parsed = serde_json::from_str::<RawExtensionOutputEnvelope>(body.trim()).ok()?;
    if !matches!(
        parsed.kind.as_str(),
        "ennoia.html_reply" | "ennoia.artifact_runner"
    ) {
        return None;
    }

    let profile = parsed.profile.as_deref().map(str::trim);
    let placement = parsed.placement.as_deref().map(str::trim);
    let content_type = parsed.content_type.as_deref().map(str::trim);
    let is_html_reply = parsed.kind == "ennoia.html_reply";
    let is_artifact_runner = parsed.kind == "ennoia.artifact_runner";
    let body = non_empty_string(parsed.body.as_deref());
    let html = if is_html_reply { body.clone() } else { None };
    let fallback = non_empty_string(parsed.fallback.as_deref())
        .or_else(|| html.as_ref().map(|value| strip_html_text(value)))
        .or_else(|| {
            body.as_ref()
                .map(|value| summarize_body(value, content_type))
        })
        .unwrap_or_else(|| "扩展输出内容".to_string());
    let artifact_placement = if is_artifact_runner {
        Some("artifact")
    } else {
        placement
    };
    let artifact = artifact_block_from_profile(
        profile,
        artifact_placement,
        content_type,
        parsed.title.as_deref(),
        body,
    );

    Some(ExtensionOutputEnvelope {
        fallback,
        html_reply: html,
        artifact,
    })
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn artifact_block_from_profile(
    profile: Option<&str>,
    placement: Option<&str>,
    content_type: Option<&str>,
    title: Option<&str>,
    body: Option<String>,
) -> Option<ExtensionOutputBlock> {
    if placement != Some("artifact") {
        return None;
    }
    let content = body?;
    let (kind, mime_type) = match (profile, content_type) {
        (Some("html-artifact"), _) | (_, Some("text/html")) => ("html-preview", "text/html"),
        (Some("python-artifact"), _) | (_, Some("text/x-python")) | (_, Some("text/python")) => {
            ("python-run", "text/x-python")
        }
        _ => ("text", content_type.unwrap_or("text/plain")),
    };

    Some(ExtensionOutputBlock {
        kind: kind.to_string(),
        title: non_empty_string(title),
        mime_type: Some(mime_type.to_string()),
        content,
    })
}

fn summarize_body(body: &str, content_type: Option<&str>) -> String {
    if content_type == Some("text/html") {
        return strip_html_text(body);
    }
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("扩展输出内容")
        .to_string()
}

fn strip_html_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn removed_combined_output_kind() -> String {
        format!("ennoia.{}_{}", "rich", "output")
    }

    #[test]
    fn ignores_removed_combined_output_envelope() {
        let body = serde_json::json!({
            "kind": removed_combined_output_kind(),
            "fallback": "这里是普通文本摘要。",
            "message": {
                "type": "html-rich",
                "html": "<section><h2>摘要</h2><p>这里是富排版。</p></section>"
            },
            "blocks": [
                {
                    "kind": "html-preview",
                    "title": "页面预览",
                    "mime_type": "text/html",
                    "content": "<!doctype html><html><body>demo</body></html>"
                }
            ]
        })
        .to_string();

        assert!(parse_extension_output_reply(&body).is_none());
    }

    #[test]
    fn ignores_plain_markdown_reply() {
        assert!(parse_extension_output_reply("普通 Markdown 回复").is_none());
    }

    #[test]
    fn routes_html_message_to_html_reply_extension_only() {
        let body = serde_json::json!({
            "kind": "ennoia.html_reply",
            "version": 1,
            "profile": "html-message",
            "placement": "message",
            "content_type": "text/html",
            "fallback": "普通文本摘要。",
            "body": "<section><h2>摘要</h2><p>富排版。</p></section>"
        })
        .to_string();

        let parsed = parse_extension_output_reply(&body).expect("parse html reply");

        assert_eq!(parsed.fallback, "普通文本摘要。");
        assert_eq!(
            parsed.html_reply.as_deref(),
            Some("<section><h2>摘要</h2><p>富排版。</p></section>")
        );
        assert_eq!(parsed.artifact, None);
    }

    #[test]
    fn routes_html_artifact_to_artifact_runner_extension_only() {
        let body = serde_json::json!({
            "kind": "ennoia.artifact_runner",
            "version": 1,
            "profile": "html-artifact",
            "placement": "artifact",
            "content_type": "text/html",
            "title": "登录页原型",
            "fallback": "我生成了一个登录页原型。",
            "body": "<!doctype html><html><body>demo</body></html>"
        })
        .to_string();

        let parsed = parse_extension_output_reply(&body).expect("parse artifact runner");

        assert_eq!(parsed.fallback, "我生成了一个登录页原型。");
        assert_eq!(parsed.html_reply, None);
        let artifact = parsed.artifact.expect("artifact");
        assert_eq!(artifact.kind, "html-preview");
        assert_eq!(artifact.title.as_deref(), Some("登录页原型"));
        assert_eq!(artifact.mime_type.as_deref(), Some("text/html"));
    }

    #[test]
    fn ignores_removed_combined_output_html_message_profile_envelope() {
        let body = serde_json::json!({
            "kind": removed_combined_output_kind(),
            "version": 1,
            "profile": "html-message",
            "placement": "message",
            "content_type": "text/html",
            "fallback": "普通文本摘要。",
            "body": "<section><h2>摘要</h2><p>富排版。</p></section>"
        })
        .to_string();

        assert!(parse_extension_output_reply(&body).is_none());
    }

    #[test]
    fn ignores_removed_combined_output_html_artifact_profile_envelope() {
        let body = serde_json::json!({
            "kind": removed_combined_output_kind(),
            "version": 1,
            "profile": "html-artifact",
            "placement": "artifact",
            "content_type": "text/html",
            "title": "登录页原型",
            "fallback": "我生成了一个登录页原型。",
            "body": "<!doctype html><html><body>demo</body></html>"
        })
        .to_string();

        assert!(parse_extension_output_reply(&body).is_none());
    }

    #[test]
    fn ignores_removed_combined_output_python_artifact_profile_envelope() {
        let body = serde_json::json!({
            "kind": removed_combined_output_kind(),
            "version": 1,
            "profile": "python-artifact",
            "placement": "artifact",
            "content_type": "text/x-python",
            "title": "Python 示例",
            "fallback": "我生成了一个 Python 示例。",
            "body": "print('hello')"
        })
        .to_string();

        assert!(parse_extension_output_reply(&body).is_none());
    }
}
