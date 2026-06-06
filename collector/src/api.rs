use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, FixedOffset, Local, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tokio::{sync::oneshot, time};
use tower_http::services::ServeDir;

use crate::{
    activity::{ActivityBucketQuery, build_activity_buckets},
    blocker::BlockerEngine,
    image_retention::{ImageRetentionPolicy, cleanup_expired_images},
    input,
    insights::{ConfiguredDailyBriefReporter, ConfiguredInsightReporter, LocalDailyBriefReporter},
    interval::build_time_events_with_lifecycle,
    models::{
        ActivityBucket, ActivityCategoryCount, BlockerHit, CollectorHealth, DailyActivityStats,
        DailyAppActivity, DailyBrief, DailyComparison, DbStats, HighResScreenshotMeta,
        HourlyActivityMetric, InsightReport, LifecycleEvent, LifecycleType, ScreenshotMeta,
        SubsystemHealth, TimeEvent, VisualObservation, VisualSummary, VisualWindowSummary,
        WindowSnapshot,
    },
    screenshot,
    storage::Store,
    visual_analysis::{
        ConfiguredVisualAnalyzer, VisualAnalysisInput, WindowScreenshotSample,
        WindowVisualAnalysisInput, WindowVisualAnalysisSample, select_window_samples,
    },
    window::sample_foreground_window,
};

