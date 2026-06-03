use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::{ActivityCategory, ScreenshotMeta, VisualSummary};

const MINIMAX_PROMPT_VERSION: &str = "visual-summary-minimax-m3-v1";
const MAX_INLINE_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct VisualAnalysisInput<'a> {
    pub screenshot: &'a ScreenshotMeta,
    pub image_path: Option<&'a Path>,
}

pub trait VisualAnalyzer {
    fn analyze(
        &self,
        input: &VisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualSummary>;
}

#[derive(Debug, Clone)]
pub enum ConfiguredVisualAnalyzer {
    Local(LocalMetadataAnalyzer),
    MiniMax(MiniMaxAnalyzer),
}

impl ConfiguredVisualAnalyzer {
    pub fn from_env() -> Result<Self> {
        let provider = std::env::var("VISUAL_ANALYZER_PROVIDER").ok();
        let api_key = std::env::var("MINIMAX_API_KEY").ok();
        let base_url = std::env::var("MINIMAX_BASE_URL").ok();
        let selected_provider = select_visual_analyzer_provider(
            provider.as_deref(),
            api_key.as_deref(),
            base_url.as_deref(),
        );
        match selected_provider.to_ascii_lowercase().as_str() {
            "minimax" => Ok(Self::MiniMax(MiniMaxAnalyzer::new(
                MiniMaxConfig::from_env()?,
            ))),
            "local" | "local_stub" | "" => Ok(Self::Local(LocalMetadataAnalyzer)),
            other => bail!("unsupported VISUAL_ANALYZER_PROVIDER: {other}"),
        }
    }

