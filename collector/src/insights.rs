use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{
    ActivityCategory, ActivityCategoryCount, DailyActivityStats, DailyBrief, DailyComparison,
    HighResScreenshotMeta, HourlyActivityMetric, InsightReport, VisualObservation, VisualSummary,
    VisualWindowSummary,
};
use crate::prompt_time::minimax_prompt_timestamp;

const LOCAL_REPORT_PROMPT_VERSION: &str = "trajectory-v1";
const DAILY_BRIEF_PROMPT_VERSION: &str = "daily-brief-v1";
const LOCAL_DAILY_BRIEF_MODEL: &str = "daily-brief-local-v1";

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
        identity_tags: summary.identity_tags.clone(),
        routine_tags: summary.routine_tags.clone(),
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

pub fn build_five_hour_report_from_window_summaries(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> InsightReport {
    let category_mix = category_mix_from_window_summaries(window_summaries);
    let project_hints = top_project_hints_from_window_summaries(window_summaries);
    let summary_text = window_report_summary_text(window_summaries, &category_mix);

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
        evidence_count: window_summaries.len(),
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

fn category_mix_from_window_summaries(
    window_summaries: &[VisualWindowSummary],
) -> Vec<ActivityCategoryCount> {
    let mut counts: Vec<ActivityCategoryCount> = Vec::new();
    for summary in window_summaries {
        if let Some(existing) = counts
            .iter_mut()
            .find(|item| item.activity_category == summary.primary_activity)
        {
            existing.count += 1;
        } else {
            counts.push(ActivityCategoryCount {
                activity_category: summary.primary_activity.clone(),
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

fn top_project_hints_from_window_summaries(
    window_summaries: &[VisualWindowSummary],
) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for hint in window_summaries
        .iter()
        .flat_map(|summary| summary.project_hints.iter())
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

fn window_report_summary_text(
    window_summaries: &[VisualWindowSummary],
    category_mix: &[ActivityCategoryCount],
) -> String {
    if window_summaries.is_empty() {
        return "过去 5 小时内没有可用的 5 分钟窗口摘要，暂时无法推断工作轨迹。".into();
    }

    let dominant = category_mix
        .first()
        .map(|item| item.activity_category.as_str())
        .unwrap_or(ActivityCategory::Unknown.as_str());
    let first = window_summaries
        .first()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");
    let last = window_summaries
        .last()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");

    format!(
        "过去 5 小时内共分析 {} 条 5 分钟窗口摘要，主要活动类型是 {}。起点：{}。最近状态：{}。",
        window_summaries.len(),
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

    pub async fn report_from_window_summaries(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        window_summaries: &[VisualWindowSummary],
    ) -> Result<InsightReport> {
        match self {
            Self::Local(reporter) => {
                reporter.report_from_window_summaries(period_start, period_end, window_summaries)
            }
            Self::MiniMax(reporter) => {
                reporter
                    .report_from_window_summaries(
                        period_start,
                        period_end,
                        window_summaries,
                        Utc::now(),
                    )
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

    pub fn report_from_window_summaries(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        window_summaries: &[VisualWindowSummary],
    ) -> Result<InsightReport> {
        Ok(build_five_hour_report_from_window_summaries(
            period_start,
            period_end,
            window_summaries,
        ))
    }
}

#[derive(Debug, Clone)]
pub enum ConfiguredDailyBriefReporter {
    Local(LocalDailyBriefReporter),
    MiniMax(MiniMaxDailyBriefReporter),
}

impl ConfiguredDailyBriefReporter {
    pub fn from_env() -> Result<Self> {
        let provider = std::env::var("DAILY_BRIEF_PROVIDER").ok();
        let api_key = std::env::var("MINIMAX_API_KEY").ok();
        let base_url = std::env::var("MINIMAX_BASE_URL").ok();
        let selected_provider = select_insight_report_provider(
            provider.as_deref(),
            api_key.as_deref(),
            base_url.as_deref(),
        );

        match selected_provider.to_ascii_lowercase().as_str() {
            "minimax" => Ok(Self::MiniMax(MiniMaxDailyBriefReporter::new(
                MiniMaxInsightConfig::from_daily_env()?,
            ))),
            "local" | "local_stub" | "" => Ok(Self::Local(LocalDailyBriefReporter)),
            other => bail!("unsupported DAILY_BRIEF_PROVIDER: {other}"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn report(
        &self,
        date: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scheduled_for_local: &str,
        stats: &DailyActivityStats,
        hourly_metrics: &[HourlyActivityMetric],
        comparison: &DailyComparison,
        reports: &[InsightReport],
    ) -> Result<DailyBrief> {
        let generated_at = Utc::now();
        match self {
            Self::Local(reporter) => reporter.report(
                date,
                period_start,
                period_end,
                scheduled_for_local,
                stats,
                hourly_metrics,
                comparison,
                reports,
                generated_at,
            ),
            Self::MiniMax(reporter) => {
                reporter
                    .report(
                        date,
                        period_start,
                        period_end,
                        scheduled_for_local,
                        stats,
                        hourly_metrics,
                        comparison,
                        reports,
                        generated_at,
                    )
                    .await
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalDailyBriefReporter;

impl LocalDailyBriefReporter {
    #[allow(clippy::too_many_arguments)]
    pub fn report(
        &self,
        date: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scheduled_for_local: &str,
        stats: &DailyActivityStats,
        hourly_metrics: &[HourlyActivityMetric],
        comparison: &DailyComparison,
        reports: &[InsightReport],
        generated_at: DateTime<Utc>,
    ) -> Result<DailyBrief> {
        let report_ids = reports.iter().map(|report| report.id).collect::<Vec<_>>();
        let dominant = stats
            .category_mix
            .first()
            .map(|item| item.activity_category.as_str())
            .unwrap_or(ActivityCategory::Unknown.as_str());
        let daily_summary_text = format!(
            "{date} 记录包含 {:.1} 小时活跃桌面时间，主要活动类型为 {dominant}，覆盖 {} 个 5 小时报告。",
            stats.active_hours,
            reports.len()
        );
        let action_trajectory = if reports.is_empty() {
            "当日没有可用的 5 小时报告，暂不能形成连续行动轨迹。".to_string()
        } else {
            reports
                .iter()
                .map(|report| {
                    format!(
                        "{} - {}：{}",
                        report.period_start.format("%H:%M"),
                        report.period_end.format("%H:%M"),
                        report.summary_text
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let raw_summary_json = serde_json::json!({
            "dailySummaryText": daily_summary_text,
            "actionTrajectory": action_trajectory,
            "comparisonExplanation": comparison.explanation
        });

        Ok(DailyBrief {
            id: 0,
            date: date.into(),
            period_start,
            period_end,
            generated_at,
            scheduled_for_local: scheduled_for_local.into(),
            model_provider: "local_insight".into(),
            model_name: LOCAL_DAILY_BRIEF_MODEL.into(),
            prompt_version: DAILY_BRIEF_PROMPT_VERSION.into(),
            status: "complete".into(),
            descriptive_stats: stats.clone(),
            hourly_metrics: hourly_metrics.to_vec(),
            comparison: comparison.clone(),
            five_hour_report_ids: report_ids,
            daily_summary_text,
            action_trajectory,
            raw_summary_json,
            error: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MiniMaxDailyBriefReporter {
    client: reqwest::Client,
    config: MiniMaxInsightConfig,
}

impl MiniMaxDailyBriefReporter {
    pub fn new(config: MiniMaxInsightConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_chat_completions_request(
        &self,
        date: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        stats: &DailyActivityStats,
        hourly_metrics: &[HourlyActivityMetric],
        comparison: &DailyComparison,
        reports: &[InsightReport],
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You write a neutral Chinese daily desktop-work brief from structured activity metrics and five-hour reports. Return compact JSON only."
                },
                {
                    "role": "user",
                    "content": daily_brief_prompt(
                        date,
                        period_start,
                        period_end,
                        stats,
                        hourly_metrics,
                        comparison,
                        reports,
                    )
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
        body
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn report(
        &self,
        date: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scheduled_for_local: &str,
        stats: &DailyActivityStats,
        hourly_metrics: &[HourlyActivityMetric],
        comparison: &DailyComparison,
        reports: &[InsightReport],
        generated_at: DateTime<Utc>,
    ) -> Result<DailyBrief> {
        let body = self.build_chat_completions_request(
            date,
            period_start,
            period_end,
            stats,
            hourly_metrics,
            comparison,
            reports,
        );
        let response = self
            .client
            .post(self.config.chat_completions_url())
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .context("MiniMax daily brief request failed")?;
        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("MiniMax daily brief response body read failed")?;
        if !status.is_success() {
            bail!("MiniMax daily brief returned {status}: {response_text}");
        }
        let content = parse_chat_completion_content(&response_text)?;
        Self::brief_from_response_text(
            date,
            period_start,
            period_end,
            scheduled_for_local,
            stats,
            hourly_metrics,
            comparison,
            reports,
            generated_at,
            &self.config.model,
            &content,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn brief_from_response_text(
        date: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scheduled_for_local: &str,
        stats: &DailyActivityStats,
        hourly_metrics: &[HourlyActivityMetric],
        comparison: &DailyComparison,
        reports: &[InsightReport],
        generated_at: DateTime<Utc>,
        model_name: &str,
        content: &str,
    ) -> Result<DailyBrief> {
        let local = LocalDailyBriefReporter.report(
            date,
            period_start,
            period_end,
            scheduled_for_local,
            stats,
            hourly_metrics,
            comparison,
            reports,
            generated_at,
        )?;
        let parsed = parse_model_daily_brief_json(content);
        let mut comparison = comparison.clone();
        if let Some(explanation) = parsed
            .as_ref()
            .and_then(|value| value.comparison_explanation.clone())
        {
            comparison.explanation = explanation;
        }

        Ok(DailyBrief {
            model_provider: "minimax".into(),
            model_name: model_name.into(),
            daily_summary_text: parsed
                .as_ref()
                .and_then(|value| value.daily_summary_text.clone())
                .unwrap_or(local.daily_summary_text),
            action_trajectory: parsed
                .as_ref()
                .and_then(|value| value.action_trajectory.clone())
                .unwrap_or(local.action_trajectory),
            raw_summary_json: crate::llm_json::parse_json_object(content)
                .unwrap_or_else(|| serde_json::json!({ "content": content.trim() })),
            comparison,
            ..local
        })
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
            max_completion_tokens: 10_000,
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

    pub fn from_daily_env() -> Result<Self> {
        let api_key = std::env::var("MINIMAX_API_KEY")
            .context("MINIMAX_API_KEY is required when DAILY_BRIEF_PROVIDER=minimax")?;
        let base_url = std::env::var("MINIMAX_BASE_URL")
            .context("MINIMAX_BASE_URL is required when DAILY_BRIEF_PROVIDER=minimax")?;
        let model = std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M3".to_string());
        let mut config = Self::new(api_key, base_url, model);
        config.max_completion_tokens = 10_000;
        if let Ok(value) = std::env::var("MINIMAX_DAILY_BRIEF_MAX_COMPLETION_TOKENS")
            .or_else(|_| std::env::var("MINIMAX_MAX_COMPLETION_TOKENS"))
        {
            config.max_completion_tokens = value.parse().with_context(|| {
                format!("MINIMAX_DAILY_BRIEF_MAX_COMPLETION_TOKENS must be an integer, got {value}")
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
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You infer a personal work trajectory from timestamped screenshot summaries. Return one complete compact JSON object only, without markdown fences."
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
        });
        if let Some(response_format) =
            crate::llm_json::minimax_json_response_format(&self.config.model)
        {
            body["response_format"] = response_format;
        }
        body
    }

    pub fn build_window_summary_chat_completions_request(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        window_summaries: &[VisualWindowSummary],
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You infer a personal work trajectory from structured 5-minute screenshot-window summaries. Return one complete compact JSON object only, without markdown fences."
                },
                {
                    "role": "user",
                    "content": window_summary_report_prompt(period_start, period_end, window_summaries)
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
        body
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

    pub async fn report_from_window_summaries(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        window_summaries: &[VisualWindowSummary],
        generated_at: DateTime<Utc>,
    ) -> Result<InsightReport> {
        let body = self.build_window_summary_chat_completions_request(
            period_start,
            period_end,
            window_summaries,
        );
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
        Self::report_from_window_summary_response_text(
            period_start,
            period_end,
            window_summaries,
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

    pub fn report_from_window_summary_response_text(
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        window_summaries: &[VisualWindowSummary],
        generated_at: DateTime<Utc>,
        model_name: &str,
        content: &str,
    ) -> Result<InsightReport> {
        let local = build_five_hour_report_from_window_summaries(
            period_start,
            period_end,
            window_summaries,
        );
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
            evidence_count: window_summaries.len(),
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
#[serde(rename_all = "camelCase")]
struct ModelDailyBriefJson {
    daily_summary_text: Option<String>,
    action_trajectory: Option<String>,
    comparison_explanation: Option<String>,
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

fn report_prompt(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    observations: &[VisualObservation],
) -> String {
    let observations_json = observations
        .iter()
        .map(|observation| {
            serde_json::json!({
                "capturedAt": minimax_prompt_timestamp(observation.captured_at),
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
        "Infer the user's work trajectory for this 5-hour window. Return JSON only with keys summaryText and projectHints. summaryText must be complete, structured, human-readable Chinese. Focus on projects, time allocation, workflow pattern, switching or loafing signs, and uncertainty; do not list every 5-minute window. periodStart={}, periodEnd={}, observations={}",
        minimax_prompt_timestamp(period_start),
        minimax_prompt_timestamp(period_end),
        serde_json::to_string(&observations_json).unwrap_or_else(|_| "[]".to_string())
    )
}

fn window_summary_report_prompt(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> String {
    let summaries_json = window_summaries
        .iter()
        .map(|summary| {
            serde_json::json!({
                "windowStart": minimax_prompt_timestamp(summary.window_start),
                "windowEnd": minimax_prompt_timestamp(summary.window_end),
                "summaryText": summary.summary_text,
                "continuity": summary.continuity,
                "primaryActivity": summary.primary_activity.as_str(),
                "projectHints": summary.project_hints,
                "taskIntent": summary.task_intent,
                "trajectory": summary.trajectory,
                "switchingLevel": summary.switching_level,
                "switchingEvidence": summary.switching_evidence,
                "loafingLevel": summary.loafing_level,
                "loafingEvidence": summary.loafing_evidence,
                "visibleApps": summary.visible_apps,
                "visibleTextHints": summary.visible_text_hints,
                "riskFlags": summary.risk_flags,
                "confidence": summary.confidence
            })
        })
        .collect::<Vec<_>>();

    format!(
        "Infer the user's work trajectory for this 5-hour window from 5-minute structured summaries. Return JSON only with keys summaryText and projectHints. summaryText must be complete, structured, human-readable Chinese and cover: project-based work path, time allocation, possible loafing, switching frequency, long-run pattern, and uncertainty. Do not produce a 5-minute log. periodStart={}, periodEnd={}, windowSummaries={}",
        minimax_prompt_timestamp(period_start),
        minimax_prompt_timestamp(period_end),
        serde_json::to_string(&summaries_json).unwrap_or_else(|_| "[]".to_string())
    )
}

fn daily_activity_stats_prompt_value(stats: &DailyActivityStats) -> serde_json::Value {
    serde_json::json!({
        "date": &stats.date,
        "periodStart": minimax_prompt_timestamp(stats.period_start),
        "periodEnd": minimax_prompt_timestamp(stats.period_end),
        "activeSeconds": stats.active_seconds,
        "activeHours": stats.active_hours,
        "windowEventCount": stats.window_event_count,
        "switchCount": stats.switch_count,
        "distinctAppCount": stats.distinct_app_count,
        "topApps": &stats.top_apps,
        "categoryMix": &stats.category_mix,
        "inputChars": stats.input_chars,
        "inputEvents": stats.input_events,
        "screenshotCount": stats.screenshot_count,
        "highResScreenshotCount": stats.high_res_screenshot_count,
        "visualWindowCount": stats.visual_window_count,
        "fiveHourReportCount": stats.five_hour_report_count,
        "firstActivityAt": stats.first_activity_at.map(minimax_prompt_timestamp),
        "lastActivityAt": stats.last_activity_at.map(minimax_prompt_timestamp),
    })
}

fn hourly_metrics_prompt_values(metrics: &[HourlyActivityMetric]) -> Vec<serde_json::Value> {
    metrics
        .iter()
        .map(|metric| {
            serde_json::json!({
                "hour": metric.hour,
                "startAt": minimax_prompt_timestamp(metric.start_at),
                "endAt": minimax_prompt_timestamp(metric.end_at),
                "activeSeconds": metric.active_seconds,
                "activeRatio": metric.active_ratio,
                "windowEventCount": metric.window_event_count,
                "switchCount": metric.switch_count,
                "distinctAppCount": metric.distinct_app_count,
                "dominantApp": &metric.dominant_app,
                "dominantCategory": &metric.dominant_category,
                "inputChars": metric.input_chars,
                "screenshotCount": metric.screenshot_count,
                "highResScreenshotCount": metric.high_res_screenshot_count,
                "visualWindowCount": metric.visual_window_count,
                "fiveHourReportIds": &metric.five_hour_report_ids,
            })
        })
        .collect()
}

fn daily_brief_prompt(
    date: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    comparison: &DailyComparison,
    reports: &[InsightReport],
) -> String {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ReportPromptRow<'a> {
        period_start: String,
        period_end: String,
        summary_text: &'a str,
        category_mix: &'a [ActivityCategoryCount],
        project_hints: &'a [String],
        evidence_count: usize,
    }

    let report_rows = reports
        .iter()
        .map(|report| ReportPromptRow {
            period_start: minimax_prompt_timestamp(report.period_start),
            period_end: minimax_prompt_timestamp(report.period_end),
            summary_text: &report.summary_text,
            category_mix: &report.category_mix,
            project_hints: &report.project_hints,
            evidence_count: report.evidence_count,
        })
        .collect::<Vec<_>>();
    let stats = daily_activity_stats_prompt_value(stats);
    let hourly_metrics = hourly_metrics_prompt_values(hourly_metrics);

    format!(
        "Write a neutral daily brief in Chinese. Return JSON only with keys dailySummaryText, actionTrajectory, comparisonExplanation. Do not include advice, praise, criticism, ranking, or value judgment. Avoid words equivalent to productive, wasted, efficient, inefficient, good, bad, should. actionTrajectory must be complete, human-readable, and structured around parallel projects, time allocation, workflow pattern, work mode, design/tooling activity, and important evidence; do not create a raw 5-minute log. Use uncertainty when evidence is incomplete. date={}, periodStart={}, periodEnd={}, descriptiveStats={}, hourlyMetrics={}, comparison={}, fiveHourReports={}",
        date,
        minimax_prompt_timestamp(period_start),
        minimax_prompt_timestamp(period_end),
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string(&hourly_metrics).unwrap_or_else(|_| "[]".to_string()),
        serde_json::to_string(comparison).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string(&report_rows).unwrap_or_else(|_| "[]".to_string())
    )
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
            anyhow::anyhow!(
                "MiniMax response did not include complete message content; finish_reason may be length"
            )
        })
}

fn parse_model_report_json(content: &str) -> Option<ModelReportJson> {
    crate::llm_json::parse_json_object_as(content)
}

fn parse_model_daily_brief_json(content: &str) -> Option<ModelDailyBriefJson> {
    crate::llm_json::parse_json_object_as(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_completion_content_accepts_stop_finish_reason() {
        let content = parse_chat_completion_content(
            r#"{
              "choices": [
                {
                  "finish_reason": "stop",
                  "message": { "content": "{\"summaryText\":\"完整报告\",\"projectHints\":[]}" }
                }
              ]
            }"#,
        )
        .unwrap();

        assert!(content.contains("完整报告"));
    }

    #[test]
    fn parse_chat_completion_content_rejects_length_finish_reason() {
        let error = parse_chat_completion_content(
            r#"{
              "choices": [
                {
                  "finish_reason": "length",
                  "message": { "content": "{\"summaryText\":\"半截报告" }
                }
              ]
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("finish_reason"));
    }
}