#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<Store>>,
    blocker_engine: Arc<BlockerEngine>,
    screenshot_dir: Arc<PathBuf>,
    screenshot_interval_secs: Arc<u64>,
    high_res_screenshot_dir: Arc<PathBuf>,
    high_res_screenshot_interval_secs: Arc<u64>,
    idle_threshold_secs: Arc<u64>,
    health: Arc<Mutex<CollectorHealth>>,
    analysis_status: Arc<Mutex<AnalysisStatus>>,
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DateQuery {
    date: Option<String>,
    limit: Option<usize>,
    #[serde(rename = "tzOffsetMinutes")]
    tz_offset_minutes: Option<i32>,
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityBucketsQuery {
    date: Option<String>,
    bucket_seconds: Option<i64>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeEventsResponse {
    events: Vec<TimeEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityBucketsResponse {
    date: String,
    bucket_seconds: i64,
    buckets: Vec<ActivityBucket>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleEventsResponse {
    events: Vec<LifecycleEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockersResponse {
    rules: Vec<crate::models::BlockerRule>,
    hits: Vec<BlockerHit>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotsResponse {
    screenshots: Vec<ScreenshotMeta>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HighResScreenshotsResponse {
    screenshots: Vec<HighResScreenshotMeta>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisualSummariesResponse {
    summaries: Vec<VisualSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisualObservationsResponse {
    observations: Vec<VisualObservation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisualWindowSummariesResponse {
    summaries: Vec<VisualWindowSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsightReportsResponse {
    reports: Vec<InsightReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyBriefResponse {
    date: String,
    status: String,
    next_run_at: Option<DateTime<Utc>>,
    brief: Option<DailyBrief>,
    five_hour_reports: Vec<InsightReport>,
    descriptive_stats: DailyActivityStats,
    hourly_metrics: Vec<HourlyActivityMetric>,
    comparison: DailyComparison,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotionDailyArchiveResponse {
    date: String,
    generated_at: DateTime<Utc>,
    archive_title: String,
    daily_diary_title: String,
    source: NotionArchiveSource,
    status: String,
    archive_markdown: String,
    brief: Option<DailyBrief>,
    five_hour_reports: Vec<InsightReport>,
    descriptive_stats: DailyActivityStats,
    hourly_metrics: Vec<HourlyActivityMetric>,
    comparison: DailyComparison,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotionArchiveSource {
    app: String,
    endpoint: String,
    local_date: String,
    timezone: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisStatus {
    visual: AnalysisWorkerStatus,
    report: AnalysisWorkerStatus,
    daily: AnalysisWorkerStatus,
    latest_observation: Option<VisualObservation>,
    latest_window_summary: Option<VisualWindowSummary>,
    latest_report: Option<InsightReport>,
    latest_daily_brief: Option<DailyBrief>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisWorkerStatus {
    status: String,
    last_started_at: Option<DateTime<Utc>>,
    last_finished_at: Option<DateTime<Utc>>,
    next_run_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

impl Default for AnalysisStatus {
    fn default() -> Self {
        Self {
            visual: AnalysisWorkerStatus::idle(),
            report: AnalysisWorkerStatus::idle(),
            daily: AnalysisWorkerStatus::idle(),
            latest_observation: None,
            latest_window_summary: None,
            latest_report: None,
            latest_daily_brief: None,
        }
    }
}

impl AnalysisWorkerStatus {
    fn idle() -> Self {
        Self {
            status: "idle".into(),
            last_started_at: None,
            last_finished_at: None,
            next_run_at: None,
            last_error: None,
        }
    }

    fn running(now: DateTime<Utc>) -> Self {
        Self {
            status: "running".into(),
            last_started_at: Some(now),
            last_finished_at: None,
            next_run_at: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisualAnalyzeResponse {
    summary: VisualSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InputEventsResponse {
    events: Vec<crate::models::InputEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextSegmentsResponse {
    segments: Vec<crate::models::TextSegment>,
}

const DEFAULT_SCREENSHOT_INTERVAL: u64 = 60;
const DEFAULT_IDLE_THRESHOLD: u64 = 120;
const DEFAULT_SCREENSHOT_LIMIT: usize = 1440;
const DEFAULT_HIGH_RES_SCREENSHOT_LIMIT: usize = 1440;
const DEFAULT_HIGH_RES_SCREENSHOT_INTERVAL: u64 = 60;
const THUMBNAIL_SCREENSHOT_MAX_WIDTH: u32 = 960;
const THUMBNAIL_SCREENSHOT_QUALITY: u8 = 82;
const HIGH_RES_SCREENSHOT_MAX_WIDTH: u32 = 1600;
const HIGH_RES_SCREENSHOT_QUALITY: u8 = 88;
const DEFAULT_IMAGE_RETENTION_DAYS: u32 = 30;
const IMAGE_RETENTION_SCAN_INTERVAL: u64 = 12 * 60 * 60;
const VISUAL_ANALYSIS_SCAN_INTERVAL: u64 = 30;
const INSIGHT_REPORT_INTERVAL: u64 = 5 * 60 * 60;
const INSIGHT_REPORT_CHECK_INTERVAL: u64 = 5 * 60;
const VISUAL_WINDOW_INTERVAL: i64 = 5 * 60;
const VISUAL_WINDOW_LOOKBACK_HOURS: i64 = 6;
const DAILY_BRIEF_CHECK_INTERVAL: u64 = 60;
const DEFAULT_DAILY_BRIEF_LOCAL_TIME: &str = "23:40";

#[derive(Debug, Clone)]
struct DateWindow {
    date: String,
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenshotCaptureKind {
    Thumbnail,
    HighRes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenshotCaptureProfile {
    directory: PathBuf,
    interval_secs: u64,
    max_width: u32,
    quality: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenshotCaptureRecord {
    captured_at: DateTime<Utc>,
    file_path: String,
    file_size_bytes: u64,
    width: u32,
    height: u32,
    process_name: Option<String>,
    window_title: Option<String>,
    capture_status: String,
}

pub fn router(store: Store, blocker_config_path: Option<PathBuf>) -> Router {
    router_from_state(default_state(store, blocker_config_path, None))
}

fn default_state(
    store: Store,
    blocker_config_path: Option<PathBuf>,
    shutdown_tx: Option<oneshot::Sender<()>>,
) -> AppState {
    let engine = blocker_config_path
        .as_deref()
        .and_then(|p| BlockerEngine::load(p).ok())
        .unwrap_or_else(BlockerEngine::empty);
    let now = Utc::now();
    AppState {
        store: Arc::new(Mutex::new(store)),
        blocker_engine: Arc::new(engine),
        screenshot_dir: Arc::new(PathBuf::from("data/screenshots")),
        screenshot_interval_secs: Arc::new(DEFAULT_SCREENSHOT_INTERVAL),
        high_res_screenshot_dir: Arc::new(PathBuf::from("data/high-res-screenshots")),
        high_res_screenshot_interval_secs: Arc::new(DEFAULT_HIGH_RES_SCREENSHOT_INTERVAL),
        idle_threshold_secs: Arc::new(DEFAULT_IDLE_THRESHOLD),
        health: Arc::new(Mutex::new(CollectorHealth {
            status: "ok".into(),
            started_at: now,
            uptime_seconds: 0,
            version: env!("CARGO_PKG_VERSION").into(),
            window_collector: SubsystemHealth {
                status: "not_started".into(),
                mode: Some("polling".into()),
                last_event_at: None,
                error_count: 0,
                last_error: None,
                last_capture_status: None,
                last_skip_reason: None,
            },
            input_collector: SubsystemHealth {
                status: "not_started".into(),
                mode: Some("raw_input".into()),
                last_event_at: None,
                error_count: 0,
                last_error: None,
                last_capture_status: None,
                last_skip_reason: None,
            },
            screenshot_collector: SubsystemHealth {
                status: "not_started".into(),
                mode: Some("interval_thumbnail".into()),
                last_event_at: None,
                error_count: 0,
                last_error: None,
                last_capture_status: None,
                last_skip_reason: None,
            },
            db_stats: DbStats::empty(DEFAULT_IMAGE_RETENTION_DAYS),
        })),
        analysis_status: Arc::new(Mutex::new(AnalysisStatus::default())),
        shutdown_tx: Arc::new(Mutex::new(shutdown_tx)),
    }
}

fn router_from_state(state: AppState) -> Router {
    let screenshot_dir = state.screenshot_dir.to_path_buf();
    let high_res_screenshot_dir = state.high_res_screenshot_dir.to_path_buf();
    Router::new()
        .route("/api/health", get(health))
        .route("/api/window-events", get(window_events))
        .route("/api/lifecycle-events", get(lifecycle_events))
        .route("/api/time-events", get(time_events))
        .route("/api/activity-buckets", get(activity_buckets))
        .route("/api/blockers", get(blockers))
        .route("/api/screenshots", get(screenshots))
        .route("/api/screenshot-summary", get(screenshot_summary))
        .route("/api/high-res-screenshots", get(high_res_screenshots))
        .route("/api/visual-summaries", get(visual_summaries))
        .route("/api/visual-observations", get(visual_observations))
        .route("/api/visual-window-summaries", get(visual_window_summaries))
        .route("/api/insight-reports", get(insight_reports))
        .route("/api/daily-brief", get(daily_brief))
        .route("/api/notion/daily-archive", get(notion_daily_archive))
        .route("/api/daily-brief/generate", post(generate_daily_brief))
        .route("/api/analysis-status", get(analysis_status))
        .route("/api/screenshots/{id}/analyze", post(analyze_screenshot))
        .route("/api/input-events", get(input_events))
        .route("/api/input-summary", get(input_summary))
        .route("/api/text-segments", get(text_segments))
        .route("/api/shutdown", post(shutdown))
        .nest_service("/screenshots", ServeDir::new(screenshot_dir))
        .nest_service(
            "/high-res-screenshots",
            ServeDir::new(high_res_screenshot_dir),
        )
        .with_state(state)
}

fn date_window_from_query(query: &DateQuery) -> std::result::Result<DateWindow, String> {
    let date = query
        .date
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let parsed = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| "date must use YYYY-MM-DD".to_string())?;
    let (start_utc, end_utc) = if let Some(offset_minutes) = query.tz_offset_minutes {
        fixed_offset_day_bounds(parsed, offset_minutes)?
    } else {
        local_day_bounds(parsed)?
    };
    Ok(DateWindow {
        date,
        start_utc,
        end_utc,
    })
}

fn date_window_for_local_date(date: NaiveDate) -> std::result::Result<DateWindow, String> {
    let (start_utc, end_utc) = local_day_bounds(date)?;
    Ok(DateWindow {
        date: date.format("%Y-%m-%d").to_string(),
        start_utc,
        end_utc,
    })
}

fn daily_brief_schedule_label() -> String {
    let value = std::env::var("DAILY_BRIEF_LOCAL_TIME")
        .unwrap_or_else(|_| DEFAULT_DAILY_BRIEF_LOCAL_TIME.to_string());
    if parse_daily_brief_time(&value).is_some() {
        value
    } else {
        DEFAULT_DAILY_BRIEF_LOCAL_TIME.to_string()
    }
}

fn parse_daily_brief_time(value: &str) -> Option<(u32, u32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour = hour.parse::<u32>().ok()?;
    let minute = minute.parse::<u32>().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}

fn daily_brief_due_now(now: DateTime<Local>, schedule_label: &str) -> bool {
    let Some(scheduled) = local_scheduled_at(now.date_naive(), schedule_label) else {
        return false;
    };
    now >= scheduled
}

fn next_daily_brief_run_at(now: DateTime<Utc>, schedule_label: &str) -> Option<DateTime<Utc>> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let today_run = local_scheduled_at(today, schedule_label)?;
    let next_local = if now_local < today_run {
        today_run
    } else {
        let tomorrow = today.succ_opt()?;
        local_scheduled_at(tomorrow, schedule_label)?
    };
    Some(next_local.with_timezone(&Utc))
}

fn local_scheduled_at(date: NaiveDate, schedule_label: &str) -> Option<DateTime<Local>> {
    let (hour, minute) = parse_daily_brief_time(schedule_label)?;
    let naive = date.and_hms_opt(hour, minute, 0)?;
    Local.from_local_datetime(&naive).single()
}

fn fixed_offset_day_bounds(
    date: NaiveDate,
    browser_offset_minutes: i32,
) -> std::result::Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let east_seconds = browser_offset_minutes
        .checked_mul(-60)
        .ok_or_else(|| "tzOffsetMinutes is out of range".to_string())?;
    let offset = FixedOffset::east_opt(east_seconds)
        .ok_or_else(|| "tzOffsetMinutes is out of range".to_string())?;
    let start = date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_local_timezone(offset)
        .single()
        .ok_or_else(|| "date cannot be resolved for tzOffsetMinutes".to_string())?;
    let end = date
        .succ_opt()
        .ok_or_else(|| "date is out of range".to_string())?
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_local_timezone(offset)
        .single()
        .ok_or_else(|| "date cannot be resolved for tzOffsetMinutes".to_string())?;
    Ok((start.with_timezone(&Utc), end.with_timezone(&Utc)))
}

fn local_day_bounds(
    date: NaiveDate,
) -> std::result::Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let start = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight is valid"))
        .earliest()
        .ok_or_else(|| "date cannot be resolved in the local timezone".to_string())?;
    let end_date = date
        .succ_opt()
        .ok_or_else(|| "date is out of range".to_string())?;
    let end = Local
        .from_local_datetime(&end_date.and_hms_opt(0, 0, 0).expect("midnight is valid"))
        .earliest()
        .ok_or_else(|| "date cannot be resolved in the local timezone".to_string())?;
    Ok((start.with_timezone(&Utc), end.with_timezone(&Utc)))
}

fn screenshot_capture_profile(
    state: &AppState,
    kind: ScreenshotCaptureKind,
) -> ScreenshotCaptureProfile {
    match kind {
        ScreenshotCaptureKind::Thumbnail => ScreenshotCaptureProfile {
            directory: state.screenshot_dir.to_path_buf(),
            interval_secs: *state.screenshot_interval_secs,
            max_width: THUMBNAIL_SCREENSHOT_MAX_WIDTH,
            quality: THUMBNAIL_SCREENSHOT_QUALITY,
        },
        ScreenshotCaptureKind::HighRes => ScreenshotCaptureProfile {
            directory: state.high_res_screenshot_dir.to_path_buf(),
            interval_secs: *state.high_res_screenshot_interval_secs,
            max_width: HIGH_RES_SCREENSHOT_MAX_WIDTH,
            quality: HIGH_RES_SCREENSHOT_QUALITY,
        },
    }
}

fn insert_screenshot_capture(
    store: &mut Store,
    kind: ScreenshotCaptureKind,
    session_id: &str,
    record: &ScreenshotCaptureRecord,
) -> Result<i64> {
    match kind {
        ScreenshotCaptureKind::Thumbnail => store.insert_screenshot_with_file_size(
            session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: record.captured_at,
                file_path: record.file_path.clone(),
                width: record.width,
                height: record.height,
                process_name: record.process_name.clone(),
                window_title: record.window_title.clone(),
                capture_status: record.capture_status.clone(),
            },
            record.file_size_bytes,
        ),
        ScreenshotCaptureKind::HighRes => store.insert_high_res_screenshot_with_file_size(
            session_id,
            &HighResScreenshotMeta {
                id: 0,
                captured_at: record.captured_at,
                file_path: record.file_path.clone(),
                width: record.width,
                height: record.height,
                process_name: record.process_name.clone(),
                window_title: record.window_title.clone(),
                capture_status: record.capture_status.clone(),
            },
            record.file_size_bytes,
        ),
    }
}

pub async fn serve(
    mut store: Store,
    addr: SocketAddr,
    poll_ms: u64,
    blocker_config_path: Option<PathBuf>,
) -> Result<()> {
    anyhow::ensure!(poll_ms >= 100, "poll_ms must be at least 100");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let now = Utc::now();
    store.close_stale_sessions(now, "abnormal_stop")?;
    let session_id = store.create_session(env!("CARGO_PKG_VERSION"), "default")?;
    store.insert_lifecycle_event(
        &session_id,
        now,
        LifecycleType::SessionStart,
        None,
        serde_json::json!({ "appVersion": env!("CARGO_PKG_VERSION") }),
    )?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = default_state(store, blocker_config_path, Some(shutdown_tx));

    let window_collector = spawn_collector_loop(state.clone(), session_id.clone(), poll_ms);
    let screenshot_collector = spawn_screenshot_loop(
        state.clone(),
        session_id.clone(),
        ScreenshotCaptureKind::Thumbnail,
    );
    let high_res_screenshot_collector = spawn_screenshot_loop(
        state.clone(),
        session_id.clone(),
        ScreenshotCaptureKind::HighRes,
    );
    let visual_analysis_collector = spawn_visual_analysis_loop(state.clone());
    let insight_report_collector = spawn_insight_report_loop(state.clone());
    let daily_brief_collector = spawn_daily_brief_loop(state.clone());
    let image_retention_collector = spawn_image_retention_loop(state.clone());
    let input_collector = input::spawn_input_collector(state.store.clone(), state.health.clone());

    let app = router_from_state(state.clone());
    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    window_collector.abort();
    screenshot_collector.abort();
    high_res_screenshot_collector.abort();
    visual_analysis_collector.abort();
    insight_report_collector.abort();
    daily_brief_collector.abort();
    image_retention_collector.abort();
    input_collector.abort();
    let _ = window_collector.await;
    let _ = screenshot_collector.await;
    let _ = high_res_screenshot_collector.await;
    let _ = visual_analysis_collector.await;
    let _ = insight_report_collector.await;
    let _ = daily_brief_collector.await;
    let _ = image_retention_collector.await;
    let _ = input_collector.await;

    if let Ok(mut store) = state.store.lock() {
        if let Err(err) = store.close_session(&session_id, Utc::now(), "service_stop") {
            eprintln!("session close failed: {err:#}");
        }
    }

    serve_result?;
    Ok(())
}

fn record_window_capture_success(
    state: &AppState,
    capture_status: &str,
    last_event_at: Option<DateTime<Utc>>,
) {
    if let Ok(mut h) = state.health.lock() {
        h.window_collector.last_capture_status = Some(capture_status.to_string());
        h.window_collector.error_count = 0;
        h.window_collector.last_error = None;
        if let Some(last_event_at) = last_event_at {
            h.window_collector.last_event_at = Some(last_event_at);
        }
    }
}

fn update_visual_analysis_running(state: &AppState, started_at: DateTime<Utc>) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.visual = AnalysisWorkerStatus::running(started_at);
    }
}

fn update_visual_analysis_success(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    window_summary: VisualWindowSummary,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.visual.status = "idle".into();
        status.visual.last_finished_at = Some(finished_at);
        status.visual.next_run_at = Some(next_run_at);
        status.visual.last_error = None;
        status.latest_window_summary = Some(window_summary);
    }
}

fn update_visual_analysis_error(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    error: String,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.visual.status = "error".into();
        status.visual.last_finished_at = Some(finished_at);
        status.visual.next_run_at = Some(next_run_at);
        status.visual.last_error = Some(error);
    }
}

fn update_report_running(state: &AppState, started_at: DateTime<Utc>) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.report = AnalysisWorkerStatus::running(started_at);
    }
}

fn update_report_success(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    report: InsightReport,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.report.status = "idle".into();
        status.report.last_finished_at = Some(finished_at);
        status.report.next_run_at = Some(next_run_at);
        status.report.last_error = None;
        status.latest_report = Some(report);
    }
}

fn update_report_error(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    error: String,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.report.status = "error".into();
        status.report.last_finished_at = Some(finished_at);
        status.report.next_run_at = Some(next_run_at);
        status.report.last_error = Some(error);
    }
}

fn update_daily_running(state: &AppState, started_at: DateTime<Utc>) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.daily = AnalysisWorkerStatus::running(started_at);
    }
}

fn update_daily_success(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    brief: DailyBrief,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.daily.status = "idle".into();
        status.daily.last_finished_at = Some(finished_at);
        status.daily.next_run_at = Some(next_run_at);
        status.daily.last_error = None;
        status.latest_daily_brief = Some(brief);
    }
}

fn update_daily_idle(state: &AppState, next_run_at: DateTime<Utc>) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.daily.status = "idle".into();
        status.daily.next_run_at = Some(next_run_at);
    }
}

fn update_daily_error(
    state: &AppState,
    finished_at: DateTime<Utc>,
    next_run_at: DateTime<Utc>,
    error: String,
) {
    if let Ok(mut status) = state.analysis_status.lock() {
        status.daily.status = "error".into();
        status.daily.last_finished_at = Some(finished_at);
        status.daily.next_run_at = Some(next_run_at);
        status.daily.last_error = Some(error);
    }
}

fn should_record_capture_unavailable(last_error: &mut Option<String>, error: &str) -> bool {
    if last_error.as_deref() == Some(error) {
        return false;
    }
    *last_error = Some(error.to_string());
    true
}

fn spawn_collector_loop(
    state: AppState,
    session_id: String,
    poll_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        {
            if let Ok(mut h) = state.health.lock() {
                h.window_collector.status = "running".into();
            }
        }
        let mut last_identity: Option<(i64, u32, Option<String>)> = None;
        let mut last_capture_unavailable_error: Option<String> = None;
        loop {
            match sample_foreground_window() {
                Ok(snapshot) => {
                    last_capture_unavailable_error = None;
                    record_window_capture_success(&state, snapshot.capture_status.as_str(), None);
                    let identity = (snapshot.hwnd, snapshot.pid, snapshot.window_title.clone());
                    if last_identity.as_ref() != Some(&identity) {
                        if let Ok(mut store) = state.store.lock() {
                            match store.insert_window_focus(&session_id, &snapshot) {
                                Ok(_) => {
                                    last_identity = Some(identity);
                                    record_window_capture_success(
                                        &state,
                                        snapshot.capture_status.as_str(),
                                        Some(Utc::now()),
                                    );
                                }
                                Err(err) => {
                                    eprintln!("window event write failed: {err:#}");
                                    if let Ok(mut h) = state.health.lock() {
                                        h.window_collector.error_count += 1;
                                        h.window_collector.last_error = Some(format!("{err:#}"));
                                    }
                                }
                            }
                        } else {
                            eprintln!("window event write failed: store lock poisoned");
                            if let Ok(mut h) = state.health.lock() {
                                h.window_collector.error_count += 1;
                                h.window_collector.last_error = Some("store lock poisoned".into());
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("window sample failed: {err:#}");
                    let error = format!("{err:#}");
                    if should_record_capture_unavailable(
                        &mut last_capture_unavailable_error,
                        &error,
                    ) {
                        if let Ok(mut store) = state.store.lock() {
                            if let Err(write_err) = store.insert_lifecycle_event(
                                &session_id,
                                Utc::now(),
                                LifecycleType::CaptureUnavailable,
                                Some("window_sample_failed"),
                                serde_json::json!({ "error": error.clone() }),
                            ) {
                                eprintln!("window sample lifecycle write failed: {write_err:#}");
                            }
                        }
                    }
                    if let Ok(mut h) = state.health.lock() {
                        h.window_collector.error_count += 1;
                        h.window_collector.last_error = Some(error);
                        h.window_collector.last_capture_status = Some("capture_unavailable".into());
                    }
                }
            }

            time::sleep(Duration::from_millis(poll_ms)).await;
        }
    })
}

fn screenshot_skip_metadata(
    reason: &str,
    snapshot: Option<&WindowSnapshot>,
) -> (Option<String>, Option<String>) {
    if reason == "blocked" {
        return (None, None);
    }

    (
        snapshot.map(|s| s.process_name.clone()),
        snapshot.and_then(|s| s.window_title.clone()),
    )
}

fn record_screenshot_skip(
    state: &AppState,
    session_id: &str,
    reason: &str,
    snapshot: Option<&WindowSnapshot>,
    kind: ScreenshotCaptureKind,
) {
    let now = Utc::now();
    let (process_name, window_title) = screenshot_skip_metadata(reason, snapshot);
    let metadata_error = match state.store.lock() {
        Ok(mut store) => insert_screenshot_capture(
            &mut store,
            kind,
            session_id,
            &ScreenshotCaptureRecord {
                captured_at: now,
                file_path: String::new(),
                file_size_bytes: 0,
                width: 0,
                height: 0,
                process_name,
                window_title,
                capture_status: reason.to_string(),
            },
        )
        .err()
        .map(|err| {
            eprintln!("screenshot skip metadata write failed: {err:#}");
            format!("{err:#}")
        }),
        Err(_) => {
            eprintln!("screenshot skip metadata write failed: store lock poisoned");
            Some("store lock poisoned".into())
        }
    };

    if let Ok(mut h) = state.health.lock() {
        h.screenshot_collector.last_event_at = Some(now);
        h.screenshot_collector.last_skip_reason = Some(reason.to_string());
        if let Some(error) = metadata_error {
            h.screenshot_collector.error_count += 1;
            h.screenshot_collector.last_error = Some(error);
        }
    }
}

fn spawn_screenshot_loop(
    state: AppState,
    session_id: String,
    kind: ScreenshotCaptureKind,
) -> tokio::task::JoinHandle<()> {
    let profile = screenshot_capture_profile(&state, kind);
    let interval = profile.interval_secs;
    let idle_threshold = *state.idle_threshold_secs;
    let screenshot_dir = profile.directory.clone();

    tokio::spawn(async move {
        {
            if let Ok(mut h) = state.health.lock() {
                h.screenshot_collector.status = "running".into();
            }
        }
        loop {
            time::sleep(Duration::from_secs(interval)).await;

            if screenshot::idle_seconds() > idle_threshold as f64 {
                record_screenshot_skip(&state, &session_id, "idle", None, kind);
                continue;
            }

            let snapshot = match sample_foreground_window() {
                Ok(s) => s,
                Err(_) => {
                    record_screenshot_skip(&state, &session_id, "capture_unavailable", None, kind);
                    continue;
                }
            };

            if state.blocker_engine.is_blocked("screenshot", &snapshot) {
                for rule in state.blocker_engine.matching_rules("screenshot", &snapshot) {
                    if let Ok(mut store) = state.store.lock() {
                        let _ = store.insert_blocker_hit(&BlockerHit {
                            id: 0,
                            hit_at: Utc::now(),
                            capture_type: "screenshot".to_string(),
                            field: rule.field.clone(),
                            operator: rule.operator.clone(),
                            rule_value: rule.value.clone(),
                            actual_value: match rule.field.as_str() {
                                "process_name" => snapshot.process_name.clone(),
                                "window_title" => snapshot.window_title.clone().unwrap_or_default(),
                                _ => String::new(),
                            },
                        });
                    }
                }
                record_screenshot_skip(&state, &session_id, "blocked", Some(&snapshot), kind);
                continue;
            }

            let (bytes, w, h) =
                match screenshot::capture_thumbnail(profile.max_width, profile.quality) {
                    Some(data) => data,
                    None => {
                        if let Ok(mut h) = state.health.lock() {
                            h.screenshot_collector.error_count += 1;
                            h.screenshot_collector.last_error =
                                Some("capture_thumbnail returned None".into());
                        }
                        record_screenshot_skip(
                            &state,
                            &session_id,
                            "capture_failed",
                            Some(&snapshot),
                            kind,
                        );
                        continue;
                    }
                };

            let now = Utc::now();
            let date_dir = now.format("%Y-%m-%d").to_string();
            let filename = match kind {
                ScreenshotCaptureKind::Thumbnail => format!("{}.jpg", now.format("%H-%M")),
                ScreenshotCaptureKind::HighRes => format!("{}.jpg", now.format("%H-%M-%S")),
            };
            let dir = screenshot_dir.join(&date_dir);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                eprintln!("screenshot dir create failed: {e:#}");
                if let Ok(mut h) = state.health.lock() {
                    h.screenshot_collector.error_count += 1;
                    h.screenshot_collector.last_error = Some(format!("{e:#}"));
                }
                record_screenshot_skip(&state, &session_id, "write_failed", Some(&snapshot), kind);
                continue;
            }
            let filepath = dir.join(&filename);

            if let Err(e) = std::fs::write(&filepath, &bytes) {
                eprintln!("screenshot write failed: {e:#}");
                if let Ok(mut h) = state.health.lock() {
                    h.screenshot_collector.error_count += 1;
                    h.screenshot_collector.last_error = Some(format!("{e:#}"));
                }
                record_screenshot_skip(&state, &session_id, "write_failed", Some(&snapshot), kind);
                continue;
            }

            let relative_path = format!("{}/{}", date_dir, filename);

            let metadata_write_result = match state.store.lock() {
                Ok(mut store) => insert_screenshot_capture(
                    &mut store,
                    kind,
                    &session_id,
                    &ScreenshotCaptureRecord {
                        captured_at: now,
                        file_path: relative_path,
                        file_size_bytes: bytes.len() as u64,
                        width: w,
                        height: h,
                        process_name: Some(snapshot.process_name.clone()),
                        window_title: snapshot.window_title.clone(),
                        capture_status: "ok".to_string(),
                    },
                )
                .map(|_| ())
                .map_err(|err| format!("{err:#}")),
                Err(_) => Err("store lock poisoned".into()),
            };

            match metadata_write_result {
                Ok(_) => {
                    if let Ok(mut h) = state.health.lock() {
                        h.screenshot_collector.last_event_at = Some(Utc::now());
                        h.screenshot_collector.error_count = 0;
                        h.screenshot_collector.last_error = None;
                    }
                }
                Err(error) => {
                    eprintln!("screenshot metadata write failed: {error}");
                    if let Ok(mut h) = state.health.lock() {
                        h.screenshot_collector.error_count += 1;
                        h.screenshot_collector.last_error = Some(error);
                    }
                    record_screenshot_skip(
                        &state,
                        &session_id,
                        "metadata_write_failed",
                        Some(&snapshot),
                        kind,
                    );
                }
            }
        }
    })
}

fn spawn_image_retention_loop(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let cleanup_result = match state.store.lock() {
                Ok(mut store) => cleanup_expired_images(
                    &mut store,
                    &state.screenshot_dir,
                    &state.high_res_screenshot_dir,
                    Utc::now(),
                    ImageRetentionPolicy {
                        retention_days: DEFAULT_IMAGE_RETENTION_DAYS,
                    },
                ),
                Err(_) => Err(anyhow::anyhow!("store lock poisoned")),
            };

            match cleanup_result {
                Ok(result) => {
                    if result.deleted_files > 0 {
                        eprintln!(
                            "image retention expired {} files, {} bytes",
                            result.deleted_files, result.deleted_bytes
                        );
                    }
                    if result.failed_files > 0 {
                        if let Ok(mut h) = state.health.lock() {
                            h.screenshot_collector.error_count += 1;
                            h.screenshot_collector.last_error = Some(format!(
                                "image retention failed for {} files",
                                result.failed_files
                            ));
                        }
                    }
                }
                Err(err) => {
                    eprintln!("image retention cleanup failed: {err:#}");
                    if let Ok(mut h) = state.health.lock() {
                        h.screenshot_collector.error_count += 1;
                        h.screenshot_collector.last_error = Some(format!("{err:#}"));
                    }
                }
            }

            time::sleep(Duration::from_secs(IMAGE_RETENTION_SCAN_INTERVAL)).await;
        }
    })
}

fn spawn_visual_analysis_loop(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let started_at = Utc::now();
            let next_run_at =
                started_at + chrono::Duration::seconds(VISUAL_ANALYSIS_SCAN_INTERVAL as i64);
            update_visual_analysis_running(&state, started_at);
            match process_next_visual_window_summary(&state).await {
                Ok(Some(window_summary)) => {
                    update_visual_analysis_success(&state, Utc::now(), next_run_at, window_summary);
                }
                Ok(None) => {
                    if let Ok(mut status) = state.analysis_status.lock() {
                        status.visual.status = "idle".into();
                        status.visual.next_run_at = Some(next_run_at);
                    }
                }
                Err(error) => {
                    update_visual_analysis_error(
                        &state,
                        Utc::now(),
                        next_run_at,
                        format!("{error:#}"),
                    );
                }
            }

            time::sleep(Duration::from_secs(VISUAL_ANALYSIS_SCAN_INTERVAL)).await;
        }
    })
}

async fn process_next_visual_window_summary(
    state: &AppState,
) -> Result<Option<VisualWindowSummary>> {
    let pending_window = {
        let store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        find_next_pending_visual_window(&store, Utc::now())?
    };
    let Some(pending_window) = pending_window else {
        return Ok(None);
    };

    let image_paths = pending_window
        .samples
        .iter()
        .map(|sample| {
            state
                .high_res_screenshot_dir
                .join(&sample.screenshot.file_path)
        })
        .collect::<Vec<_>>();
    let analysis_samples = pending_window
        .samples
        .iter()
        .zip(image_paths.iter())
        .map(|(sample, image_path)| WindowVisualAnalysisSample {
            minute_mark: sample.minute_mark,
            screenshot: &sample.screenshot,
            image_path: image_path.as_path(),
        })
        .collect::<Vec<_>>();
    let input = WindowVisualAnalysisInput {
        window_start: pending_window.window_start,
        window_end: pending_window.window_end,
        samples: analysis_samples,
        previous_summary: pending_window.previous_summary.as_ref(),
    };
    let created_at = Utc::now();
    let mut summary = ConfiguredVisualAnalyzer::from_env()?
        .analyze_window(&input, created_at)
        .await
        .with_context(|| {
            format!(
                "visual window analysis failed for {}..{}",
                pending_window.window_start.to_rfc3339(),
                pending_window.window_end.to_rfc3339()
            )
        })?;

    let summary_id = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        store.insert_visual_window_summary(&summary)?
    };
    summary.id = summary_id;
    Ok(Some(summary))
}

