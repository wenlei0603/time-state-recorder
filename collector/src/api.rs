use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tokio::{sync::oneshot, time};
use tower_http::services::ServeDir;

use crate::{
    activity::{ActivityBucketQuery, build_activity_buckets},
    blocker::BlockerEngine,
    input,
    interval::build_time_events_with_lifecycle,
    models::{
        ActivityBucket, BlockerHit, CollectorHealth, DbStats, HighResScreenshotMeta,
        LifecycleEvent, LifecycleType, ScreenshotMeta, SubsystemHealth, TimeEvent, VisualSummary,
        WindowSnapshot,
    },
    screenshot,
    storage::Store,
    visual_analysis::{LocalMetadataAnalyzer, VisualAnalysisInput, VisualAnalyzer},
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
const DEFAULT_HIGH_RES_SCREENSHOT_LIMIT: usize = 288;
const DEFAULT_HIGH_RES_SCREENSHOT_INTERVAL: u64 = 300;
const THUMBNAIL_SCREENSHOT_MAX_WIDTH: u32 = 640;
const THUMBNAIL_SCREENSHOT_QUALITY: u8 = 60;
const HIGH_RES_SCREENSHOT_MAX_WIDTH: u32 = 1440;
const HIGH_RES_SCREENSHOT_QUALITY: u8 = 80;

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
            db_stats: DbStats {
                window_events: 0,
                lifecycle_events: 0,
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        })),
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
        ScreenshotCaptureKind::Thumbnail => store.insert_screenshot(
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
        ),
        ScreenshotCaptureKind::HighRes => store.insert_high_res_screenshot(
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
    let input_collector = input::spawn_input_collector(state.store.clone(), state.health.clone());

    let app = router_from_state(state.clone());
    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    window_collector.abort();
    screenshot_collector.abort();
    high_res_screenshot_collector.abort();
    input_collector.abort();
    let _ = window_collector.await;
    let _ = screenshot_collector.await;
    let _ = high_res_screenshot_collector.await;
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

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let h = match state.health.lock() {
        Ok(h) => h,
        Err(_) => return internal_error("health lock poisoned"),
    };

    let uptime = (Utc::now() - h.started_at).num_seconds().max(0) as u64;

    let overall = if h.window_collector.status == "error"
        || h.input_collector.status == "error"
        || h.screenshot_collector.status == "error"
    {
        "error"
    } else if h.window_collector.status == "not_started"
        || h.input_collector.status == "not_started"
        || h.screenshot_collector.status == "not_started"
    {
        "degraded"
    } else {
        "ok"
    };

    let db_stats = match state.store.lock() {
        Ok(store) => match store.get_db_stats() {
            Ok(stats) => stats,
            Err(_) => DbStats {
                window_events: 0,
                lifecycle_events: 0,
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        },
        Err(_) => DbStats {
            window_events: 0,
            lifecycle_events: 0,
            input_events: 0,
            text_segments: 0,
            screenshots: 0,
            blocker_hits: 0,
        },
    };

    Json(CollectorHealth {
        status: overall.into(),
        uptime_seconds: uptime,
        db_stats,
        ..h.clone()
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
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let limit = query.limit.unwrap_or(DEFAULT_SCREENSHOT_LIMIT).min(5000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_screenshots_by_date(&date, limit) {
        Ok(screenshots) => Json(ScreenshotsResponse { screenshots }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn screenshot_summary(
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

    match store.get_screenshot_summary(&date) {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn high_res_screenshots(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_err() {
        return bad_request("date must use YYYY-MM-DD");
    }
    let limit = query
        .limit
        .unwrap_or(DEFAULT_HIGH_RES_SCREENSHOT_LIMIT)
        .min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_high_res_screenshots_by_date(&date, limit) {
        Ok(screenshots) => Json(HighResScreenshotsResponse { screenshots }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn visual_summaries(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query
        .date
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_err() {
        return bad_request("date must use YYYY-MM-DD");
    }
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_visual_summaries_by_date(&date, limit) {
        Ok(summaries) => Json(VisualSummariesResponse { summaries }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn analyze_screenshot(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let now = Utc::now();
    let mut store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };
    let screenshot = match store.get_screenshot(id) {
        Ok(Some(screenshot)) => screenshot,
        Ok(None) => return (StatusCode::NOT_FOUND, "screenshot not found").into_response(),
        Err(err) => return internal_error(err),
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
    let analyzer = LocalMetadataAnalyzer;
    let mut summary = match analyzer.analyze(&input, now) {
        Ok(summary) => summary,
        Err(err) => return internal_error(err),
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
    fn high_res_capture_profile_uses_prd_interval_and_resolution() {
        let store = Store::open_memory().unwrap();
        store.init().unwrap();
        let state = default_state(store, None, None);

        let profile = screenshot_capture_profile(&state, ScreenshotCaptureKind::HighRes);

        assert_eq!(profile.interval_secs, 300);
        assert_eq!(profile.max_width, 1440);
        assert_eq!(profile.quality, 80);
        assert_eq!(
            profile.directory,
            PathBuf::from("data/high-res-screenshots")
        );
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
