use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};

use ennoia_kernel::{
    ExtensionHostCapabilityRequest, ExtensionRpcResponse, ProcessWorkerControlMessage,
};
use tokio::sync::oneshot;
use uuid::Uuid;

static HOST_BRIDGE: OnceLock<Arc<HostBridge>> = OnceLock::new();

#[derive(Debug)]
pub struct HostBridge {
    stdout: Mutex<io::Stdout>,
    pending: Mutex<HashMap<String, oneshot::Sender<ExtensionRpcResponse>>>,
}

impl HostBridge {
    pub fn install() -> Arc<Self> {
        HOST_BRIDGE
            .get_or_init(|| {
                Arc::new(Self {
                    stdout: Mutex::new(io::stdout()),
                    pending: Mutex::new(HashMap::new()),
                })
            })
            .clone()
    }

    pub fn global() -> Result<Arc<Self>, String> {
        HOST_BRIDGE
            .get()
            .cloned()
            .ok_or_else(|| "host bridge is not initialized".to_string())
    }

    pub async fn call(
        &self,
        request: ExtensionHostCapabilityRequest,
    ) -> Result<ExtensionRpcResponse, String> {
        let call_id = format!("hostcall-{}", Uuid::new_v4());
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .map_err(|_| "host bridge pending map poisoned".to_string())?
            .insert(call_id.clone(), tx);
        if let Err(error) = self.write_control_message(&ProcessWorkerControlMessage::HostCall {
            call_id: call_id.clone(),
            request,
        }) {
            let _ = self
                .pending
                .lock()
                .map(|mut pending| pending.remove(&call_id));
            return Err(format!("write host capability call failed: {error}"));
        }
        rx.await
            .map_err(|_| "host capability response channel closed".to_string())
    }

    pub fn try_resolve_control_line(&self, line: &str) -> io::Result<bool> {
        let Some(message) = parse_control_message(line)? else {
            return Ok(false);
        };
        match message {
            ProcessWorkerControlMessage::HostResult { call_id, response } => {
                if let Some(sender) = self
                    .pending
                    .lock()
                    .map_err(|_| io::Error::other("host bridge pending map poisoned"))?
                    .remove(&call_id)
                {
                    let _ = sender.send(response);
                }
                Ok(true)
            }
            ProcessWorkerControlMessage::HostCall { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "worker received unexpected host call control message",
            )),
        }
    }

    pub fn write_rpc_response(&self, response: &ExtensionRpcResponse) -> io::Result<()> {
        self.write_json_line(response)
    }

    fn write_control_message(&self, message: &ProcessWorkerControlMessage) -> io::Result<()> {
        self.write_json_line(message)
    }

    fn write_json_line<T: serde::Serialize>(&self, value: &T) -> io::Result<()> {
        let mut stdout = self
            .stdout
            .lock()
            .map_err(|_| io::Error::other("host bridge stdout poisoned"))?;
        serde_json::to_writer(&mut *stdout, value).map_err(io::Error::other)?;
        stdout.write_all(b"\n")?;
        stdout.flush()
    }
}

fn parse_control_message(line: &str) -> io::Result<Option<ProcessWorkerControlMessage>> {
    let value = match serde_json::from_str::<serde_json::Value>(line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    if !matches!(kind, "host_call" | "host_result") {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(io::Error::other)
}
