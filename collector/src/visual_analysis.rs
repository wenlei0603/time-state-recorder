use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{
    ActivityCategory, HighResScreenshotMeta, ScreenshotMeta, VisualSummary, VisualTrajectoryPoint,
    VisualWindowSummary,
};
use crate::prompt_time::minimax_prompt_timestamp;
use crate::visual_labels::{
    fallback_identity_tags, fallback_routine_tags, identity_tag_list_for_prompt,
    routine_tag_list_for_prompt, sanitize_identity_tags, sanitize_routine_tags,
};

const MINIMAX_PROMPT_VERSION: &str = "visual-summary-minimax-m3-v1";
const MINIMAX_WINDOW_PROMPT_VERSION: &str = "visual-window-minimax-m3-v1";
const WINDOW_SAMPLE_MARKS: [u8; 3] = [1, 3, 5];
const MAX_INLINE_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct VisualAnalysisInput<'a> {
    pub screenshot: &'a ScreenshotMeta,
    pub image_path: Option<&'a Path>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowScreenshotSample {
    pub minute_mark: u8,
    pub screenshot: HighResScreenshotMeta,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowVisualAnalysisSample<'a> {
    pub minute_mark: u8,
    pub screenshot: &'a HighResScreenshotMeta,
    pub image_path: &'a Path,
}

#[derive(Debug, Clone)]
pub struct WindowVisualAnalysisInput<'a> {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub samples: Vec<WindowVisualAnalysisSample<'a>>,
    pub previous_summary: Option<&'a VisualWindowSummary>,
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

    pub async fn analyze_window(
        &self,
        input: &WindowVisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualWindowSummary> {
        match self {
            Self::Local(_) => Ok(local_stub_visual_window_summary(
                input.window_start,
                input.window_end,
                &owned_window_samples(input),
                input.previous_summary.map(|summary| summary.id),
                created_at,
            )),
            Self::MiniMax(analyzer) => analyzer.analyze_window(input, created_at).await,
        }
    }
}