#[derive(Debug)]
struct PendingVisualWindow {
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    samples: Vec<WindowScreenshotSample>,
    previous_summary: Option<VisualWindowSummary>,
}

fn find_next_pending_visual_window(
    store: &Store,
    now: DateTime<Utc>,
) -> Result<Option<PendingVisualWindow>> {
    let latest_summary = store.list_visual_window_summaries(1)?.into_iter().next();
    let mut window_start = latest_summary
        .as_ref()
        .map(|summary| summary.window_end)
        .unwrap_or_else(|| now - chrono::Duration::hours(VISUAL_WINDOW_LOOKBACK_HOURS));
    window_start = floor_to_visual_window(window_start);
    let latest_complete_end = floor_to_visual_window(now);

    while window_start + chrono::Duration::seconds(VISUAL_WINDOW_INTERVAL) <= latest_complete_end {
        let window_end = window_start + chrono::Duration::seconds(VISUAL_WINDOW_INTERVAL);
        let already_summarized = !store
            .list_visual_window_summaries_between(window_start, window_end, 1)?
            .is_empty();
        if !already_summarized {
            let screenshots =
                store.list_high_res_screenshots_between(window_start, window_end, 20)?;
            if let Ok(samples) = select_window_samples(window_start, window_end, &screenshots) {
                let previous_summary = store.latest_visual_window_summary_before(window_start)?;
                return Ok(Some(PendingVisualWindow {
                    window_start,
                    window_end,
                    samples,
                    previous_summary,
                }));
            }
        }
        window_start = window_end;
    }

    Ok(None)
}

