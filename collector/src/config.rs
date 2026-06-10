use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub storage: StorageConfig,
    pub runtime: RuntimeConfig,
    pub capture: CaptureConfig,
    pub visual: VisualConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    pub database_path: PathBuf,
    pub screenshot_dir: PathBuf,
    pub high_res_screenshot_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub api_addr: String,
    pub poll_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureConfig {
    pub screenshot_interval_secs: u64,
    pub high_res_screenshot_interval_secs: u64,
    pub idle_threshold_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub image_detail: String,
    pub max_completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicAppConfigResponse {
    pub config: PublicAppConfig,
    pub restart_required: bool,
    pub restart_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicAppConfig {
    pub storage: StorageConfig,
    pub runtime: RuntimeConfig,
    pub capture: CaptureConfig,
    pub visual: PublicVisualConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicVisualConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub api_key_masked: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub image_detail: String,
    pub max_completion_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigPatch {
    pub storage: Option<StorageConfigPatch>,
    pub runtime: Option<RuntimeConfigPatch>,
    pub capture: Option<CaptureConfigPatch>,
    pub visual: Option<VisualConfigPatch>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfigPatch {
    pub database_path: Option<PathBuf>,
    pub screenshot_dir: Option<PathBuf>,
    pub high_res_screenshot_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigPatch {
    pub api_addr: Option<String>,
    pub poll_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureConfigPatch {
    pub screenshot_interval_secs: Option<u64>,
    pub high_res_screenshot_interval_secs: Option<u64>,
    pub idle_threshold_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualConfigPatch {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub image_detail: Option<String>,
    pub max_completion_tokens: Option<u32>,
}

impl AppConfig {
    pub fn default_for_paths(
        database_path: PathBuf,
        screenshot_dir: PathBuf,
        high_res_screenshot_dir: PathBuf,
    ) -> Self {
        Self {
            storage: StorageConfig {
                database_path,
                screenshot_dir,
                high_res_screenshot_dir,
            },
            runtime: RuntimeConfig {
                api_addr: "127.0.0.1:4317".into(),
                poll_ms: 1000,
            },
            capture: CaptureConfig {
                screenshot_interval_secs: 60,
                high_res_screenshot_interval_secs: 60,
                idle_threshold_secs: 120,
            },
            visual: VisualConfig {
                provider: "local".into(),
                api_key: None,
                base_url: None,
                model: "MiniMax-M3".into(),
                image_detail: "high".into(),
                max_completion_tokens: 200_000,
            },
        }
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read app config {}", path.display()))?;
        let config = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse app config {}", path.display()))?;
        Ok(config)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)
            .with_context(|| format!("failed to write app config {}", path.display()))?;
        Ok(())
    }

    pub fn to_public_response(
        &self,
        restart_required: bool,
        restart_reasons: Vec<String>,
    ) -> PublicAppConfigResponse {
        PublicAppConfigResponse {
            config: PublicAppConfig {
                storage: self.storage.clone(),
                runtime: self.runtime.clone(),
                capture: self.capture.clone(),
                visual: PublicVisualConfig {
                    provider: self.visual.provider.clone(),
                    api_key: None,
                    api_key_masked: self.visual.api_key.as_deref().map(mask_secret),
                    base_url: self.visual.base_url.clone(),
                    model: self.visual.model.clone(),
                    image_detail: self.visual.image_detail.clone(),
                    max_completion_tokens: self.visual.max_completion_tokens,
                },
            },
            restart_required,
            restart_reasons,
        }
    }

    pub fn apply_patch(&mut self, patch: AppConfigPatch) -> Result<Vec<String>> {
        let mut restart_reasons = Vec::new();
        if let Some(storage) = patch.storage {
            if let Some(value) = storage.database_path {
                ensure_non_empty_path(&value, "databasePath")?;
                if self.storage.database_path != value {
                    self.storage.database_path = value;
                    restart_reasons.push("database_path".into());
                }
            }
            if let Some(value) = storage.screenshot_dir {
                ensure_non_empty_path(&value, "screenshotDir")?;
                if self.storage.screenshot_dir != value {
                    self.storage.screenshot_dir = value;
                    restart_reasons.push("screenshot_dir".into());
                }
            }
            if let Some(value) = storage.high_res_screenshot_dir {
                ensure_non_empty_path(&value, "highResScreenshotDir")?;
                if self.storage.high_res_screenshot_dir != value {
                    self.storage.high_res_screenshot_dir = value;
                    restart_reasons.push("high_res_screenshot_dir".into());
                }
            }
        }
        if let Some(runtime) = patch.runtime {
            if let Some(value) = runtime.api_addr {
                if value.trim().is_empty() {
                    bail!("apiAddr cannot be empty");
                }
                if self.runtime.api_addr != value {
                    self.runtime.api_addr = value;
                    restart_reasons.push("api_addr".into());
                }
            }
            if let Some(value) = runtime.poll_ms {
                if value < 100 {
                    bail!("pollMs must be at least 100");
                }
                self.runtime.poll_ms = value;
            }
        }
        if let Some(capture) = patch.capture {
            if let Some(value) = capture.screenshot_interval_secs {
                if value == 0 {
                    bail!("screenshotIntervalSecs must be greater than 0");
                }
                self.capture.screenshot_interval_secs = value;
            }
            if let Some(value) = capture.high_res_screenshot_interval_secs {
                if value == 0 {
                    bail!("highResScreenshotIntervalSecs must be greater than 0");
                }
                self.capture.high_res_screenshot_interval_secs = value;
            }
            if let Some(value) = capture.idle_threshold_secs {
                if value == 0 {
                    bail!("idleThresholdSecs must be greater than 0");
                }
                self.capture.idle_threshold_secs = value;
            }
        }
        if let Some(visual) = patch.visual {
            if let Some(value) = visual.provider {
                if !matches!(value.as_str(), "local" | "minimax") {
                    bail!("provider must be local or minimax");
                }
                self.visual.provider = value;
            }
            if let Some(value) = visual.api_key {
                self.visual.api_key = non_empty_string(value);
            }
            if let Some(value) = visual.base_url {
                self.visual.base_url = non_empty_string(value);
            }
            if let Some(value) = visual.model {
                if value.trim().is_empty() {
                    bail!("model cannot be empty");
                }
                self.visual.model = value;
            }
            if let Some(value) = visual.image_detail {
                if !matches!(value.as_str(), "low" | "default" | "high") {
                    bail!("imageDetail must be low, default, or high");
                }
                self.visual.image_detail = value;
            }
            if let Some(value) = visual.max_completion_tokens {
                if value == 0 {
                    bail!("maxCompletionTokens must be greater than 0");
                }
                self.visual.max_completion_tokens = value;
            }
        }
        Ok(restart_reasons)
    }
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 4 {
        return "********".into();
    }
    let suffix = &value[value.len() - 4..];
    format!("********{suffix}")
}

fn ensure_non_empty_path(value: &Path, field: &str) -> Result<()> {
    if value.as_os_str().is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
