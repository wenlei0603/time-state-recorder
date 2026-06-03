use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::{
    ActivityCategory, ActivityCategoryCount, HighResScreenshotMeta, InsightReport,
    VisualObservation, VisualSummary,
};

const LOCAL_REPORT_PROMPT_VERSION: &str = "trajectory-v1";

pub fn observation_from_visual_summary(
    high_res: &HighResScreenshotMeta,
    summary: &VisualSummary,
) -> VisualObservation {
    VisualObservation {
        id: 0,
        high_res_screenshot_id: high_res.id,
        captured_at: high_res.captured_at,
        file_path: high_res.file_path.clone(),
        model_provider: summary.model_provider.clone(),
        model_name: summary.model_name.clone(),
        prompt_version: summary.prompt_version.clone(),
        summary_text: summary.summary_text.clone(),
        activity_category: summary.activity_category.clone(),
        project_hints: summary.project_hints.clone(),
        visible_apps: summary.visible_apps.clone(),
        visible_text_hints: summary.visible_text_hints.clone(),
        risk_flags: summary.risk_flags.clone(),
        confidence: summary.confidence,
        created_at: summary.created_at,
        error: summary.error.clone(),
    }
}

pub fn build_five_hour_report(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    observations: &[VisualObservation],
) -> InsightReport {
    let category_mix = category_mix(observations);
    let project_hints = top_project_hints(observations);
    let summary_text = report_summary_text(observations, &category_mix);

    InsightReport {
        id: 0,
        period_start,
        period_end,
        generated_at: Utc::now(),
        report_kind: "5h".into(),
        model_provider: "local_insight".into(),
        model_name: LOCAL_REPORT_PROMPT_VERSION.into(),
        summary_text,
        category_mix,
        project_hints,
        evidence_count: observations.len(),
        error: None,
    }
}

fn category_mix(observations: &[VisualObservation]) -> Vec<ActivityCategoryCount> {
    let mut counts: Vec<ActivityCategoryCount> = Vec::new();
    for observation in observations {
        if let Some(existing) = counts
            .iter_mut()
            .find(|item| item.activity_category == observation.activity_category)
        {
            existing.count += 1;
        } else {
            counts.push(ActivityCategoryCount {
                activity_category: observation.activity_category.clone(),
                count: 1,
            });
        }
    }
    counts.sort_by(|left, right| {
        right.count.cmp(&left.count).then_with(|| {
            left.activity_category
                .as_str()
                .cmp(right.activity_category.as_str())
        })
    });
    counts
}

fn top_project_hints(observations: &[VisualObservation]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for hint in observations
        .iter()
        .flat_map(|observation| observation.project_hints.iter())
    {
        if let Some((_, count)) = counts.iter_mut().find(|(value, _)| value == hint) {
            *count += 1;
        } else {
            counts.push((hint.clone(), 1));
        }
    }
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts.into_iter().take(5).map(|(hint, _)| hint).collect()
}

fn report_summary_text(
    observations: &[VisualObservation],
    category_mix: &[ActivityCategoryCount],
) -> String {
    if observations.is_empty() {
        return "这 5 小时内没有可用的视觉摘要，暂时无法推断工作轨迹。".into();
    }

    let dominant = category_mix
        .first()
        .map(|item| item.activity_category.as_str())
        .unwrap_or(ActivityCategory::Unknown.as_str());
    let first = observations
        .first()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");
    let last = observations
        .last()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");

    format!(
        "这 5 小时内共分析 {} 张屏幕图像，主要活动类型是 {}。起点：{}。最近状态：{}。",
        observations.len(),
        dominant,
        first,
        last
    )
}

#[derive(Debug, Clone)]
pub enum ConfiguredInsightReporter {
    Local(LocalInsightReporter),
    MiniMax(MiniMaxInsightReporter),
}

impl ConfiguredInsightReporter {
    pub fn from_env() -> Result<Self> {
        let provider = std::env::var("INSIGHT_REPORT_PROVIDER").ok();
        let api_key = std::env::var("MINIMAX_API_KEY").ok();
        let base_url = std::env::var("MINIMAX_BASE_URL").ok();
        let selected_provider = select_insight_report_provider(
            provider.as_deref(),
            api_key.as_deref(),
            base_url.as_deref(),
        );

        match selected_provider.to_ascii_lowercase().as_str() {
            "minimax" => Ok(Self::MiniMax(MiniMaxInsightReporter::new(
                MiniMaxInsightConfig::from_env()?,
            ))),
            "local" | "local_stub" | "" => Ok(Self::Local(LocalInsightReporter)),
            other => bail!("unsupported INSIGHT_REPORT_PROVIDER: {other}"),
        }
    }

    pub async fn report(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        observations: &[VisualObservation],
    ) -> Result<InsightReport> {
        match self {
            Self::Local(reporter) => reporter.report(period_start, period_end, observations),
            Self::MiniMax(reporter) => {
                reporter
                    .report(period_start, period_end, observations, Utc::now())
                    .await
            }
        }
    }
}

