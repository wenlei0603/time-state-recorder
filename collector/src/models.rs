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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEventType {
    #[serde(rename = "keydown")]
    KeyDown,
    #[serde(rename = "keyup")]
    KeyUp,
}

impl InputEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KeyDown => "keydown",
            Self::KeyUp => "keyup",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "keyup" => Self::KeyUp,
            _ => Self::KeyDown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    pub id: i64,
    pub event_ts: DateTime<Utc>,
    pub event_type: InputEventType,
    pub vk_code: u32,
    pub scan_code: u32,
    pub character: Option<String>,
    pub segment_id: String,
    pub foreground_hwnd: i64,
    pub foreground_pid: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub text_content: String,
    pub key_count: usize,
    pub backspace_count: usize,
    pub delete_count: usize,
    pub foreground_hwnd: i64,
    pub foreground_pid: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSummary {
    pub date: String,
    pub total_events: usize,
    pub keydown_count: usize,
    pub keyup_count: usize,
    pub segment_count: usize,
    pub total_chars: usize,
    pub last_activity: Option<DateTime<Utc>>,
    pub top_apps: Vec<AppInputCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInputCount {
    pub process_name: String,
    pub char_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorHealth {
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub version: String,
    pub window_collector: SubsystemHealth,
    pub input_collector: SubsystemHealth,
    pub screenshot_collector: SubsystemHealth,
    pub db_stats: DbStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemHealth {
    pub status: String,
    pub last_event_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub window_events: usize,
    pub input_events: usize,
    pub text_segments: usize,
    pub screenshots: usize,
    pub blocker_hits: usize,
}
