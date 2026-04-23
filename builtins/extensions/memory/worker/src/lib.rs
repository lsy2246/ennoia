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