pub fn select_insight_report_provider<'a>(
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
pub struct LocalInsightReporter;

impl LocalInsightReporter {
    pub fn report(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        observations: &[VisualObservation],
    ) -> Result<InsightReport> {
        Ok(build_five_hour_report(
            period_start,
            period_end,
            observations,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct MiniMaxInsightConfig {
    api_key: String,
    base_url: String,
    model: String,
    max_completion_tokens: u32,
}

impl MiniMaxInsightConfig {
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
            max_completion_tokens: 900,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("MINIMAX_API_KEY")
            .context("MINIMAX_API_KEY is required when INSIGHT_REPORT_PROVIDER=minimax")?;
        let base_url = std::env::var("MINIMAX_BASE_URL")
            .context("MINIMAX_BASE_URL is required when INSIGHT_REPORT_PROVIDER=minimax")?;
        let model = std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M3".to_string());
        let mut config = Self::new(api_key, base_url, model);
        if let Ok(value) = std::env::var("MINIMAX_REPORT_MAX_COMPLETION_TOKENS")
            .or_else(|_| std::env::var("MINIMAX_MAX_COMPLETION_TOKENS"))
        {
            config.max_completion_tokens = value.parse().with_context(|| {
                format!("MINIMAX_REPORT_MAX_COMPLETION_TOKENS must be an integer, got {value}")
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
pub struct MiniMaxInsightReporter {
    client: reqwest::Client,
    config: MiniMaxInsightConfig,
}

impl MiniMaxInsightReporter {
    pub fn new(config: MiniMaxInsightConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub fn build_chat_completions_request(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        observations: &[VisualObservation],
    ) -> serde_json::Value {
        serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You infer a personal work trajectory from timestamped screenshot summaries. Return compact JSON only."
                },
                {
                    "role": "user",
                    "content": report_prompt(period_start, period_end, observations)
                }
            ],
            "temperature": 0.2,
            "top_p": 0.95,
            "max_completion_tokens": self.config.max_completion_tokens,
            "thinking": { "type": "disabled" }
        })
    }

    pub async fn report(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        observations: &[VisualObservation],
        generated_at: DateTime<Utc>,
    ) -> Result<InsightReport> {
        let body = self.build_chat_completions_request(period_start, period_end, observations);
        let response = self
            .client
            .post(self.config.chat_completions_url())
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .context("MiniMax insight report request failed")?;
        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("MiniMax insight response body read failed")?;
        if !status.is_success() {
            bail!("MiniMax insight report returned {status}: {response_text}");
        }
        let content = parse_chat_completion_content(&response_text)?;
        Self::report_from_response_text(
            period_start,
            period_end,
            observations,
            generated_at,
            &self.config.model,
            &content,
        )
    }

    pub fn report_from_response_text(
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        observations: &[VisualObservation],
        generated_at: DateTime<Utc>,
        model_name: &str,
        content: &str,
    ) -> Result<InsightReport> {
        let local = build_five_hour_report(period_start, period_end, observations);
        let parsed = parse_model_report_json(content);
        Ok(InsightReport {
            id: 0,
            period_start,
            period_end,
            generated_at,
            report_kind: "5h".into(),
            model_provider: "minimax".into(),
            model_name: model_name.into(),
            summary_text: parsed
                .as_ref()
                .and_then(|value| value.summary_text.clone())
                .unwrap_or_else(|| content.trim().to_string()),
            category_mix: local.category_mix,
            project_hints: parsed
                .as_ref()
                .and_then(|value| value.project_hints.clone())
                .unwrap_or(local.project_hints),
            evidence_count: observations.len(),
            error: None,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelReportJson {
    summary_text: Option<String>,
    project_hints: Option<Vec<String>>,
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

fn report_prompt(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    observations: &[VisualObservation],
) -> String {
    let observations_json = observations
        .iter()
        .map(|observation| {
            serde_json::json!({
                "capturedAt": observation.captured_at,
                "summaryText": observation.summary_text,
                "activityCategory": observation.activity_category.as_str(),
                "projectHints": observation.project_hints,
                "visibleApps": observation.visible_apps,
                "visibleTextHints": observation.visible_text_hints,
                "confidence": observation.confidence
            })
        })
        .collect::<Vec<_>>();

    format!(
        "Infer the user's work trajectory for this 5-hour window. Return JSON only with keys summaryText and projectHints. summaryText should be concise Chinese, mention sequence, dominant activity, possible project, switching or loafing signs, and uncertainty. periodStart={}, periodEnd={}, observations={}",
        period_start.to_rfc3339(),
        period_end.to_rfc3339(),
        serde_json::to_string(&observations_json).unwrap_or_else(|_| "[]".to_string())
    )
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
        .ok_or_else(|| anyhow::anyhow!("MiniMax response did not include message content"))
}

fn parse_model_report_json(content: &str) -> Option<ModelReportJson> {
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
