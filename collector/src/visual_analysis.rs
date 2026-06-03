use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::models::{ActivityCategory, ScreenshotMeta, VisualSummary};

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
