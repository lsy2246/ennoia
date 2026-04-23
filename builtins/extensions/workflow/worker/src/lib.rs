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
        "behavior/status" => success(json!({
            "extension_id": "workflow",
            "behavior_id": "default",
            "healthy": true,
            "version": "1",
            "interfaces": ["runs", "tasks", "artifacts", "handoffs", "status"]
        })),
        "behavior/runs" | "behavior/run" | "behavior/start" => {
            let goal = invocation
                .params
                .get("goal")
                .and_then(Value::as_str)
                .unwrap_or("Wasm workflow run");
            success(json!({
                "run": {
                    "id": "wasm-run-1",
                    "owner": invocation.params.get("owner").cloned().unwrap_or(json!({ "kind": "operator", "id": "local" })),
                    "goal": goal,
                    "status": "planned",
                    "stage": "planning",
                    "created_at": "0",
                    "updated_at": "0"
                },
                "tasks": [],
                "artifacts": [],
                "handoffs": [],
                "stage_events": [],
                "decision": {
                    "id": "wasm-decision-1",
                    "summary": "Workflow Worker accepted the run request.",
                    "rationale": "Handled inside ennoia.worker.v1 sandbox.",
                    "created_at": "0"
                },
                "gate_verdicts": []
            }))
        }
        path if path.starts_with("behavior/") => success(json!({
            "handled": true,
            "path": path,
            "source": "workflow.wasm"
        })),
        _ => failure(
            "method_not_found",
            format!("workflow worker method '{path}' not found"),
        ),
    }
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