fn floor_to_visual_window(value: DateTime<Utc>) -> DateTime<Utc> {
    let timestamp = value.timestamp();
    let floored = timestamp - timestamp.rem_euclid(VISUAL_WINDOW_INTERVAL);
    Utc.timestamp_opt(floored, 0)
        .single()
        .expect("floored timestamp must be valid")
}

fn spawn_insight_report_loop(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let started_at = Utc::now();
            let next_run_at =
                started_at + chrono::Duration::seconds(INSIGHT_REPORT_CHECK_INTERVAL as i64);
            update_report_running(&state, started_at);
            match maybe_generate_insight_report(&state, started_at).await {
                Ok(Some(report)) => {
                    let next_report_at = report.period_end
                        + chrono::Duration::seconds(INSIGHT_REPORT_INTERVAL as i64);
                    update_report_success(&state, Utc::now(), next_report_at, report);
                }
                Ok(None) => {
                    if let Ok(mut status) = state.analysis_status.lock() {
                        status.report.status = "idle".into();
                        status.report.next_run_at = Some(next_run_at);
                    }
                }
                Err(error) => {
                    update_report_error(&state, Utc::now(), next_run_at, format!("{error:#}"));
                }
            }

            time::sleep(Duration::from_secs(INSIGHT_REPORT_CHECK_INTERVAL)).await;
        }
    })
}

