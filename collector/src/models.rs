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
    pub kind: TimeEventKind,
    pub status: Option<String>,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeEventKind {
    ActiveWindow,
    Lifecycle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityBucket {
    pub id: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub bucket_seconds: i64,
    pub dominant_app: String,
    pub dominant_title: String,
    pub normalized_title: String,
    pub dominant_duration_seconds: i64,
    pub switch_count: usize,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub activity_category: ActivityCategory,
    pub attention_state: AttentionState,
    pub confidence: f64,
    pub evidence: Vec<BucketEvidence>,
    pub visual_summary_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketEvidence {
    pub event_id: String,
    pub app: String,
    pub title: String,
    pub normalized_title: String,
    pub kind: TimeEventKind,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    ProjectWork,
    Research,
    Writing,
    Coding,
    Communication,
    Meeting,
    Admin,
    Learning,
    Planning,
    Loafing,
    Personal,
    Idle,
    Unknown,
}

impl ActivityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectWork => "project_work",
            Self::Research => "research",
            Self::Writing => "writing",
            Self::Coding => "coding",
            Self::Communication => "communication",
            Self::Meeting => "meeting",
            Self::Admin => "admin",
            Self::Learning => "learning",
            Self::Planning => "planning",
            Self::Loafing => "loafing",
            Self::Personal => "personal",
            Self::Idle => "idle",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "project_work" => Some(Self::ProjectWork),
            "research" => Some(Self::Research),
            "writing" => Some(Self::Writing),
            "coding" => Some(Self::Coding),
            "communication" => Some(Self::Communication),
            "meeting" => Some(Self::Meeting),
            "admin" => Some(Self::Admin),
            "learning" => Some(Self::Learning),
            "planning" => Some(Self::Planning),
            "loafing" => Some(Self::Loafing),
            "personal" => Some(Self::Personal),
            "idle" => Some(Self::Idle),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionState {
    DeepFocus,
    Steady,
    LightSwitching,
    Fragmented,
    Away,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleType {
    SessionStart,
    SessionStop,
    WindowsLock,
    WindowsUnlock,
    PowerSuspend,
    PowerResume,
    IdleStart,
    IdleEnd,
    CaptureUnavailable,
    CollectorGap,
    SessionDisconnect,
    SessionReconnect,
}

impl LifecycleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::SessionStop => "session_stop",
            Self::WindowsLock => "windows_lock",
            Self::WindowsUnlock => "windows_unlock",
            Self::PowerSuspend => "power_suspend",
            Self::PowerResume => "power_resume",
            Self::IdleStart => "idle_start",
            Self::IdleEnd => "idle_end",
            Self::CaptureUnavailable => "capture_unavailable",
            Self::CollectorGap => "collector_gap",
            Self::SessionDisconnect => "session_disconnect",
            Self::SessionReconnect => "session_reconnect",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "session_start" => Some(Self::SessionStart),
            "session_stop" => Some(Self::SessionStop),
            "windows_lock" => Some(Self::WindowsLock),
            "windows_unlock" => Some(Self::WindowsUnlock),
            "power_suspend" => Some(Self::PowerSuspend),
            "power_resume" => Some(Self::PowerResume),
            "idle_start" => Some(Self::IdleStart),
            "idle_end" => Some(Self::IdleEnd),
            "capture_unavailable" => Some(Self::CaptureUnavailable),
            "collector_gap" => Some(Self::CollectorGap),
            "session_disconnect" => Some(Self::SessionDisconnect),
            "session_reconnect" => Some(Self::SessionReconnect),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    pub raw_event_id: i64,
    pub session_id: String,
    pub event_ts: DateTime<Utc>,
    pub lifecycle_type: LifecycleType,
    pub reason: Option<String>,
    pub active_session_id: Option<String>,
    pub payload: serde_json::Value,
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
pub struct HighResScreenshotMeta {
    pub id: i64,
    pub captured_at: DateTime<Utc>,
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub capture_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSummary {
    pub id: i64,
    pub screenshot_id: i64,
    pub captured_at: DateTime<Utc>,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub summary_text: String,
    pub activity_category: ActivityCategory,
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub visible_apps: Vec<String>,
    pub visible_text_hints: Vec<String>,
    pub risk_flags: Vec<String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualObservation {
    pub id: i64,
    pub high_res_screenshot_id: i64,
    pub captured_at: DateTime<Utc>,
    pub file_path: String,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub summary_text: String,
    pub activity_category: ActivityCategory,
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub visible_apps: Vec<String>,
    pub visible_text_hints: Vec<String>,
    pub risk_flags: Vec<String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualTrajectoryPoint {
    pub minute_mark: u8,
    pub screenshot_id: i64,
    pub observation: String,
    pub activity_category: ActivityCategory,
    #[serde(default)]
    pub project_hints: Vec<String>,
    #[serde(default = "unknown_visual_tags")]
    pub identity_tags: Vec<String>,
    #[serde(default = "unknown_visual_tags")]
    pub routine_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualWindowSummary {
    pub id: i64,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub sampled_screenshot_ids: Vec<i64>,
    pub previous_summary_id: Option<i64>,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub summary_text: String,
    pub continuity: String,
    pub primary_activity: ActivityCategory,
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub task_intent: String,
    pub trajectory: Vec<VisualTrajectoryPoint>,
    pub switching_level: String,
    pub switching_evidence: String,
    pub loafing_level: String,
    pub loafing_evidence: String,
    pub visible_apps: Vec<String>,
    pub visible_text_hints: Vec<String>,
    pub risk_flags: Vec<String>,
    pub confidence: f64,
    pub raw_summary_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
}

fn unknown_visual_tags() -> Vec<String> {
    vec!["unknown".to_string()]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCategoryCount {
    pub activity_category: ActivityCategory,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightReport {
    pub id: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub report_kind: String,
    pub model_provider: String,
    pub model_name: String,
    pub summary_text: String,
    pub category_mix: Vec<ActivityCategoryCount>,
    pub project_hints: Vec<String>,
    pub evidence_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyAppActivity {
    pub process_name: String,
    pub active_seconds: i64,
    pub share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivityStats {
    pub date: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub active_seconds: i64,
    pub active_hours: f64,
    pub window_event_count: usize,
    pub switch_count: usize,
    pub distinct_app_count: usize,
    pub top_apps: Vec<DailyAppActivity>,
    pub category_mix: Vec<ActivityCategoryCount>,
    pub input_chars: usize,
    pub input_events: usize,
    pub screenshot_count: usize,
    pub high_res_screenshot_count: usize,
    pub visual_window_count: usize,
    pub five_hour_report_count: usize,
    pub first_activity_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyActivityMetric {
    pub hour: u8,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub active_seconds: i64,
    pub active_ratio: f64,
    pub window_event_count: usize,
    pub switch_count: usize,
    pub distinct_app_count: usize,
    pub dominant_app: Option<String>,
    pub dominant_category: ActivityCategory,
    pub input_chars: usize,
    pub screenshot_count: usize,
    pub high_res_screenshot_count: usize,
    pub visual_window_count: usize,
    pub five_hour_report_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyComparison {
    pub baseline_days: usize,
    pub compared_dates: Vec<String>,
    pub active_seconds_delta: i64,
    pub switches_per_hour_delta: f64,
    pub input_chars_delta: i64,
    pub screenshot_coverage_delta: f64,
    pub dominant_category_shift: Option<String>,
    pub start_time_shift_minutes: Option<i64>,
    pub end_time_shift_minutes: Option<i64>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyBrief {
    pub id: i64,
    pub date: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub scheduled_for_local: String,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub status: String,
    pub descriptive_stats: DailyActivityStats,
    pub hourly_metrics: Vec<HourlyActivityMetric>,
    pub comparison: DailyComparison,
    pub five_hour_report_ids: Vec<i64>,
    pub daily_summary_text: String,
    pub action_trajectory: String,
    pub raw_summary_json: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSummary {
    pub date: String,
    pub total_screenshots: usize,
    pub hours_covered: usize,
    pub top_apps: Vec<AppScreenshotCount>,
    pub skipped_reasons: Vec<ScreenshotSkippedReasonCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSkippedReasonCount {
    pub reason: String,
    pub count: usize,
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
    pub mode: Option<String>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub last_capture_status: Option<String>,
    pub last_skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub window_events: usize,
    pub lifecycle_events: usize,
    pub input_events: usize,
    pub text_segments: usize,
    pub screenshots: usize,
    pub high_res_screenshots: usize,
    pub blocker_hits: usize,
    pub image_retention: ImageRetentionStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRetentionStats {
    pub retention_days: u32,
    pub active_files: usize,
    pub expired_files: usize,
    pub active_bytes: u64,
    pub expired_bytes: u64,
    pub pending_google_drive_upload: bool,
    pub google_drive_message: Option<String>,
}

impl ImageRetentionStats {
    pub fn inactive(retention_days: u32) -> Self {
        Self {
            retention_days,
            active_files: 0,
            expired_files: 0,
            active_bytes: 0,
            expired_bytes: 0,
            pending_google_drive_upload: false,
            google_drive_message: None,
        }
    }
}

impl DbStats {
    pub fn empty(retention_days: u32) -> Self {
        Self {
            window_events: 0,
            lifecycle_events: 0,
            input_events: 0,
            text_segments: 0,
            screenshots: 0,
            high_res_screenshots: 0,
            blocker_hits: 0,
            image_retention: ImageRetentionStats::inactive(retention_days),
        }
    }
}
