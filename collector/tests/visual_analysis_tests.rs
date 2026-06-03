use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tsr_collector::{
    models::{ActivityCategory, ScreenshotMeta},
    visual_analysis::{LocalMetadataAnalyzer, VisualAnalysisInput, VisualAnalyzer},
};

#[test]
fn local_metadata_analyzer_turns_screenshot_metadata_into_summary() {
    let analyzer = LocalMetadataAnalyzer::default();
    let screenshot = ScreenshotMeta {
        id: 42,
        captured_at: ts("2026-05-25T09:05:00Z"),
        file_path: "2026-05-25/09-05.jpg".into(),
        width: 1280,
        height: 720,
        process_name: Some("Code.exe".into()),
        window_title: Some("time-state-recorder main.rs".into()),
        capture_status: "ok".into(),
    };

    let input = VisualAnalysisInput {
        screenshot: &screenshot,
        image_path: None,
    };
    let summary = analyzer
        .analyze(&input, ts("2026-05-25T09:05:10Z"))
        .unwrap();

    assert_eq!(summary.screenshot_id, 42);
    assert_eq!(summary.model_provider, "local_stub");
    assert_eq!(summary.model_name, "metadata-v1");
    assert_eq!(summary.activity_category, ActivityCategory::Coding);
    assert_eq!(summary.project_hints, vec!["Time State Recorder"]);
    assert_eq!(summary.visible_apps, vec!["Code.exe"]);
    assert_eq!(
        summary.visible_text_hints,
        vec!["time-state-recorder main.rs"]
    );
    assert!(summary.summary_text.contains("Code.exe"));
}

#[test]
fn local_metadata_analyzer_flags_low_quality_capture_metadata() {
    let analyzer = LocalMetadataAnalyzer::default();
    let screenshot = ScreenshotMeta {
        id: 43,
        captured_at: ts("2026-05-25T09:06:00Z"),
        file_path: String::new(),
        width: 0,
        height: 0,
        process_name: None,
        window_title: None,
        capture_status: "capture_failed".into(),
    };

    let input = VisualAnalysisInput {
        screenshot: &screenshot,
        image_path: None,
    };
    let summary = analyzer
        .analyze(&input, ts("2026-05-25T09:06:10Z"))
        .unwrap();

    assert_eq!(summary.activity_category, ActivityCategory::Unknown);
    assert!(
        summary
            .risk_flags
            .contains(&"capture_status:capture_failed".to_string())
    );
    assert!(summary.risk_flags.contains(&"empty_dimensions".to_string()));
    assert!(summary.summary_text.contains("capture_failed"));
}

#[test]
fn visual_analysis_input_carries_optional_image_path() {
    let analyzer = LocalMetadataAnalyzer::default();
    let screenshot = ScreenshotMeta {
        id: 44,
        captured_at: ts("2026-05-25T09:07:00Z"),
        file_path: "2026-05-25/09-07.jpg".into(),
        width: 1280,
        height: 720,
        process_name: Some("msedge.exe".into()),
        window_title: Some("Research notes".into()),
        capture_status: "ok".into(),
    };
    let image_path = PathBuf::from("data/screenshots/2026-05-25/09-07.jpg");
    let input = VisualAnalysisInput {
        screenshot: &screenshot,
        image_path: Some(image_path.as_path()),
    };

    let summary = analyzer
        .analyze(&input, ts("2026-05-25T09:07:10Z"))
        .unwrap();

    assert_eq!(summary.screenshot_id, 44);
    assert_eq!(summary.activity_category, ActivityCategory::Research);
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