async fn maybe_generate_insight_report(
    state: &AppState,
    period_end: DateTime<Utc>,
) -> Result<Option<InsightReport>> {
    let period_start = period_end - chrono::Duration::seconds(INSIGHT_REPORT_INTERVAL as i64);
    let window_summaries = {
        let store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        if let Some(latest) = store.list_insight_reports(1)?.into_iter().next() {
            let next_due =
                latest.period_end + chrono::Duration::seconds(INSIGHT_REPORT_INTERVAL as i64);
            if period_end < next_due {
                return Ok(None);
            }
        }

        store
            .list_visual_window_summaries_between(period_start, period_end, 1000)?
            .into_iter()
            .filter(|summary| summary.error.is_none())
            .collect::<Vec<_>>()
    };
    if window_summaries.is_empty() {
        return Ok(None);
    }

    let mut report = ConfiguredInsightReporter::from_env()?
        .report_from_window_summaries(period_start, period_end, &window_summaries)
        .await?;
    let report_id = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        store.insert_insight_report(&report)?
    };
    report.id = report_id;
    Ok(Some(report))
}

fn spawn_daily_brief_loop(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let started_at = Utc::now();
            let schedule_label = daily_brief_schedule_label();
            let next_run_at =
                next_daily_brief_run_at(started_at, &schedule_label).unwrap_or_else(|| {
                    started_at + chrono::Duration::seconds(DAILY_BRIEF_CHECK_INTERVAL as i64)
                });
            update_daily_running(&state, started_at);

            if daily_brief_due_now(Local::now(), &schedule_label) {
                let local_date = Local::now().date_naive();
                let generation_result = match date_window_for_local_date(local_date) {
                    Ok(date_window) => {
                        async_maybe_generate_daily_brief(&state, date_window, &schedule_label).await
                    }
                    Err(message) => Err(anyhow::anyhow!(message)),
                };
                match generation_result {
                    Ok(Some(brief)) => {
                        let next = next_daily_brief_run_at(Utc::now(), &schedule_label)
                            .unwrap_or(next_run_at);
                        update_daily_success(&state, Utc::now(), next, brief);
                    }
                    Ok(None) => update_daily_idle(&state, next_run_at),
                    Err(error) => {
                        update_daily_error(&state, Utc::now(), next_run_at, format!("{error:#}"));
                    }
                }
            } else {
                update_daily_idle(&state, next_run_at);
            }

            time::sleep(Duration::from_secs(DAILY_BRIEF_CHECK_INTERVAL)).await;
        }
    })
}

