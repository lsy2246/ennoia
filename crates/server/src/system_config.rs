//! Hot-reload runtime container for every system_config slot.
//!
//! Each sub-config is wrapped in `Arc<ArcSwap<...>>` so middleware reads the
//! latest value on every request without locking.

use std::sync::Arc;

use arc_swap::ArcSwap;
use ennoia_config::SqliteConfigStore;
use ennoia_kernel::{
    AuthConfig, BodyLimitConfig, BootstrapState, ConfigStore, CorsConfig, LoggingConfig,
    RateLimitConfig, SystemConfig, TimeoutConfig, CONFIG_KEY_AUTH, CONFIG_KEY_BODY_LIMIT,
    CONFIG_KEY_BOOTSTRAP, CONFIG_KEY_CORS, CONFIG_KEY_LOGGING, CONFIG_KEY_RATE_LIMIT,
    CONFIG_KEY_TIMEOUT,
};

use crate::middleware::RateLimitState;

/// SystemConfigRuntime holds the live, atomic-swap view of every config slot.
#[derive(Clone)]
pub struct SystemConfigRuntime {
    pub store: Arc<SqliteConfigStore>,
    pub auth: Arc<ArcSwap<AuthConfig>>,
    pub rate_limit: Arc<ArcSwap<RateLimitConfig>>,
    pub cors: Arc<ArcSwap<CorsConfig>>,
    pub timeout: Arc<ArcSwap<TimeoutConfig>>,
    pub logging: Arc<ArcSwap<LoggingConfig>>,
    pub body_limit: Arc<ArcSwap<BodyLimitConfig>>,
    pub bootstrap: Arc<ArcSwap<BootstrapState>>,
    pub rate_limit_state: RateLimitState,
}

impl SystemConfigRuntime {
    pub fn defaulted(store: Arc<SqliteConfigStore>) -> Self {
        Self {
            store,
            auth: Arc::new(ArcSwap::from_pointee(AuthConfig::default())),
            rate_limit: Arc::new(ArcSwap::from_pointee(RateLimitConfig::default())),
            cors: Arc::new(ArcSwap::from_pointee(CorsConfig::default())),
            timeout: Arc::new(ArcSwap::from_pointee(TimeoutConfig::default())),
            logging: Arc::new(ArcSwap::from_pointee(LoggingConfig::default())),
            body_limit: Arc::new(ArcSwap::from_pointee(BodyLimitConfig::default())),
            bootstrap: Arc::new(ArcSwap::from_pointee(BootstrapState::default())),
            rate_limit_state: RateLimitState::new(),
        }
    }

    /// Loads every key from the store; missing keys are populated with defaults
    /// (and persisted so subsequent starts are consistent).
    pub async fn load_from_store(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.hydrate_slot(CONFIG_KEY_AUTH, &self.auth, AuthConfig::default())
            .await?;
        self.hydrate_slot(
            CONFIG_KEY_RATE_LIMIT,
            &self.rate_limit,
            RateLimitConfig::default(),
        )
        .await?;
        self.hydrate_slot(CONFIG_KEY_CORS, &self.cors, CorsConfig::default())
            .await?;
        self.hydrate_slot(CONFIG_KEY_TIMEOUT, &self.timeout, TimeoutConfig::default())
            .await?;
        self.hydrate_slot(CONFIG_KEY_LOGGING, &self.logging, LoggingConfig::default())
            .await?;
        self.hydrate_slot(
            CONFIG_KEY_BODY_LIMIT,
            &self.body_limit,
            BodyLimitConfig::default(),
        )
        .await?;
        self.hydrate_slot(
            CONFIG_KEY_BOOTSTRAP,
            &self.bootstrap,
            BootstrapState::default(),
        )
        .await?;
        Ok(())
    }

    async fn hydrate_slot<T>(
        &self,
        key: &str,
        slot: &Arc<ArcSwap<T>>,
        default_value: T,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        match self.store.get(key).await? {
            Some(entry) => {
                let parsed: T = serde_json::from_str(&entry.payload_json)?;
                slot.store(Arc::new(parsed));
            }
            None => {
                slot.store(Arc::new(default_value.clone()));
                let payload = serde_json::to_value(&default_value)?;
                self.store.put(key, &payload, Some("bootstrap")).await?;
            }
        }
        Ok(())
    }

    /// Build a full `SystemConfig` snapshot from the current live values.
    pub fn snapshot(&self) -> SystemConfig {
        SystemConfig {
            auth: (**self.auth.load()).clone(),
            rate_limit: (**self.rate_limit.load()).clone(),
            cors: (**self.cors.load()).clone(),
            timeout: (**self.timeout.load()).clone(),
            logging: (**self.logging.load()).clone(),
            body_limit: (**self.body_limit.load()).clone(),
            bootstrap: (**self.bootstrap.load()).clone(),
        }
    }

    /// Apply an already-parsed value to the matching slot. Returns false if `key` unknown.
    pub fn apply(&self, key: &str, payload: &serde_json::Value) -> Result<bool, serde_json::Error> {
        match key {
            CONFIG_KEY_AUTH => {
                let v: AuthConfig = serde_json::from_value(payload.clone())?;
                self.auth.store(Arc::new(v));
            }
            CONFIG_KEY_RATE_LIMIT => {
                let v: RateLimitConfig = serde_json::from_value(payload.clone())?;
                self.rate_limit.store(Arc::new(v));
            }
            CONFIG_KEY_CORS => {
                let v: CorsConfig = serde_json::from_value(payload.clone())?;
                self.cors.store(Arc::new(v));
            }
            CONFIG_KEY_TIMEOUT => {
                let v: TimeoutConfig = serde_json::from_value(payload.clone())?;
                self.timeout.store(Arc::new(v));
            }
            CONFIG_KEY_LOGGING => {
                let v: LoggingConfig = serde_json::from_value(payload.clone())?;
                self.logging.store(Arc::new(v));
            }
            CONFIG_KEY_BODY_LIMIT => {
                let v: BodyLimitConfig = serde_json::from_value(payload.clone())?;
                self.body_limit.store(Arc::new(v));
            }
            CONFIG_KEY_BOOTSTRAP => {
                let v: BootstrapState = serde_json::from_value(payload.clone())?;
                self.bootstrap.store(Arc::new(v));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