pub fn select_window_samples(
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    screenshots: &[HighResScreenshotMeta],
) -> Result<Vec<WindowScreenshotSample>> {
    let mut selected = Vec::new();
    for minute_mark in WINDOW_SAMPLE_MARKS {
        let slot_start = window_start + Duration::minutes(i64::from(minute_mark) - 1);
        let slot_end = std::cmp::min(slot_start + Duration::minutes(1), window_end);
        let screenshot = screenshots
            .iter()
            .find(|screenshot| {
                screenshot.capture_status == "ok"
                    && !screenshot.file_path.is_empty()
                    && screenshot.captured_at >= slot_start
                    && screenshot.captured_at < slot_end
                    && !selected.iter().any(|sample: &WindowScreenshotSample| {
                        sample.screenshot.id == screenshot.id
                    })
            })
            .cloned()
            .with_context(|| {
                format!(
                    "missing high-res screenshot for minute {minute_mark} in {}..{}",
                    window_start.to_rfc3339(),
                    window_end.to_rfc3339()
                )
            })?;
        selected.push(WindowScreenshotSample {
            minute_mark,
            screenshot,
        });
    }
    Ok(selected)
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
            max_completion_tokens: 10_000,
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
        if let Ok(value) = std::env::var("MINIMAX_VISUAL_MAX_COMPLETION_TOKENS")
            .or_else(|_| std::env::var("MINIMAX_MAX_COMPLETION_TOKENS"))
        {
            config.max_completion_tokens = value.parse().with_context(|| {
                format!("MINIMAX_VISUAL_MAX_COMPLETION_TOKENS must be an integer, got {value}")
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

        let mut body = serde_json::json!({
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
        });
        if let Some(response_format) =
            crate::llm_json::minimax_json_response_format(&self.config.model)
        {
            body["response_format"] = response_format;
        }
        Ok(body)
    }

    pub fn build_window_chat_completions_request(
        &self,
        input: &WindowVisualAnalysisInput<'_>,
    ) -> Result<serde_json::Value> {
        ensure_window_sample_marks(&input.samples)?;
        let mut content = vec![serde_json::json!({
            "type": "text",
            "text": window_summary_prompt(input),
        })];

        for sample in &input.samples {
            let image_url = image_file_to_data_url(sample.image_path)?;
            let mut image_url_block = serde_json::json!({
                "url": image_url,
                "detail": self.config.image_detail,
            });
            if let Some(max_long_side_pixel) = self.config.max_long_side_pixel {
                image_url_block["max_long_side_pixel"] = serde_json::json!(max_long_side_pixel);
            }
            content.push(serde_json::json!({
                "type": "image_url",
                "image_url": image_url_block
            }));
        }

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "Return one valid JSON object only. Do not use markdown fences or commentary. Use only the provided label values."
                },
                {
                    "role": "user",
                    "content": content
                }
            ],
            "temperature": 0.2,
            "top_p": 0.95,
            "max_completion_tokens": self.config.max_completion_tokens,
            "thinking": { "type": "disabled" }
        });
        if let Some(response_format) =
            crate::llm_json::minimax_json_response_format(&self.config.model)
        {
            body["response_format"] = response_format;
        }
        Ok(body)
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

    pub async fn analyze_window(
        &self,
        input: &WindowVisualAnalysisInput<'_>,
        created_at: DateTime<Utc>,
    ) -> Result<VisualWindowSummary> {
        let body = self.build_window_chat_completions_request(input)?;
        let response = self
            .client
            .post(self.config.chat_completions_url())
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .context("MiniMax window visual analysis request failed")?;
        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("MiniMax window visual response body read failed")?;
        if !status.is_success() {
            bail!("MiniMax window visual analysis returned {status}: {response_text}");
        }
        let content = parse_chat_completion_content(&response_text)?;
        Self::window_summary_from_response_text(
            input.window_start,
            input.window_end,
            &owned_window_samples(input),
            input.previous_summary.map(|summary| summary.id),
            created_at,
            &self.config.model,
            &content,
        )
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
            identity_tags: parsed
                .as_ref()
                .and_then(|value| value.identity_tags.as_ref())
                .map(|tags| sanitize_identity_tags(tags))
                .unwrap_or(local_fallback.identity_tags),
            routine_tags: parsed
                .as_ref()
                .and_then(|value| value.routine_tags.as_ref())
                .map(|tags| sanitize_routine_tags(tags))
                .unwrap_or(local_fallback.routine_tags),
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

    pub fn window_summary_from_response_text(
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
        samples: &[WindowScreenshotSample],
        previous_summary_id: Option<i64>,
        created_at: DateTime<Utc>,
        model_name: &str,
        content: &str,
    ) -> Result<VisualWindowSummary> {
        let raw_summary_json = parse_model_window_summary_value(content)
            .unwrap_or_else(|| serde_json::json!({ "content": content.trim() }));
        let parsed =
            serde_json::from_value::<ModelWindowSummaryJson>(raw_summary_json.clone()).ok();
        let local = local_stub_visual_window_summary(
            window_start,
            window_end,
            samples,
            previous_summary_id,
            created_at,
        );

        let primary_activity = parsed
            .as_ref()
            .and_then(|value| ActivityCategory::from_db(value.primary_activity.as_deref()?))
            .unwrap_or_else(|| local.primary_activity.clone());
        let trajectory = parsed
            .as_ref()
            .and_then(|value| value.trajectory.clone())
            .map(|items| model_trajectory_to_points(&items, samples))
            .filter(|items| !items.is_empty())
            .unwrap_or(local.trajectory);

        Ok(VisualWindowSummary {
            id: 0,
            window_start,
            window_end,
            sampled_screenshot_ids: samples
                .iter()
                .map(|sample| sample.screenshot.id)
                .collect::<Vec<_>>(),
            previous_summary_id,
            model_provider: "minimax".to_string(),
            model_name: model_name.to_string(),
            prompt_version: MINIMAX_WINDOW_PROMPT_VERSION.to_string(),
            summary_text: parsed
                .as_ref()
                .and_then(|value| value.summary_text.clone())
                .unwrap_or_else(|| content.trim().to_string()),
            continuity: parsed
                .as_ref()
                .and_then(|value| value.continuity.clone())
                .unwrap_or_else(|| local.continuity.clone()),
            primary_activity,
            project_hints: parsed
                .as_ref()
                .and_then(|value| value.project_hints.clone())
                .unwrap_or(local.project_hints),
            identity_tags: parsed
                .as_ref()
                .and_then(|value| value.identity_tags.as_ref())
                .map(|tags| sanitize_identity_tags(tags))
                .unwrap_or(local.identity_tags),
            routine_tags: parsed
                .as_ref()
                .and_then(|value| value.routine_tags.as_ref())
                .map(|tags| sanitize_routine_tags(tags))
                .unwrap_or(local.routine_tags),
            task_intent: parsed
                .as_ref()
                .and_then(|value| value.task_intent.clone())
                .unwrap_or(local.task_intent),
            trajectory,
            switching_level: parsed
                .as_ref()
                .and_then(|value| value.switching_level.clone())
                .unwrap_or(local.switching_level),
            switching_evidence: parsed
                .as_ref()
                .and_then(|value| value.switching_evidence.clone())
                .unwrap_or(local.switching_evidence),
            loafing_level: parsed
                .as_ref()
                .and_then(|value| value.loafing_level.clone())
                .unwrap_or(local.loafing_level),
            loafing_evidence: parsed
                .as_ref()
                .and_then(|value| value.loafing_evidence.clone())
                .unwrap_or(local.loafing_evidence),
            visible_apps: parsed
                .as_ref()
                .and_then(|value| value.visible_apps.clone())
                .unwrap_or(local.visible_apps),
            visible_text_hints: parsed
                .as_ref()
                .and_then(|value| value.visible_text_hints.clone())
                .unwrap_or(local.visible_text_hints),
            risk_flags: parsed
                .as_ref()
                .and_then(|value| value.risk_flags.clone())
                .unwrap_or(local.risk_flags),
            confidence: parsed
                .as_ref()
                .and_then(|value| value.confidence)
                .unwrap_or(0.5)
                .clamp(0.0, 1.0),
            raw_summary_json,
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
    identity_tags: Option<Vec<String>>,
    routine_tags: Option<Vec<String>>,
    visible_apps: Option<Vec<String>>,
    visible_text_hints: Option<Vec<String>>,
    risk_flags: Option<Vec<String>>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelWindowSummaryJson {
    summary_text: Option<String>,
    continuity: Option<String>,
    primary_activity: Option<String>,
    project_hints: Option<Vec<String>>,
    identity_tags: Option<Vec<String>>,
    routine_tags: Option<Vec<String>>,
    task_intent: Option<String>,
    trajectory: Option<Vec<ModelTrajectoryPointJson>>,
    switching_level: Option<String>,
    switching_evidence: Option<String>,
    loafing_level: Option<String>,
    loafing_evidence: Option<String>,
    visible_apps: Option<Vec<String>>,
    visible_text_hints: Option<Vec<String>>,
    risk_flags: Option<Vec<String>>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelTrajectoryPointJson {
    minute_mark: u8,
    observation: String,
    activity_category: Option<String>,
    project_hints: Option<Vec<String>>,
    identity_tags: Option<Vec<String>>,
    routine_tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
    #[serde(default)]
    finish_reason: Option<String>,
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
        .and_then(|choice| {
            if choice
                .finish_reason
                .as_deref()
                .is_some_and(|reason| reason.eq_ignore_ascii_case("length"))
            {
                None
            } else {
                Some(choice.message.content)
            }
        })
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "MiniMax response did not include complete message content; finish_reason may be length"
            )
        })
}

fn parse_model_summary_json(content: &str) -> Option<ModelSummaryJson> {
    crate::llm_json::parse_json_object_as(content)
}

fn parse_model_window_summary_value(content: &str) -> Option<serde_json::Value> {
    crate::llm_json::parse_json_object(content)
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
        "Summarize this screenshot for personal work insight. Return JSON only with keys: summaryText, activityCategory, projectHints, identityTags, routineTags, visibleApps, visibleTextHints, riskFlags, confidence. activityCategory must be one of project_work, research, writing, coding, communication, meeting, admin, learning, planning, loafing, personal, idle, unknown. identityTags must use only these values: {}. routineTags must use only these values: {}. Context metadata: processName={:?}, windowTitle={:?}, capturedAt={}, dimensions={}x{}.",
        identity_tag_list_for_prompt(),
        routine_tag_list_for_prompt(),
        screenshot.process_name,
        screenshot.window_title,
        minimax_prompt_timestamp(screenshot.captured_at),
        screenshot.width,
        screenshot.height
    )
}

fn window_summary_prompt(input: &WindowVisualAnalysisInput<'_>) -> String {
    let samples_json = input
        .samples
        .iter()
        .map(|sample| {
            serde_json::json!({
                "minuteMark": sample.minute_mark,
                "highResScreenshotId": sample.screenshot.id,
                "capturedAt": minimax_prompt_timestamp(sample.screenshot.captured_at),
                "processName": sample.screenshot.process_name,
                "windowTitle": sample.screenshot.window_title,
                "dimensions": format!("{}x{}", sample.screenshot.width, sample.screenshot.height)
            })
        })
        .collect::<Vec<_>>();
    let previous_summary = input.previous_summary.map(|summary| {
        serde_json::json!({
            "id": summary.id,
            "windowStart": minimax_prompt_timestamp(summary.window_start),
            "windowEnd": minimax_prompt_timestamp(summary.window_end),
            "summaryText": summary.summary_text,
            "primaryActivity": summary.primary_activity.as_str(),
            "projectHints": summary.project_hints,
            "identityTags": summary.identity_tags,
            "routineTags": summary.routine_tags,
            "taskIntent": summary.task_intent,
            "switchingLevel": summary.switching_level,
            "loafingLevel": summary.loafing_level,
        })
    });

    format!(
        "Analyze this 5-minute work window using exactly three screenshots from minute marks 1, 3, and 5. Use the previous window summary only as continuity context, not as evidence for the current window. Return JSON only with keys: summaryText, continuity, primaryActivity, projectHints, identityTags, routineTags, taskIntent, trajectory, switchingLevel, switchingEvidence, loafingLevel, loafingEvidence, visibleApps, visibleTextHints, riskFlags, confidence. primaryActivity and each trajectory.activityCategory must be one of project_work, research, writing, coding, communication, meeting, admin, learning, planning, loafing, personal, idle, unknown. identityTags and each trajectory.identityTags must use only these values: {}. routineTags and each trajectory.routineTags must use only these values: {}. trajectory must include one object per image with minuteMark, observation, activityCategory, projectHints, identityTags, routineTags. switchingLevel must be low, medium, or high. loafingLevel must be none, possible, or clear. Human-facing strings including summaryText, continuity, taskIntent, trajectory.observation, switchingEvidence, loafingEvidence must be concise Chinese. windowStart={}, windowEnd={}, previousWindowSummary={}, samples={}",
        identity_tag_list_for_prompt(),
        routine_tag_list_for_prompt(),
        minimax_prompt_timestamp(input.window_start),
        minimax_prompt_timestamp(input.window_end),
        previous_summary
            .map(|value| serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "null".to_string()),
        serde_json::to_string(&samples_json).unwrap_or_else(|_| "[]".to_string())
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
    let identity_tags = fallback_identity_tags(&app, &title);
    let routine_tags = fallback_routine_tags(&app, &title);
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
        identity_tags,
        routine_tags,
        visible_apps,
        visible_text_hints,
        risk_flags,
        confidence: 0.35,
        created_at,
        error: None,
    }
}

fn local_stub_visual_window_summary(
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    samples: &[WindowScreenshotSample],
    previous_summary_id: Option<i64>,
    created_at: DateTime<Utc>,
) -> VisualWindowSummary {
    let trajectory = samples
        .iter()
        .map(|sample| {
            let app = sample
                .screenshot
                .process_name
                .clone()
                .unwrap_or_else(|| "Unknown app".to_string());
            let title = sample
                .screenshot
                .window_title
                .clone()
                .unwrap_or_else(|| "Untitled window".to_string());
            let point_project_hints = project_hints_from_metadata(&app, &title);
            VisualTrajectoryPoint {
                minute_mark: sample.minute_mark,
                screenshot_id: sample.screenshot.id,
                observation: format!(
                    "Metadata-only observation: minute {} shows {app} focused on {title}.",
                    sample.minute_mark
                ),
                activity_category: categorize_screenshot_metadata(
                    &app,
                    &title,
                    &sample.screenshot.capture_status,
                ),
                project_hints: point_project_hints,
                identity_tags: fallback_identity_tags(&app, &title),
                routine_tags: fallback_routine_tags(&app, &title),
            }
        })
        .collect::<Vec<_>>();
    let primary_activity = dominant_activity(&trajectory);
    let visible_apps = dedupe_strings(
        samples
            .iter()
            .filter_map(|sample| sample.screenshot.process_name.clone())
            .collect(),
    );
    let visible_text_hints = dedupe_strings(
        samples
            .iter()
            .filter_map(|sample| sample.screenshot.window_title.clone())
            .collect(),
    );
    let project_hints = dedupe_strings(
        samples
            .iter()
            .flat_map(|sample| {
                let app = sample.screenshot.process_name.clone().unwrap_or_default();
                let title = sample.screenshot.window_title.clone().unwrap_or_default();
                project_hints_from_metadata(&app, &title)
            })
            .collect(),
    );
    let identity_tags = unknown_if_empty(dedupe_strings(
        trajectory
            .iter()
            .flat_map(|point| point.identity_tags.clone())
            .collect(),
    ));
    let routine_tags = unknown_if_empty(dedupe_strings(
        trajectory
            .iter()
            .flat_map(|point| point.routine_tags.clone())
            .collect(),
    ));
    let risk_flags = if samples.len() == WINDOW_SAMPLE_MARKS.len() {
        Vec::new()
    } else {
        vec!["incomplete_window_samples".to_string()]
    };

    VisualWindowSummary {
        id: 0,
        window_start,
        window_end,
        sampled_screenshot_ids: samples
            .iter()
            .map(|sample| sample.screenshot.id)
            .collect::<Vec<_>>(),
        previous_summary_id,
        model_provider: "local_stub".to_string(),
        model_name: "metadata-window-v1".to_string(),
        prompt_version: "visual-window-summary-v1".to_string(),
        summary_text: format!(
            "Metadata-only 5-minute window summary from {} to {} with {} screenshots.",
            window_start.to_rfc3339(),
            window_end.to_rfc3339(),
            samples.len()
        ),
        continuity: if previous_summary_id.is_some() {
            "continued_or_unknown".to_string()
        } else {
            "new_or_unknown".to_string()
        },
        primary_activity,
        project_hints,
        identity_tags,
        routine_tags,
        task_intent: "Metadata-only task intent is uncertain.".to_string(),
        trajectory,
        switching_level: "unknown".to_string(),
        switching_evidence: "Metadata-only fallback cannot judge visual switching.".to_string(),
        loafing_level: "unknown".to_string(),
        loafing_evidence: "Metadata-only fallback cannot judge loafing.".to_string(),
        visible_apps,
        visible_text_hints,
        risk_flags,
        confidence: 0.35,
        raw_summary_json: serde_json::json!({}),
        created_at,
        error: None,
    }
}

fn ensure_window_sample_marks(samples: &[WindowVisualAnalysisSample<'_>]) -> Result<()> {
    let marks = samples
        .iter()
        .map(|sample| sample.minute_mark)
        .collect::<Vec<_>>();
    if marks == WINDOW_SAMPLE_MARKS {
        Ok(())
    } else {
        bail!(
            "window visual analysis requires minute marks {:?}",
            WINDOW_SAMPLE_MARKS
        )
    }
}

fn owned_window_samples(input: &WindowVisualAnalysisInput<'_>) -> Vec<WindowScreenshotSample> {
    input
        .samples
        .iter()
        .map(|sample| WindowScreenshotSample {
            minute_mark: sample.minute_mark,
            screenshot: sample.screenshot.clone(),
        })
        .collect()
}

fn model_trajectory_to_points(
    items: &[ModelTrajectoryPointJson],
    samples: &[WindowScreenshotSample],
) -> Vec<VisualTrajectoryPoint> {
    items
        .iter()
        .filter_map(|item| {
            let sample = samples
                .iter()
                .find(|sample| sample.minute_mark == item.minute_mark)?;
            Some(VisualTrajectoryPoint {
                minute_mark: item.minute_mark,
                screenshot_id: sample.screenshot.id,
                observation: item.observation.clone(),
                activity_category: item
                    .activity_category
                    .as_deref()
                    .and_then(ActivityCategory::from_db)
                    .unwrap_or(ActivityCategory::Unknown),
                project_hints: item.project_hints.clone().unwrap_or_default(),
                identity_tags: item
                    .identity_tags
                    .as_ref()
                    .map(|tags| sanitize_identity_tags(tags))
                    .unwrap_or_else(|| vec!["unknown".to_string()]),
                routine_tags: item
                    .routine_tags
                    .as_ref()
                    .map(|tags| sanitize_routine_tags(tags))
                    .unwrap_or_else(|| vec!["unknown".to_string()]),
            })
        })
        .collect()
}

fn dominant_activity(points: &[VisualTrajectoryPoint]) -> ActivityCategory {
    let mut counts: Vec<(ActivityCategory, usize)> = Vec::new();
    for point in points {
        if let Some((_, count)) = counts
            .iter_mut()
            .find(|(activity, _)| activity == &point.activity_category)
        {
            *count += 1;
        } else {
            counts.push((point.activity_category.clone(), 1));
        }
    }
    counts
        .into_iter()
        .max_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| right.0.as_str().cmp(left.0.as_str()))
        })
        .map(|(activity, _)| activity)
        .unwrap_or(ActivityCategory::Unknown)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !value.trim().is_empty() && !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    deduped
}

fn unknown_if_empty(values: Vec<String>) -> Vec<String> {
    if values.is_empty() {
        vec!["unknown".to_string()]
    } else {
        values
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_completion_content_rejects_length_finish_reason() {
        let error = parse_chat_completion_content(
            r#"{
              "choices": [{
                "finish_reason": "length",
                "message": { "content": "{\"summaryText\":\"截断" }
              }]
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("finish_reason"));
    }
}