async fn async_maybe_generate_daily_brief(
    state: &AppState,
    date_window: DateWindow,
    scheduled_for_local: &str,
) -> Result<Option<DailyBrief>> {
    let (reports, stats, hourly_metrics, comparison) = {
        let store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        if store.daily_brief_exists(&date_window.date, scheduled_for_local)? {
            return Ok(store.get_daily_brief_by_date(&date_window.date, scheduled_for_local)?);
        }
        let reports = store.list_insight_reports_between(
            date_window.start_utc,
            date_window.end_utc,
            Some("5h"),
            100,
        )?;
        let stats = store.build_daily_activity_stats(
            &date_window.date,
            date_window.start_utc,
            date_window.end_utc,
            &reports,
        )?;
        let hourly_metrics = store.build_hourly_activity_metrics(
            date_window.start_utc,
            date_window.end_utc,
            &reports,
        )?;
        let comparison = store.build_daily_comparison(&date_window.date, &stats)?;
        (reports, stats, hourly_metrics, comparison)
    };

    let report_result = ConfiguredDailyBriefReporter::from_env()?
        .report(
            &date_window.date,
            date_window.start_utc,
            date_window.end_utc,
            scheduled_for_local,
            &stats,
            &hourly_metrics,
            &comparison,
            &reports,
        )
        .await;
    let mut brief = match report_result {
        Ok(brief) => brief,
        Err(error) => {
            let mut fallback = LocalDailyBriefReporter.report(
                &date_window.date,
                date_window.start_utc,
                date_window.end_utc,
                scheduled_for_local,
                &stats,
                &hourly_metrics,
                &comparison,
                &reports,
                Utc::now(),
            )?;
            fallback.status = "error".into();
            fallback.error = Some(format!("{error:#}"));
            let mut store = state
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
            let id = store.upsert_daily_brief_error(fallback.clone(), format!("{error:#}"))?;
            fallback.id = id;
            return Err(error);
        }
    };
    let id = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        store.insert_daily_brief(&brief)?
    };
    brief.id = id;
    Ok(Some(brief))
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let health_snapshot = match state.health.lock() {
        Ok(h) => h.clone(),
        Err(_) => return internal_error("health lock poisoned"),
    };

    let uptime = (Utc::now() - health_snapshot.started_at)
        .num_seconds()
        .max(0) as u64;

    let overall = if health_snapshot.window_collector.status == "error"
        || health_snapshot.input_collector.status == "error"
        || health_snapshot.screenshot_collector.status == "error"
    {
        "error"
    } else if health_snapshot.window_collector.status == "not_started"
        || health_snapshot.input_collector.status == "not_started"
        || health_snapshot.screenshot_collector.status == "not_started"
    {
        "degraded"
    } else {
        "ok"
    };

    let db_stats = match state.store.lock() {
        Ok(store) => match store.get_db_stats_with_retention(DEFAULT_IMAGE_RETENTION_DAYS) {
            Ok(stats) => stats,
            Err(_) => DbStats::empty(DEFAULT_IMAGE_RETENTION_DAYS),
        },
        Err(_) => DbStats::empty(DEFAULT_IMAGE_RETENTION_DAYS),
    };

    Json(CollectorHealth {
        status: overall.into(),
        uptime_seconds: uptime,
        db_stats,
        ..health_snapshot
    })
    .into_response()
}

async fn window_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_window_events(limit) {
        Ok(events) => Json(events).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn time_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let context_limit = limit.saturating_mul(2).min(10_000);
    let store = state.store.clone();
    let mut events = match tokio::task::spawn_blocking(move || -> Result<Vec<TimeEvent>> {
        let (window_events, lifecycle_events) = {
            let store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
            let window_events = store.list_window_events(context_limit)?;
            let lifecycle_events = store.list_lifecycle_events(context_limit)?;
            (window_events, lifecycle_events)
        };

        Ok(build_time_events_with_lifecycle(
            &window_events,
            &lifecycle_events,
        ))
    })
    .await
    {
        Ok(Ok(events)) => events,
        Ok(Err(err)) => return internal_error(err),
        Err(err) => return internal_error(err),
    };
    if events.len() > limit {
        events = events.split_off(events.len() - limit);
    }

    Json(TimeEventsResponse { events }).into_response()
}

async fn activity_buckets(
    State(state): State<AppState>,
    Query(query): Query<ActivityBucketsQuery>,
) -> impl IntoResponse {
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_err() {
        return bad_request("date must use YYYY-MM-DD");
    }
    let bucket_seconds = query.bucket_seconds.unwrap_or(180);
    if !(60..=3600).contains(&bucket_seconds) {
        return bad_request("bucketSeconds must be between 60 and 3600");
    }

    let limit = query.limit.unwrap_or(10_000).min(50_000);
    let activity_query = ActivityBucketQuery {
        date: date.clone(),
        bucket_seconds,
    };
    let store = state.store.clone();
    let buckets = match tokio::task::spawn_blocking(move || -> Result<Vec<ActivityBucket>> {
        let (window_events, lifecycle_events) = {
            let store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
            let window_events = store.list_window_events(limit)?;
            let lifecycle_events = store.list_lifecycle_events(limit)?;
            (window_events, lifecycle_events)
        };
        let events = build_time_events_with_lifecycle(&window_events, &lifecycle_events);

        Ok(build_activity_buckets(&events, activity_query))
    })
    .await
    {
        Ok(Ok(buckets)) => buckets,
        Ok(Err(err)) => return internal_error(err),
        Err(err) => return internal_error(err),
    };

    Json(ActivityBucketsResponse {
        date,
        bucket_seconds,
        buckets,
    })
    .into_response()
}

