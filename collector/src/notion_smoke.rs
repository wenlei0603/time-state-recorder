use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::Value;
use tokio::net::TcpListener;

use crate::{
    api,
    models::{
        ActivityCategory, ActivityCategoryCount, DailyActivityStats, DailyAppActivity, DailyBrief,
        DailyComparison, HourlyActivityMetric, InsightReport,
    },
    storage::Store,
};

pub const SMOKE_DATE: &str = "2026-05-24";
pub const DEFAULT_ARTIFACT_FILE_NAME: &str = "notion-daily-archive-smoke.json";
pub const DEFAULT_ARTIFACT_PATH: &str = "reports/notion-daily-archive-smoke.json";
pub const REQUIRED_MARKDOWN_SECTIONS: &[&str] = &[
    "## Daily Summary",
    "## Descriptive Statistics",
    "## Parallel Projects And Time Allocation",
    "## Workflow Pattern",
    "## Five-Hour Reports",
    "## Comparison",
    "## Uncertainty And Review Notes",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionDailyArchiveSmokeSummary {
    pub date: String,
    pub artifact_path: PathBuf,
    pub endpoint_url: String,
    pub five_hour_report_count: usize,
    pub markdown_sections_checked: Vec<&'static str>,
}

pub async fn run_notion_daily_archive_smoke(
    artifact_path: impl AsRef<Path>,
) -> Result<NotionDailyArchiveSmokeSummary> {
    let artifact_path = artifact_path.as_ref().to_path_buf();
    let store = sample_store()?;
    let app = api::router(store, None);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("notion daily archive smoke server stopped: {err}");
        }
    });

    let result = fetch_assert_and_write(addr, artifact_path).await;
    server.abort();
    result
}

async fn fetch_assert_and_write(
    addr: SocketAddr,
    artifact_path: PathBuf,
) -> Result<NotionDailyArchiveSmokeSummary> {
    let endpoint_url =
        format!("http://{addr}/api/notion/daily-archive?date={SMOKE_DATE}&tzOffsetMinutes=0");
    let response = reqwest::get(&endpoint_url).await?;
    let status = response.status();
    ensure!(
        status == StatusCode::OK,
        "expected HTTP 200 from smoke endpoint, got {status}"
    );
    let body: Value = response.json().await?;
    assert_daily_archive_body(&body)?;

    if let Some(parent) = artifact_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let pretty = serde_json::to_string_pretty(&body)?;
    fs::write(&artifact_path, format!("{pretty}\n"))?;

    Ok(NotionDailyArchiveSmokeSummary {
        date: SMOKE_DATE.into(),
        artifact_path,
        endpoint_url,
        five_hour_report_count: 2,
        markdown_sections_checked: REQUIRED_MARKDOWN_SECTIONS.to_vec(),
    })
}

fn assert_daily_archive_body(body: &Value) -> Result<()> {
    ensure!(body["date"] == SMOKE_DATE, "smoke response date mismatch");
    ensure!(
        body["dailyDiaryTitle"] == "INDEX-20260524 | Daily Diary",
        "smoke response daily diary title mismatch"
    );
    ensure!(
        body["source"]["endpoint"] == "/api/notion/daily-archive",
        "smoke response source endpoint mismatch"
    );
    ensure!(
        body["descriptiveStats"]["fiveHourReportCount"] == 2,
        "smoke response stats must report two 5-hour reports"
    );

    let reports = body["fiveHourReports"]
        .as_array()
        .context("fiveHourReports must be an array")?;
    ensure!(reports.len() == 2, "expected two same-day 5-hour reports");
    ensure!(
        reports
            .iter()
            .all(|report| report["reportKind"].as_str() == Some("5h")),
        "all smoke reports must be 5h reports"
    );
    ensure!(
        reports
            .iter()
            .any(|report| report["summaryText"] == "Morning build report."),
        "missing morning same-day report"
    );
    ensure!(
        reports
            .iter()
            .any(|report| report["summaryText"] == "Afternoon documentation report."),
        "missing afternoon same-day report"
    );
    ensure!(
        !reports
            .iter()
            .any(|report| report["summaryText"] == "Previous-day report should be excluded."),
        "previous-day report leaked into same-day archive"
    );

    let markdown = body["archiveMarkdown"]
        .as_str()
        .context("archiveMarkdown must be a string")?;
    for section in REQUIRED_MARKDOWN_SECTIONS {
        ensure!(
            markdown.contains(section),
            "archiveMarkdown missing {section}"
        );
    }
    ensure!(
        markdown.contains("Morning build report.")
            && markdown.contains("Afternoon documentation report."),
        "archiveMarkdown missing same-day report text"
    );
    ensure!(
        !markdown.contains("Previous-day report should be excluded."),
        "archiveMarkdown included a previous-day report"
    );
    Ok(())
}

