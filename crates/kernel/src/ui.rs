use serde::{Deserialize, Serialize};

/// LocalizedText is the stable protocol for any UI-facing label/title.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalizedText {
    pub key: String,
    pub fallback: String,
}

impl LocalizedText {
    pub fn new(key: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            fallback: fallback.into(),
        }
    }
}

/// ThemeAppearance expresses the intended visual category of a theme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeAppearance {
    Light,
    Dark,
    System,
    HighContrast,
}

impl ThemeAppearance {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeAppearance::Light => "light",
            ThemeAppearance::Dark => "dark",
            ThemeAppearance::System => "system",
            ThemeAppearance::HighContrast => "high-contrast",
        }
    }
}

/// UiPreference stores a resolved preference override for one scope.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiPreference {
    pub locale: Option<String>,
    pub theme_id: Option<String>,
    pub time_zone: Option<String>,
    pub date_style: Option<String>,
    pub density: Option<String>,
    pub motion: Option<String>,
    pub version: u64,
    pub updated_at: String,
}

/// UiPreferenceRecord binds one preference payload to a stable subject id.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiPreferenceRecord {
    pub subject_id: String,
    pub preference: UiPreference,
}
