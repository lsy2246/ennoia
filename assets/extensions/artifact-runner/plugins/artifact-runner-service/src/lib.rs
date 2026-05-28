use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::time::Instant;

use ennoia_kernel::ExtensionRpcResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tempfile::tempdir;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const PYTHON_TIMEOUT_MS: u64 = 10_000;
const MAX_CODE_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize)]
struct Invocation {
    method: String,
    #[serde(default)]
    params: JsonValue,
    #[serde(default)]
    context: JsonValue,
}

#[derive(Debug, Deserialize)]
struct PythonRunPayload {
    code: String,
    #[serde(default)]
    record_id: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PythonRunResult {
    ok: bool,
    command: String,
    args: Vec<String>,
    cwd: String,
    record_id: Option<String>,
    conversation_id: Option<String>,
    agent_id: Option<String>,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }

        let response = match serde_json::from_str::<Invocation>(line.trim_end()) {
            Ok(invocation) => handle_invocation(invocation).await,
            Err(error) => ExtensionRpcResponse::failure("invalid_request", error.to_string()),
        };

        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }

    Ok(())
}

async fn handle_invocation(invocation: Invocation) -> ExtensionRpcResponse {
    let path = invocation.method.trim_matches('/');
    let _context = invocation.context;
    match path {
        "artifact.run_python" => match parse_json::<PythonRunPayload>(invocation.params) {
            Ok(payload) => match run_python(payload).await {
                Ok(result) => ExtensionRpcResponse::success(serde_json::json!(result)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        _ => ExtensionRpcResponse::failure(
            "method_not_found",
            format!("artifact-runner worker method '{path}' not found"),
        ),
    }
}

async fn run_python(payload: PythonRunPayload) -> Result<PythonRunResult, ExtensionRpcResponse> {
    let code = payload.code;
    if code.trim().is_empty() {
        return Err(ExtensionRpcResponse::failure(
            "python_code_required",
            "Python code is required",
        ));
    }
    if code.as_bytes().len() > MAX_CODE_BYTES {
        return Err(ExtensionRpcResponse::failure(
            "python_code_too_large",
            format!("Python code exceeds {MAX_CODE_BYTES} bytes"),
        ));
    }

    let temp_dir = tempdir().map_err(|error| {
        ExtensionRpcResponse::failure("python_temp_dir_failed", error.to_string())
    })?;
    let script_path = temp_dir.path().join("artifact.py");
    std::fs::write(&script_path, &code).map_err(|error| {
        ExtensionRpcResponse::failure("python_script_write_failed", error.to_string())
    })?;

    let command_name = "python";
    let args = vec![script_path.to_string_lossy().to_string()];
    let started = Instant::now();
    let mut command = Command::new(command_name);
    command
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .kill_on_drop(true);
    let output = timeout(Duration::from_millis(PYTHON_TIMEOUT_MS), command.output())
        .await
        .map_err(|_| {
            ExtensionRpcResponse::failure(
                "python_run_timeout",
                format!("Python run timed out after {PYTHON_TIMEOUT_MS}ms"),
            )
        })?
        .map_err(|error| ExtensionRpcResponse::failure("python_spawn_failed", error.to_string()))?;

    Ok(PythonRunResult {
        ok: output.status.success(),
        command: command_name.to_string(),
        args,
        cwd: temp_dir.path().to_string_lossy().to_string(),
        record_id: payload.record_id,
        conversation_id: payload.conversation_id,
        agent_id: payload.agent_id,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn parse_json<T>(value: JsonValue) -> Result<T, ExtensionRpcResponse>
where
    T: for<'de> Deserialize<'de>,
{
    if value.is_null() {
        serde_json::from_value(serde_json::json!({}))
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string()))
    } else {
        serde_json::from_value(value)
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_python_captures_stdout_and_exit_code() {
        if std::process::Command::new("python")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }

        let result = run_python(PythonRunPayload {
            code: "print('hello from artifact')".to_string(),
            record_id: Some("rec-1".to_string()),
            conversation_id: Some("conv-1".to_string()),
            agent_id: Some("agent-1".to_string()),
        })
        .await
        .expect("python run should succeed");

        assert!(result.ok);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello from artifact"));
        assert_eq!(result.record_id.as_deref(), Some("rec-1"));
    }

    #[tokio::test]
    async fn run_python_rejects_empty_code() {
        let error = run_python(PythonRunPayload {
            code: "  \n  ".to_string(),
            record_id: None,
            conversation_id: None,
            agent_id: None,
        })
        .await
        .expect_err("empty code should be rejected");

        let rpc_error = error.error.expect("rpc error");
        assert_eq!(rpc_error.code, "python_code_required");
    }
}
