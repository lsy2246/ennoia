use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct Invocation {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    context: Value,
}

#[no_mangle]
pub extern "C" fn ennoia_worker_alloc(len: i32) -> i32 {
    if len <= 0 {
        return 0;
    }
    let mut buffer = Vec::<u8>::with_capacity(len as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr as i32
}

#[no_mangle]
pub unsafe extern "C" fn ennoia_worker_dealloc(ptr: i32, len: i32) {
    if ptr <= 0 || len <= 0 {
        return;
    }
    let _ = Vec::from_raw_parts(ptr as *mut u8, 0, len as usize);
}

#[no_mangle]
pub unsafe extern "C" fn ennoia_worker_handle(ptr: i32, len: i32) -> i64 {
    let input = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    let output = match serde_json::from_slice::<Invocation>(input) {
        Ok(invocation) => handle(invocation),
        Err(error) => failure("invalid_request", error.to_string()),
    };
    pack_response(output)
}

fn handle(invocation: Invocation) -> String {
    let path = invocation.method.trim_matches('/');
    let _context = invocation.context;
    match path {
        "memory/workspace" => success(json!({
            "conversations": [],
            "pending_review_count": 0,
            "active_memory_count": 1,
            "message_count": 0,
            "graph_nodes_count": 1
        })),
        "memory/conversations/list" => success(json!([sample_conversation()])),
        "memory/conversations/create" => {
            let title = invocation
                .params
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("新会话");
            let conversation_id = "wasm-conversation-1";
            let lane_id = "wasm-lane-1";
            let conversation = json!({
                "id": conversation_id,
                "topology": invocation.params.get("topology").and_then(Value::as_str).unwrap_or("direct"),
                "owner": invocation.params.get("owner").cloned().unwrap_or(json!({ "kind": "global", "id": "runtime" })),
                "space_id": invocation.params.get("space_id").cloned().unwrap_or(Value::Null),
                "title": title,
                "participants": invocation.params.get("agent_ids").cloned().unwrap_or_else(|| {
                    invocation.params.get("participants").cloned().unwrap_or(json!([]))
                }),
                "default_lane_id": lane_id,
                "created_at": "0",
                "updated_at": "0"
            });
            success(json!({
                "conversation": conversation,
                "default_lane": sample_lane(conversation_id)
            }))
        }
        "memory/conversations/get" => success(json!(sample_conversation())),
        "memory/conversations/delete" => success(json!({ "deleted": true })),
        "memory/lanes/list-by-conversation" => success(json!([sample_lane(
            invocation
                .params
                .get("conversation_id")
                .and_then(Value::as_str)
                .unwrap_or("wasm-conversation-1")
        )])),
        "memory/messages/list" => success(json!([])),
        "memory/messages/append-user" | "memory/messages/append-agent" => {
            let conversation_id = invocation
                .params
                .get("conversation_id")
                .and_then(Value::as_str)
                .unwrap_or("wasm-conversation-1");
            let message = invocation
                .params
                .get("message")
                .cloned()
                .unwrap_or(Value::Null);
            let lane_id = message
                .get("lane_id")
                .and_then(Value::as_str)
                .unwrap_or("wasm-lane-1");
            let message = json!({
                "id": "wasm-message-1",
                "conversation_id": conversation_id,
                "lane_id": lane_id,
                "sender": message.get("sender").and_then(Value::as_str).unwrap_or("operator"),
                "role": message.get("role").and_then(Value::as_str).unwrap_or("operator"),
                "body": message.get("body").and_then(Value::as_str).unwrap_or(""),
                "mentions": message.get("addressed_agents").cloned().unwrap_or_else(|| {
                    message.get("mentions").cloned().unwrap_or(json!([]))
                }),
                "created_at": "0"
            });
            success(json!({
                "conversation": sample_conversation(),
                "lane": sample_lane(conversation_id),
                "message": message,
                "runs": [],
                "tasks": [],
                "artifacts": []
            }))
        }
        "memory/memories" => success(json!(sample_memories())),
        "memory/memories/recall" => {
            let memories = sample_memories();
            let total_chars = memories
                .iter()
                .filter_map(|item| item.get("content").and_then(Value::as_str))
                .map(str::len)
                .sum::<usize>();
            success(json!({
                "memories": memories,
                "receipt_id": "wasm-memory-recall",
                "mode": invocation
                    .params
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("hybrid"),
                "total_chars": total_chars
            }))
        }
        "memory/memories/review" => success(json!({
            "reviewed": true,
            "target_memory_id": invocation.params.get("target_memory_id").cloned().unwrap_or(Value::Null),
            "action": invocation.params.get("action").cloned().unwrap_or(Value::Null)
        })),
        _ => failure(
            "method_not_found",
            format!("memory worker method '{path}' not found"),
        ),
    }
}

fn sample_conversation() -> Value {
    json!({
        "id": "wasm-conversation-1",
        "topology": "direct",
        "owner": { "kind": "global", "id": "runtime" },
        "space_id": null,
        "title": "Wasm Memory Conversation",
        "participants": [],
        "default_lane_id": "wasm-lane-1",
        "created_at": "0",
        "updated_at": "0"
    })
}

fn sample_lane(conversation_id: &str) -> Value {
    json!({
        "id": "wasm-lane-1",
        "conversation_id": conversation_id,
        "space_id": null,
        "name": "Main",
        "lane_type": "main",
        "status": "open",
        "goal": "默认会话线路",
        "participants": [],
        "created_at": "0",
        "updated_at": "0"
    })
}

fn sample_memories() -> Vec<Value> {
    vec![json!({
        "id": "wasm-memory-1",
        "owner": { "kind": "system", "id": "ennoia" },
        "namespace": "ennoia.runtime",
        "memory_kind": "truth",
        "stability": "stable",
        "status": "active",
        "superseded_by": null,
        "title": "Wasm Memory Worker",
        "content": "Memory capability is served by an Ennoia Wasm Worker.",
        "summary": "Memory Worker 已通过 Wasm ABI 接入。",
        "confidence": 1.0,
        "importance": 0.7,
        "valid_from": null,
        "valid_to": null,
        "sources": [{ "kind": "extension", "reference": "memory/worker/memory.wasm" }],
        "tags": ["wasm", "memory"],
        "entities": ["Ennoia"],
        "created_at": "0",
        "updated_at": "0"
    })]
}

fn success(data: Value) -> String {
    json!({ "ok": true, "data": data, "error": null }).to_string()
}

fn failure(code: &str, message: String) -> String {
    json!({
        "ok": false,
        "data": null,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

fn pack_response(response: String) -> i64 {
    let mut bytes = response.into_bytes();
    let ptr = bytes.as_mut_ptr() as usize;
    let len = bytes.len();
    std::mem::forget(bytes);
    ((ptr as i64) << 32) | len as i64
}
