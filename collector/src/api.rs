use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::time;
use tower_http::services::ServeDir;

use crate::{
    blocker::BlockerEngine,
    input,
    interval::build_time_events,
    models::{BlockerHit, CollectorHealth, DbStats, ScreenshotMeta, SubsystemHealth, TimeEvent},
    screenshot,
    storage::Store,
    window::sample_foreground_window,
};

#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<Store>>,
    blocker_engine: Arc<BlockerEngine>,
    screenshot_dir: Arc<PathBuf>,
    screenshot_interval_secs: Arc<u64>,
    idle_threshold_secs: Arc<u64>,
    health: Arc<Mutex<CollectorHealth>>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeEventsResponse {
    events: Vec<TimeEvent>,
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

pub fn router(store: Store, blocker_config_path: Option<PathBuf>) -> Router {
    router_from_state(default_state(store, blocker_config_path))
}

fn default_state(store: Store, blocker_config_path: Option<PathBuf>) -> AppState {
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
        idle_threshold_secs: Arc::new(DEFAULT_IDLE_THRESHOLD),
        health: Arc::new(Mutex::new(CollectorHealth {
            status: "ok".into(),
            started_at: now,
            uptime_seconds: 0,
            version: env!("CARGO_PKG_VERSION").into(),
            window_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            input_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            screenshot_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            db_stats: DbStats {
                window_events: 0,
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        })),
    }
}

fn router_from_state(state: AppState) -> Router {
    let screenshot_dir = state.screenshot_dir.to_path_buf();
    Router::new()
        .route("/api/health", get(health))
        .route("/api/window-events", get(window_events))
        .route("/api/time-events", get(time_events))
        .route("/api/blockers", get(blockers))
        .route("/api/screenshots", get(screenshots))
        .route("/api/screenshot-summary", get(screenshot_summary))
        .route("/api/input-events", get(input_events))
        .route("/api/input-summary", get(input_summary))
        .route("/api/text-segments", get(text_segments))
        .nest_service("/screenshots", ServeDir::new(screenshot_dir))
        .with_state(state)
}

pub async fn serve(
    store: Store,
    addr: SocketAddr,
    poll_ms: u64,
    blocker_config_path: Option<PathBuf>,
) -> Result<()> {
    anyhow::ensure!(poll_ms >= 100, "poll_ms must be at least 100");
    let session_id = store.create_session(env!("CARGO_PKG_VERSION"), "default")?;
    let state = default_state(store, blocker_config_path);

    spawn_collector_loop(state.clone(), session_id.clone(), poll_ms);
    spawn_screenshot_loop(state.clone(), session_id);
    input::spawn_input_collector(state.store.clone(), state.health.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router_from_state(state)).await?;
    Ok(())
}

fn spawn_collector_loop(state: AppState, session_id: String, poll_ms: u64) {
    tokio::spawn(async move {
        {
            if let Ok(mut h) = state.health.lock() {
                h.window_collector.status = "running".into();
            }
        }
        let mut last_identity: Option<(i64, u32, Option<String>)> = None;
        loop {
            match sample_foreground_window() {
                Ok(snapshot) => {
                    let identity = (snapshot.hwnd, snapshot.pid, snapshot.window_title.clone());
                    if last_identity.as_ref() != Some(&identity) {
                        if let Ok(mut store) = state.store.lock() {
                            match store.insert_window_focus(&session_id, &snapshot) {
                                Ok(_) => {
                                    last_identity = Some(identity);
                                    if let Ok(mut h) = state.health.lock() {
                                        h.window_collector.last_event_at = Some(Utc::now());
                                        h.window_collector.error_count = 0;
                                        h.window_collector.last_error = None;
                                    }
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
                    if let Ok(mut h) = state.health.lock() {
                        h.window_collector.error_count += 1;
                        h.window_collector.last_error = Some(format!("{err:#}"));
                    }
                }
            }

            time::sleep(Duration::from_millis(poll_ms)).await;
        }
    });
}

fn spawn_screenshot_loop(state: AppState, session_id: String) {
    let interval = *state.screenshot_interval_secs;
    let idle_threshold = *state.idle_threshold_secs;
    let screenshot_dir = state.screenshot_dir.to_path_buf();

    tokio::spawn(async move {
        {
            if let Ok(mut h) = state.health.lock() {
                h.screenshot_collector.status = "running".into();
            }
        }
        loop {
            time::sleep(Duration::from_secs(interval)).await;

            if screenshot::idle_seconds() > idle_threshold as f64 {
                continue;
            }

            let snapshot = match sample_foreground_window() {
                Ok(s) => s,
                Err(_) => continue,
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
                continue;
            }

            let (bytes, w, h) = match screenshot::capture_thumbnail(640, 60) {
                Some(data) => data,
                None => {
                    if let Ok(mut h) = state.health.lock() {
                        h.screenshot_collector.error_count += 1;
                        h.screenshot_collector.last_error =
                            Some("capture_thumbnail returned None".into());
                    }
                    continue;
                }
            };

            let now = Utc::now();
            let date_dir = now.format("%Y-%m-%d").to_string();
            let filename = format!("{}.jpg", now.format("%H-%M"));
            let dir = screenshot_dir.join(&date_dir);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                eprintln!("screenshot dir create failed: {e:#}");
                if let Ok(mut h) = state.health.lock() {
                    h.screenshot_collector.error_count += 1;
                    h.screenshot_collector.last_error = Some(format!("{e:#}"));
                }
                continue;
            }
            let filepath = dir.join(&filename);

            if let Err(e) = std::fs::write(&filepath, &bytes) {
                eprintln!("screenshot write failed: {e:#}");
                if let Ok(mut h) = state.health.lock() {
                    h.screenshot_collector.error_count += 1;
                    h.screenshot_collector.last_error = Some(format!("{e:#}"));
                }
                continue;
            }

            let relative_path = format!("{}/{}", date_dir, filename);

            if let Ok(mut store) = state.store.lock() {
                match store.insert_screenshot(
                    &session_id,
                    &ScreenshotMeta {
                        id: 0,
                        captured_at: now,
                        file_path: relative_path,
                        width: w,
                        height: h,
                        process_name: Some(snapshot.process_name),
                        window_title: snapshot.window_title,
                        capture_status: "ok".to_string(),
                    },
                ) {
                    Ok(_) => {
                        if let Ok(mut h) = state.health.lock() {
                            h.screenshot_collector.last_event_at = Some(Utc::now());
                            h.screenshot_collector.error_count = 0;
                            h.screenshot_collector.last_error = None;
                        }
                    }
                    Err(err) => {
                        eprintln!("screenshot metadata write failed: {err:#}");
                        if let Ok(mut h) = state.health.lock() {
                            h.screenshot_collector.error_count += 1;
                            h.screenshot_collector.last_error = Some(format!("{err:#}"));
                        }
                    }
                }
            }
        }
    });
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
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        },
        Err(_) => DbStats {
            window_events: 0,
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
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_window_events(limit) {
        Ok(window_events) => Json(TimeEventsResponse {
            events: build_time_events(&window_events),
        })
        .into_response(),
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

fn internal_error(message: impl std::fmt::Display) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response()
}