fn sample_store() -> Result<Store> {
    let mut store = Store::open_memory()?;
    store.init()?;
    let first_report_id = store.insert_insight_report(&sample_insight_report(
        "2026-05-24T05:00:00Z",
        "2026-05-24T10:00:00Z",
        "Morning build report.",
    ))?;
    let second_report_id = store.insert_insight_report(&sample_insight_report(
        "2026-05-24T10:00:00Z",
        "2026-05-24T15:00:00Z",
        "Afternoon documentation report.",
    ))?;
    store.insert_insight_report(&sample_insight_report(
        "2026-05-23T10:00:00Z",
        "2026-05-23T15:00:00Z",
        "Previous-day report should be excluded.",
    ))?;

    let mut brief = sample_daily_brief();
    brief.five_hour_report_ids = vec![first_report_id, second_report_id];
    brief.hourly_metrics[0].five_hour_report_ids = vec![first_report_id];
    store.insert_daily_brief(&brief)?;
    Ok(store)
}

fn sample_insight_report(start: &str, end: &str, summary: &str) -> InsightReport {
    InsightReport {
        id: 0,
        period_start: ts(start),
        period_end: ts(end),
        generated_at: ts(end),
        report_kind: "5h".into(),
        model_provider: "local_smoke".into(),
        model_name: "notion-daily-archive-smoke-v1".into(),
        summary_text: summary.into(),
        category_mix: vec![ActivityCategoryCount {
            activity_category: ActivityCategory::Coding,
            count: 1,
        }],
        project_hints: vec!["Time State Recorder".into()],
        evidence_count: 3,
        error: None,
    }
}

fn sample_daily_brief() -> DailyBrief {
    DailyBrief {
        id: 0,
        date: SMOKE_DATE.into(),
        period_start: ts("2026-05-24T00:00:00Z"),
        period_end: ts("2026-05-25T00:00:00Z"),
        generated_at: ts("2026-05-24T15:40:05Z"),
        scheduled_for_local: "23:40".into(),
        model_provider: "local_smoke".into(),
        model_name: "daily-brief-smoke-v1".into(),
        prompt_version: "daily-brief-v1".into(),
        status: "complete".into(),
        descriptive_stats: DailyActivityStats {
            date: SMOKE_DATE.into(),
            period_start: ts("2026-05-24T00:00:00Z"),
            period_end: ts("2026-05-25T00:00:00Z"),
            active_seconds: 3600,
            active_hours: 1.0,
            window_event_count: 4,
            switch_count: 2,
            distinct_app_count: 2,
            top_apps: vec![DailyAppActivity {
                process_name: "Code.exe".into(),
                active_seconds: 2400,
                share: 0.67,
            }],
            category_mix: vec![ActivityCategoryCount {
                activity_category: ActivityCategory::Coding,
                count: 2,
            }],
            input_chars: 120,
            input_events: 140,
            screenshot_count: 6,
            high_res_screenshot_count: 3,
            visual_window_count: 4,
            five_hour_report_count: 2,
            first_activity_at: Some(ts("2026-05-24T05:00:00Z")),
            last_activity_at: Some(ts("2026-05-24T15:00:00Z")),
        },
        hourly_metrics: vec![HourlyActivityMetric {
            hour: 9,
            start_at: ts("2026-05-24T09:00:00Z"),
            end_at: ts("2026-05-24T10:00:00Z"),
            active_seconds: 1800,
            active_ratio: 0.5,
            window_event_count: 2,
            switch_count: 1,
            distinct_app_count: 2,
            dominant_app: Some("Code.exe".into()),
            dominant_category: ActivityCategory::Coding,
            input_chars: 60,
            screenshot_count: 2,
            high_res_screenshot_count: 1,
            visual_window_count: 1,
            five_hour_report_ids: vec![],
        }],
        comparison: DailyComparison {
            baseline_days: 7,
            compared_dates: vec!["2026-05-23".into()],
            active_seconds_delta: 600,
            switches_per_hour_delta: 0.2,
            input_chars_delta: 120,
            screenshot_coverage_delta: 0.1,
            dominant_category_shift: Some("research -> coding".into()),
            start_time_shift_minutes: Some(-10),
            end_time_shift_minutes: Some(20),
            explanation: "Coding windows increased relative to the prior day.".into(),
        },
        five_hour_report_ids: vec![],
        daily_summary_text: "The sample day centered on implementation and archive wiring.".into(),
        action_trajectory:
            "Morning implementation was followed by documentation and Notion archive preparation."
                .into(),
        raw_summary_json: serde_json::json!({
            "dailySummaryText": "The sample day centered on implementation and archive wiring.",
            "actionTrajectory": "Morning implementation was followed by documentation and Notion archive preparation."
        }),
        error: None,
    }
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("smoke sample timestamps must be valid RFC3339")
        .with_timezone(&Utc)
}