    pub async fn analyze(
        &self,
        input: &VisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualSummary> {
        match self {
            Self::Local(analyzer) => analyzer.analyze(input, created_at),
            Self::MiniMax(analyzer) => analyzer.analyze(input, created_at).await,
        }
    }
}

pub fn select_visual_analyzer_provider<'a>(
    provider: Option<&'a str>,
    minimax_api_key: Option<&str>,
    minimax_base_url: Option<&str>,
) -> &'a str {
    let requested = provider.unwrap_or_default().trim();
    if !requested.is_empty() {
        return requested;
    }
    let has_minimax_credentials = minimax_api_key
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && minimax_base_url
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
    if has_minimax_credentials {
        "minimax"
    } else {
        "local"
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalMetadataAnalyzer;

impl VisualAnalyzer for LocalMetadataAnalyzer {
    fn analyze(
        &self,
        input: &VisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualSummary> {
        Ok(local_stub_visual_summary(input.screenshot, created_at))
    }
}

#[derive(Debug, Clone)]
pub struct MiniMaxConfig {
    api_key: String,
    base_url: String,
    model: String,
    image_detail: String,
    max_long_side_pixel: Option<u32>,
    max_completion_tokens: u32,
}

impl MiniMaxConfig {
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
            image_detail: "default".to_string(),
            max_long_side_pixel: None,
            max_completion_tokens: 700,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("MINIMAX_API_KEY")
            .context("MINIMAX_API_KEY is required when VISUAL_ANALYZER_PROVIDER=minimax")?;
        let base_url = std::env::var("MINIMAX_BASE_URL")
            .context("MINIMAX_BASE_URL is required when VISUAL_ANALYZER_PROVIDER=minimax")?;
        let model = std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M3".to_string());
        let mut config = Self::new(api_key, base_url, model);
        if let Ok(detail) = std::env::var("MINIMAX_IMAGE_DETAIL") {
            config.image_detail = detail;
        }
        if let Ok(value) = std::env::var("MINIMAX_MAX_LONG_SIDE_PIXEL") {
            config.max_long_side_pixel = Some(value.parse().with_context(|| {
                format!("MINIMAX_MAX_LONG_SIDE_PIXEL must be an integer, got {value}")
            })?);
        }
        if let Ok(value) = std::env::var("MINIMAX_MAX_COMPLETION_TOKENS") {
            config.max_completion_tokens = value.parse().with_context(|| {
                format!("MINIMAX_MAX_COMPLETION_TOKENS must be an integer, got {value}")
            })?;
        }
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.api_key.trim().is_empty() {
            bail!("MINIMAX_API_KEY cannot be empty");
        }
        if self.base_url.trim().is_empty() {
            bail!("MINIMAX_BASE_URL cannot be empty");
        }
        match self.image_detail.as_str() {
            "low" | "default" | "high" => {}
            _ => bail!("MINIMAX_IMAGE_DETAIL must be low, default, or high"),
        }
        Ok(())
    }

    fn chat_completions_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else {
            format!("{base}/chat/completions")
        }
    }
}

#[derive(Debug, Clone)]
pub struct MiniMaxAnalyzer {
    client: reqwest::Client,
    config: MiniMaxConfig,
}

impl MiniMaxAnalyzer {
    pub fn new(config: MiniMaxConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub fn build_chat_completions_request(
        &self,
        input: &VisualAnalysisInput<'_>,
    ) -> Result<serde_json::Value> {
        let image_path = input
            .image_path
            .ok_or_else(|| anyhow!("MiniMax visual analysis requires a screenshot image path"))?;
        let image_url = image_file_to_data_url(image_path)?;
        let mut image_url_block = serde_json::json!({
            "url": image_url,
            "detail": self.config.image_detail,
        });
        if let Some(max_long_side_pixel) = self.config.max_long_side_pixel {
            image_url_block["max_long_side_pixel"] = serde_json::json!(max_long_side_pixel);
        }

        Ok(serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You analyze screenshots for personal work insight. Return compact JSON only."
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": visual_summary_prompt(input.screenshot)
                        },
                        {
                            "type": "image_url",
                            "image_url": image_url_block
                        }
                    ]
                }
            ],
            "temperature": 0.2,
            "top_p": 0.95,
            "max_completion_tokens": self.config.max_completion_tokens,
            "thinking": { "type": "disabled" }
        }))
    }

    pub async fn analyze(
        &self,
        input: &VisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualSummary> {
        let body = self.build_chat_completions_request(input)?;
        let response = self
            .client
            .post(self.config.chat_completions_url())
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .context("MiniMax chat completions request failed")?;
        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("MiniMax response body read failed")?;
        if !status.is_success() {
            bail!("MiniMax chat completions returned {status}: {response_text}");
        }
        let content = parse_chat_completion_content(&response_text)?;
        Self::summary_from_response_text(input.screenshot, created_at, &self.config.model, &content)
    }

    pub fn summary_from_response_text(
        screenshot: &ScreenshotMeta,
        created_at: DateTime<Utc>,
        model_name: &str,
        content: &str,
    ) -> Result<VisualSummary> {
        let parsed = parse_model_summary_json(content);
        let local_fallback = local_stub_visual_summary(screenshot, created_at);
        let summary_text = parsed
            .as_ref()
            .and_then(|value| value.summary_text.clone())
            .unwrap_or_else(|| content.trim().to_string());
        let activity_category = parsed
            .as_ref()
            .and_then(|value| ActivityCategory::from_db(value.activity_category.as_deref()?))
            .unwrap_or(ActivityCategory::Unknown);

        Ok(VisualSummary {
            id: 0,
            screenshot_id: screenshot.id,
            captured_at: screenshot.captured_at,
            model_provider: "minimax".to_string(),
            model_name: model_name.to_string(),
            prompt_version: MINIMAX_PROMPT_VERSION.to_string(),
            summary_text,
            activity_category,
            project_hints: parsed
                .as_ref()
                .and_then(|value| value.project_hints.clone())
                .unwrap_or(local_fallback.project_hints),
            visible_apps: parsed
                .as_ref()
                .and_then(|value| value.visible_apps.clone())
                .unwrap_or(local_fallback.visible_apps),
            visible_text_hints: parsed
                .as_ref()
                .and_then(|value| value.visible_text_hints.clone())
                .unwrap_or_default(),
            risk_flags: parsed
                .as_ref()
                .and_then(|value| value.risk_flags.clone())
                .unwrap_or(local_fallback.risk_flags),
            confidence: parsed
                .as_ref()
                .and_then(|value| value.confidence)
                .unwrap_or(0.5)
                .clamp(0.0, 1.0),
            created_at,
            error: None,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelSummaryJson {
    summary_text: Option<String>,
    activity_category: Option<String>,
    project_hints: Option<Vec<String>>,
    visible_apps: Option<Vec<String>>,
    visible_text_hints: Option<Vec<String>>,
    risk_flags: Option<Vec<String>>,
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionMessage {
    content: String,
}

fn parse_chat_completion_content(response_text: &str) -> Result<String> {
    let response: ChatCompletionResponse =
        serde_json::from_str(response_text).context("MiniMax response was not valid JSON")?;
    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| anyhow!("MiniMax response did not include message content"))
}

fn parse_model_summary_json(content: &str) -> Option<ModelSummaryJson> {
    let trimmed = strip_json_fence(content.trim());
    serde_json::from_str(trimmed).ok()
}

fn strip_json_fence(content: &str) -> &str {
    if let Some(stripped) = content.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    if let Some(stripped) = content.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    content
}

fn image_file_to_data_url(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("screenshot image does not exist: {}", path.display()))?;
    if metadata.len() > MAX_INLINE_IMAGE_BYTES {
        bail!(
            "screenshot image is too large for inline MiniMax request: {} bytes",
            metadata.len()
        );
    }
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read image: {}", path.display()))?;
    Ok(format!(
        "data:{};base64,{}",
        mime_type_for_image_path(path),
        general_purpose::STANDARD.encode(bytes)
    ))
}

fn mime_type_for_image_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

fn visual_summary_prompt(screenshot: &ScreenshotMeta) -> String {
    format!(
        "Summarize this screenshot for personal work insight. Return JSON only with keys: summaryText, activityCategory, projectHints, visibleApps, visibleTextHints, riskFlags, confidence. activityCategory must be one of project_work, research, writing, coding, communication, meeting, admin, learning, planning, loafing, personal, idle, unknown. Context metadata: processName={:?}, windowTitle={:?}, capturedAt={}, dimensions={}x{}.",
        screenshot.process_name,
        screenshot.window_title,
        screenshot.captured_at.to_rfc3339(),
        screenshot.width,
        screenshot.height
    )
}

fn local_stub_visual_summary(
    screenshot: &ScreenshotMeta,
    created_at: DateTime<Utc>,
) -> VisualSummary {
    let app = screenshot
        .process_name
        .clone()
        .unwrap_or_else(|| "Unknown app".to_string());
    let title = screenshot
        .window_title
        .clone()
        .unwrap_or_else(|| "Untitled window".to_string());
    let activity_category =
        categorize_screenshot_metadata(&app, &title, &screenshot.capture_status);
    let visible_apps = screenshot.process_name.iter().cloned().collect::<Vec<_>>();
    let visible_text_hints = screenshot.window_title.iter().cloned().collect::<Vec<_>>();
    let project_hints = project_hints_from_metadata(&app, &title);
    let mut risk_flags = Vec::new();
    if screenshot.capture_status != "ok" {
        risk_flags.push(format!("capture_status:{}", screenshot.capture_status));
    }
    if screenshot.width == 0 || screenshot.height == 0 {
        risk_flags.push("empty_dimensions".to_string());
    }

    let summary_text = if screenshot.capture_status == "ok" {
        format!(
            "Metadata-only local summary: {app} appears focused on {title} at {}x{}.",
            screenshot.width, screenshot.height
        )
    } else {
        format!(
            "Metadata-only local summary: screenshot was not visually analyzed because capture status is {}.",
            screenshot.capture_status
        )
    };

    VisualSummary {
        id: 0,
        screenshot_id: screenshot.id,
        captured_at: screenshot.captured_at,
        model_provider: "local_stub".to_string(),
        model_name: "metadata-v1".to_string(),
        prompt_version: "visual-summary-v1".to_string(),
        summary_text,
        activity_category,
        project_hints,
        visible_apps,
        visible_text_hints,
        risk_flags,
        confidence: 0.35,
        created_at,
        error: None,
    }
}

fn categorize_screenshot_metadata(
    app: &str,
    title: &str,
    capture_status: &str,
) -> ActivityCategory {
    if capture_status != "ok" {
        return ActivityCategory::Unknown;
    }
    let combined = format!(
        "{} {}",
        app.to_ascii_lowercase(),
        title.to_ascii_lowercase()
    );
    if contains_any(&combined, &["code", "cursor", "cargo", "rust", "tsr"]) {
        ActivityCategory::Coding
    } else if contains_any(&combined, &["word", "docx", "writing"]) {
        ActivityCategory::Writing
    } else if contains_any(&combined, &["wechat", "weixin", "mail", "outlook"]) {
        ActivityCategory::Communication
    } else if contains_any(&combined, &["chrome", "msedge", "edge", "browser"]) {
        ActivityCategory::Research
    } else {
        ActivityCategory::Unknown
    }
}

fn project_hints_from_metadata(app: &str, title: &str) -> Vec<String> {
    let combined = format!(
        "{} {}",
        app.to_ascii_lowercase(),
        title.to_ascii_lowercase()
    );
    if contains_any(
        &combined,
        &["time state", "time-state", "tsr", "activity review"],
    ) {
        vec!["Time State Recorder".to_string()]
    } else {
        Vec::new()
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