async fn lifecycle_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_lifecycle_events(limit) {
        Ok(events) => Json(LifecycleEventsResponse { events }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn blockers(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    let rules = state.blocker_engine.rules().to_vec();
    match store.list_blocker_hits(limit) {
        Ok(hits) => Json(BlockersResponse { rules, hits }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn screenshots(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let limit = query.limit.unwrap_or(DEFAULT_SCREENSHOT_LIMIT).min(5000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_screenshots_between(date_window.start_utc, date_window.end_utc, limit) {
        Ok(screenshots) => Json(ScreenshotsResponse { screenshots }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn screenshot_summary(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.get_screenshot_summary_between(
        &date_window.date,
        date_window.start_utc,
        date_window.end_utc,
    ) {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn high_res_screenshots(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let limit = query
        .limit
        .unwrap_or(DEFAULT_HIGH_RES_SCREENSHOT_LIMIT)
        .min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_high_res_screenshots_between(date_window.start_utc, date_window.end_utc, limit)
    {
        Ok(screenshots) => Json(HighResScreenshotsResponse { screenshots }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn visual_summaries(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_visual_summaries_between(date_window.start_utc, date_window.end_utc, limit) {
        Ok(summaries) => Json(VisualSummariesResponse { summaries }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn visual_observations(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_visual_observations_between(date_window.start_utc, date_window.end_utc, limit)
    {
        Ok(observations) => Json(VisualObservationsResponse { observations }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn visual_window_summaries(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_visual_window_summaries_between(
        date_window.start_utc,
        date_window.end_utc,
        limit,
    ) {
        Ok(summaries) => Json(VisualWindowSummariesResponse { summaries }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn insight_reports(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).min(100);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    if query.date.is_some() {
        let date_window = match date_window_from_query(&query) {
            Ok(date_window) => date_window,
            Err(message) => return bad_request(&message),
        };
        match store.list_insight_reports_between(
            date_window.start_utc,
            date_window.end_utc,
            query.kind.as_deref().or(Some("5h")),
            limit,
        ) {
            Ok(reports) => Json(InsightReportsResponse { reports }).into_response(),
            Err(err) => internal_error(err),
        }
    } else {
        match store.list_insight_reports(limit) {
            Ok(reports) => Json(InsightReportsResponse { reports }).into_response(),
            Err(err) => internal_error(err),
        }
    }
}

async fn daily_brief(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    match build_daily_brief_response(&state, date_window) {
        Ok(response) => Json(response).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn generate_daily_brief(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    let schedule_label = daily_brief_schedule_label();
    match async_maybe_generate_daily_brief(&state, date_window.clone(), &schedule_label).await {
        Ok(Some(brief)) => {
            update_daily_success(
                &state,
                Utc::now(),
                next_daily_brief_run_at(Utc::now(), &schedule_label).unwrap_or(Utc::now()),
                brief,
            );
            match build_daily_brief_response(&state, date_window) {
                Ok(response) => Json(response).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => match build_daily_brief_response(&state, date_window) {
            Ok(response) => Json(response).into_response(),
            Err(err) => internal_error(err),
        },
        Err(err) => internal_error(err),
    }
}

async fn notion_daily_archive(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    match build_notion_daily_archive_response(&state, date_window) {
        Ok(response) => Json(response).into_response(),
        Err(err) => internal_error(err),
    }
}

fn build_daily_brief_response(
    state: &AppState,
    date_window: DateWindow,
) -> Result<DailyBriefResponse> {
    let schedule_label = daily_brief_schedule_label();
    let store = state
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
    let reports = store.list_insight_reports_between(
        date_window.start_utc,
        date_window.end_utc,
        Some("5h"),
        100,
    )?;
    let brief = store.get_daily_brief_by_date(&date_window.date, &schedule_label)?;
    let stats = if let Some(brief) = &brief {
        brief.descriptive_stats.clone()
    } else {
        store.build_daily_activity_stats(
            &date_window.date,
            date_window.start_utc,
            date_window.end_utc,
            &reports,
        )?
    };
    let hourly_metrics = if let Some(brief) = &brief {
        brief.hourly_metrics.clone()
    } else {
        store.build_hourly_activity_metrics(date_window.start_utc, date_window.end_utc, &reports)?
    };
    let comparison = if let Some(brief) = &brief {
        brief.comparison.clone()
    } else {
        store.build_daily_comparison(&date_window.date, &stats)?
    };
    let status = brief
        .as_ref()
        .map(|brief| brief.status.clone())
        .unwrap_or_else(|| "missing".into());
    Ok(DailyBriefResponse {
        date: date_window.date,
        status,
        next_run_at: next_daily_brief_run_at(Utc::now(), &schedule_label),
        brief,
        five_hour_reports: reports,
        descriptive_stats: stats,
        hourly_metrics,
        comparison,
    })
}

fn build_notion_daily_archive_response(
    state: &AppState,
    date_window: DateWindow,
) -> Result<NotionDailyArchiveResponse> {
    let response = build_daily_brief_response(state, date_window)?;
    let daily_diary_title = daily_diary_title(&response.date);
    let archive_title = format!("Time State Recorder Daily Archive | {}", response.date);
    let archive_markdown = render_notion_archive_markdown(
        &response.date,
        &archive_title,
        &daily_diary_title,
        response.brief.as_ref(),
        &response.five_hour_reports,
        &response.descriptive_stats,
        &response.hourly_metrics,
        &response.comparison,
    );

    Ok(NotionDailyArchiveResponse {
        date: response.date.clone(),
        generated_at: Utc::now(),
        archive_title,
        daily_diary_title,
        source: NotionArchiveSource {
            app: "time-state-recorder".into(),
            endpoint: "/api/notion/daily-archive".into(),
            local_date: response.date.clone(),
            timezone: "query-local-date".into(),
        },
        status: response.status,
        archive_markdown,
        brief: response.brief,
        five_hour_reports: response.five_hour_reports,
        descriptive_stats: response.descriptive_stats,
        hourly_metrics: response.hourly_metrics,
        comparison: response.comparison,
    })
}

fn daily_diary_title(date: &str) -> String {
    format!("INDEX-{} | Daily Diary", date.replace('-', ""))
}

fn render_notion_archive_markdown(
    date: &str,
    archive_title: &str,
    daily_diary_title: &str,
    brief: Option<&DailyBrief>,
    reports: &[InsightReport],
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    comparison: &DailyComparison,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {archive_title}"));
    lines.push(format!("Daily Diary: {daily_diary_title}"));
    lines.push(format!("Source date: {date}"));
    lines.push(String::new());
    lines.push("## Daily Summary".into());
    if let Some(brief) = brief {
        lines.push(brief.daily_summary_text.clone());
        lines.push(format!("Action trajectory: {}", brief.action_trajectory));
    } else {
        lines.push(
            "No generated daily brief was found; this archive uses descriptive stats and 5-hour reports only."
                .into(),
        );
    }
    lines.push(String::new());
    lines.push("## Descriptive Statistics".into());
    lines.push(format!("- Active time: {:.2} hours", stats.active_hours));
    lines.push(format!("- Window switches: {}", stats.switch_count));
    lines.push(format!("- Distinct apps: {}", stats.distinct_app_count));
    lines.push(format!(
        "- Input: {} chars across {} events",
        stats.input_chars, stats.input_events
    ));
    lines.push(format!(
        "- Visual evidence: {} visual windows, {} high-res screenshots",
        stats.visual_window_count, stats.high_res_screenshot_count
    ));
    lines.push(format!(
        "- First activity: {}",
        optional_time(stats.first_activity_at)
    ));
    lines.push(format!(
        "- Last activity: {}",
        optional_time(stats.last_activity_at)
    ));
    lines.push(format!("- Top apps: {}", top_apps_text(&stats.top_apps)));
    lines.push(String::new());
    lines.push("## Parallel Projects And Time Allocation".into());
    lines.extend(project_lines_from_reports(reports));
    lines.push(String::new());
    lines.push("## Workflow Pattern".into());
    lines.extend(hourly_lines(hourly_metrics));
    lines.push(String::new());
    lines.push("## Five-Hour Reports".into());
    lines.extend(report_lines(reports));
    lines.push(String::new());
    lines.push("## Comparison".into());
    lines.push(comparison.explanation.clone());
    lines.push(format!(
        "- Active time delta: {} seconds",
        comparison.active_seconds_delta
    ));
    lines.push(format!(
        "- Switches/hour delta: {:.2}",
        comparison.switches_per_hour_delta
    ));
    lines.push(format!(
        "- Input chars delta: {}",
        comparison.input_chars_delta
    ));
    lines.push(String::new());
    lines.push("## Uncertainty And Review Notes".into());
    lines.push(
        "- This archive is generated from local desktop activity records, screenshots, model summaries, and daily aggregation stats."
            .into(),
    );
    lines.push(
        "- It may miss off-screen work, offline activity, or model interpretation errors; keep diary-level review available."
            .into(),
    );
    lines.join("\n")
}

fn optional_time(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|time| time.to_rfc3339())
        .unwrap_or_else(|| "unknown".into())
}

fn top_apps_text(apps: &[DailyAppActivity]) -> String {
    if apps.is_empty() {
        return "unknown".into();
    }
    apps.iter()
        .take(5)
        .map(|app| format!("{} {:.0}%", app.process_name, app.share * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn project_lines_from_reports(reports: &[InsightReport]) -> Vec<String> {
    if reports.is_empty() {
        return vec!["- No 5-hour reports found for this date.".into()];
    }
    reports
        .iter()
        .map(|report| {
            let projects = if report.project_hints.is_empty() {
                "Unknown project".into()
            } else {
                report.project_hints.join(", ")
            };
            format!(
                "- {} to {}: {} ({})",
                report.period_start.to_rfc3339(),
                report.period_end.to_rfc3339(),
                projects,
                category_mix_text(&report.category_mix)
            )
        })
        .collect()
}

fn hourly_lines(metrics: &[HourlyActivityMetric]) -> Vec<String> {
    let active = metrics
        .iter()
        .filter(|metric| metric.active_seconds > 0)
        .collect::<Vec<_>>();
    if active.is_empty() {
        return vec!["- No active hourly metrics for this date.".into()];
    }
    active
        .into_iter()
        .map(|metric| {
            format!(
                "- {:02}:00: {:.1} active minutes, dominant app {}, category {}, reports {:?}",
                metric.hour,
                metric.active_seconds as f64 / 60.0,
                metric.dominant_app.as_deref().unwrap_or("unknown"),
                metric.dominant_category.as_str(),
                metric.five_hour_report_ids
            )
        })
        .collect()
}

fn report_lines(reports: &[InsightReport]) -> Vec<String> {
    if reports.is_empty() {
        return vec!["- No 5-hour reports available.".into()];
    }
    reports
        .iter()
        .map(|report| {
            let projects = if report.project_hints.is_empty() {
                "unknown".into()
            } else {
                report.project_hints.join(", ")
            };
            format!(
                "- {} to {} | evidence {} | projects {} | {}",
                report.period_start.to_rfc3339(),
                report.period_end.to_rfc3339(),
                report.evidence_count,
                projects,
                report.summary_text
            )
        })
        .collect()
}

fn category_mix_text(mix: &[ActivityCategoryCount]) -> String {
    if mix.is_empty() {
        return "unknown category mix".into();
    }
    mix.iter()
        .map(|item| format!("{}:{}", item.activity_category.as_str(), item.count))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn analysis_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.analysis_status.lock() {
        Ok(status) => Json(status.clone()).into_response(),
        Err(_) => internal_error("analysis status lock poisoned"),
    }
}

async fn analyze_screenshot(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let now = Utc::now();
    let screenshot = {
        let store = match state.store.lock() {
            Ok(store) => store,
            Err(_) => return internal_error("store lock poisoned"),
        };
        match store.get_screenshot(id) {
            Ok(Some(screenshot)) => screenshot,
            Ok(None) => return (StatusCode::NOT_FOUND, "screenshot not found").into_response(),
            Err(err) => return internal_error(err),
        }
    };
    let image_path = if screenshot.file_path.is_empty() {
        None
    } else {
        Some(state.screenshot_dir.join(&screenshot.file_path))
    };
    let input = VisualAnalysisInput {
        screenshot: &screenshot,
        image_path: image_path.as_deref(),
    };
    let analyzer = match ConfiguredVisualAnalyzer::from_env() {
        Ok(analyzer) => analyzer,
        Err(err) => return internal_error(err),
    };
    let mut summary = match analyzer.analyze(&input, now).await {
        Ok(summary) => summary,
        Err(err) => return internal_error(err),
    };
    let mut store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };
    match store.insert_visual_summary(&summary) {
        Ok(summary_id) => {
            summary.id = summary_id;
            Json(VisualAnalyzeResponse { summary }).into_response()
        }
        Err(err) => internal_error(err),
    }
}

#[derive(Debug, Deserialize)]
struct InputEventsQuery {
    limit: Option<usize>,
    #[serde(rename = "segmentId")]
    segment_id: Option<String>,
}

async fn input_events(
    State(state): State<AppState>,
    Query(query): Query<InputEventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_input_events(limit, query.segment_id.as_deref()) {
        Ok(events) => Json(InputEventsResponse { events }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn input_summary(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.get_input_summary(&date) {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn text_segments(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_text_segments(&date, limit) {
        Ok(segments) => Json(TextSegmentsResponse { segments }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn shutdown(State(state): State<AppState>) -> impl IntoResponse {
    let tx = match state.shutdown_tx.lock() {
        Ok(mut tx) => tx.take(),
        Err(_) => return internal_error("shutdown lock poisoned"),
    };

    match tx {
        Some(tx) => {
            let _ = tx.send(());
            StatusCode::OK.into_response()
        }
        None => (StatusCode::SERVICE_UNAVAILABLE, "shutdown unavailable").into_response(),
    }
}

fn internal_error(message: impl std::fmt::Display) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response()
}

fn bad_request(message: impl std::fmt::Display) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, message.to_string()).into_response()
}

async fn shutdown_signal(mut shutdown_rx: oneshot::Receiver<()>) {
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(err) = result {
                eprintln!("shutdown signal listener failed: {err:#}");
            }
        }
        _ = &mut shutdown_rx => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_capture_success_updates_health_after_prior_error() {
        let store = Store::open_memory().unwrap();
        store.init().unwrap();
        let state = default_state(store, None, None);
        {
            let mut health = state.health.lock().unwrap();
            health.window_collector.error_count = 1;
            health.window_collector.last_error = Some("prior failure".into());
            health.window_collector.last_capture_status = Some("capture_unavailable".into());
        }

        record_window_capture_success(&state, "ok", None);

        let health = state.health.lock().unwrap();
        assert_eq!(
            health.window_collector.last_capture_status.as_deref(),
            Some("ok")
        );
        assert_eq!(health.window_collector.error_count, 0);
        assert_eq!(health.window_collector.last_error, None);
        assert_eq!(health.window_collector.last_event_at, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_does_not_hold_health_lock_while_waiting_for_store() {
        let store = Store::open_memory().unwrap();
        store.init().unwrap();
        let state = default_state(store, None, None);
        let state_for_thread = state.clone();
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();

        let holder = std::thread::spawn(move || {
            let _store = state_for_thread.store.lock().unwrap();
            locked_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        locked_rx.recv().unwrap();

        let state_for_request = state.clone();
        let health_task =
            tokio::spawn(async move { health(State(state_for_request)).await.into_response() });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let health_lock_available = state.health.try_lock().is_ok();
        release_tx.send(()).unwrap();
        holder.join().unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(1), health_task)
            .await
            .unwrap()
            .unwrap();

        assert!(
            health_lock_available,
            "health endpoint must not hold the health lock while waiting for store stats"
        );
    }

    #[test]
    fn screenshot_skip_metadata_omits_blocked_window_details() {
        let snapshot = WindowSnapshot {
            captured_at: Utc::now(),
            hwnd: 100,
            pid: 42,
            process_name: "Secret.exe".into(),
            exe_path_hash: None,
            window_title: Some("Sensitive window".into()),
            capture_status: crate::models::CaptureStatus::Ok,
        };

        let (process_name, window_title) = screenshot_skip_metadata("blocked", Some(&snapshot));

        assert_eq!(process_name, None);
        assert_eq!(window_title, None);
    }

    #[test]
    fn capture_unavailable_lifecycle_is_recorded_only_for_new_errors() {
        let mut last_error = None;

        assert!(should_record_capture_unavailable(
            &mut last_error,
            "first error"
        ));
        assert_eq!(last_error.as_deref(), Some("first error"));
        assert!(!should_record_capture_unavailable(
            &mut last_error,
            "first error"
        ));
        assert!(should_record_capture_unavailable(
            &mut last_error,
            "second error"
        ));
        assert_eq!(last_error.as_deref(), Some("second error"));
    }

    #[test]
    fn high_res_capture_profile_uses_minute_interval_and_analysis_resolution() {
        let store = Store::open_memory().unwrap();
        store.init().unwrap();
        let state = default_state(store, None, None);

        let profile = screenshot_capture_profile(&state, ScreenshotCaptureKind::HighRes);

        assert_eq!(profile.interval_secs, 60);
        assert_eq!(profile.max_width, 1600);
        assert_eq!(profile.quality, 88);
        assert_eq!(
            profile.directory,
            PathBuf::from("data/high-res-screenshots")
        );
    }

    #[test]
    fn thumbnail_capture_profile_uses_readable_ui_resolution() {
        let store = Store::open_memory().unwrap();
        store.init().unwrap();
        let state = default_state(store, None, None);

        let profile = screenshot_capture_profile(&state, ScreenshotCaptureKind::Thumbnail);

        assert_eq!(profile.interval_secs, 60);
        assert_eq!(profile.max_width, 960);
        assert_eq!(profile.quality, 82);
        assert_eq!(profile.directory, PathBuf::from("data/screenshots"));
    }

    #[test]
    fn high_res_capture_kind_writes_high_res_table() {
        let mut store = Store::open_memory().unwrap();
        store.init().unwrap();
        let session_id = store.create_session("0.1.0", "test-config").unwrap();

        insert_screenshot_capture(
            &mut store,
            ScreenshotCaptureKind::HighRes,
            &session_id,
            &ScreenshotCaptureRecord {
                captured_at: ts("2026-05-25T09:05:00Z"),
                file_path: "2026-05-25/09-05-00.jpg".into(),
                file_size_bytes: 1024,
                width: 1440,
                height: 900,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

        assert!(
            store
                .list_screenshots_by_date("2026-05-25", 10)
                .unwrap()
                .is_empty()
        );
        let high_res_rows = store
            .list_high_res_screenshots_by_date("2026-05-25", 10)
            .unwrap();
        assert_eq!(high_res_rows.len(), 1);
        assert_eq!(high_res_rows[0].file_path, "2026-05-25/09-05-00.jpg");
    }

    fn ts(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
