use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockerConfig {
    pub version: u32,
    pub rules: Vec<BlockerRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockerRule {
    pub capture_type: String,
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockerHit {
    pub id: i64,
    pub hit_at: DateTime<Utc>,
    pub capture_type: String,
    pub field: String,
    pub operator: String,
    pub rule_value: String,
    pub actual_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub captured_at: DateTime<Utc>,
    pub hwnd: i64,
    pub pid: u32,
    pub process_name: String,
    pub exe_path_hash: Option<String>,
    pub window_title: Option<String>,
    pub capture_status: CaptureStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    Ok,
    NoForegroundWindow,
    PermissionDenied,
    Unavailable,
}

impl CaptureStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NoForegroundWindow => "no_foreground_window",
            Self::PermissionDenied => "permission_denied",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "ok" => Self::Ok,
            "no_foreground_window" => Self::NoForegroundWindow,
            "permission_denied" => Self::PermissionDenied,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredWindowEvent {
    pub raw_event_id: i64,
    pub session_id: String,
    pub event_ts: DateTime<Utc>,
    pub hwnd: i64,
    pub pid: u32,
    pub process_name: String,
    pub exe_path_hash: Option<String>,
    pub window_title: Option<String>,
    pub capture_status: CaptureStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEvent {
    pub id: String,
    pub app: String,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotMeta {
    pub id: i64,
    pub captured_at: DateTime<Utc>,
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub capture_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSummary {
    pub date: String,
    pub total_screenshots: usize,
    pub hours_covered: usize,
    pub top_apps: Vec<AppScreenshotCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppScreenshotCount {
    pub process_name: String,
    pub count: usize,
}
