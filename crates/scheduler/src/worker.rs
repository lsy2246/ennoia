use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use ennoia_kernel::{JobHandler, SchedulerStore};
use tokio::sync::watch;

/// Worker polls the scheduler store and runs due jobs through registered handlers.
#[derive(Clone)]
pub struct Worker {
    store: Arc<dyn SchedulerStore>,
    handlers: Arc<HashMap<String, Arc<dyn JobHandler>>>,
    tick_ms: u64,
    batch_size: u32,
}

impl Worker {
    pub fn new(store: Arc<dyn SchedulerStore>, tick_ms: u64) -> Self {
        Self {
            store,
            handlers: Arc::new(HashMap::new()),
            tick_ms,
            batch_size: 8,
        }
    }

    pub fn with_handlers(mut self, handlers: Vec<Arc<dyn JobHandler>>) -> Self {
        let mut map = HashMap::new();
        for handler in handlers {
            map.insert(handler.kind().to_string(), handler);
        }
        self.handlers = Arc::new(map);
        self
    }

    pub fn batch_size(mut self, n: u32) -> Self {
        self.batch_size = n;
        self
    }

    /// run_forever loops until the cancel signal fires.
    pub async fn run_forever(self, mut cancel: watch::Receiver<bool>) {
        let interval = Duration::from_millis(self.tick_ms.max(100));
        loop {
            tokio::select! {
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    let _ = self.tick().await;
                }
            }
        }
    }

    /// tick fetches due jobs once and runs them serially.
    pub async fn tick(&self) -> Result<u32, String> {
        let now = Utc::now().to_rfc3339();
        let due = self
            .store
            .fetch_due(&now, self.batch_size)
            .await
            .map_err(|e| e.to_string())?;

        let mut processed = 0u32;
        for job in due {
            let now = Utc::now().to_rfc3339();
            if let Err(e) = self.store.mark_running(&job.id, &now).await {
                return Err(e.to_string());
            }

            let kind_key = job.job_kind.as_str().to_string();
            let outcome = match self.handlers.get(&kind_key) {
                Some(handler) => handler.handle(&job).await,
                None => Err(format!("no handler registered for kind '{kind_key}'")),
            };

            let now = Utc::now().to_rfc3339();
            match outcome {
                Ok(()) => {
                    let _ = self.store.mark_done(&job.id, &now).await;
                }
                Err(error) => {
                    let _ = self.store.mark_failed(&job.id, &now, &error).await;
                }
            }
            processed += 1;
        }

        Ok(processed)
    }
}
