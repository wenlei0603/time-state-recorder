use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tsr_collector::{
    models::{ActivityCategory, ScreenshotMeta},
    visual_analysis::{
        LocalMetadataAnalyzer, MiniMaxAnalyzer, MiniMaxConfig, VisualAnalysisInput, VisualAnalyzer,
        select_visual_analyzer_provider,
    },
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

#[test]
fn minimax_request_uses_openai_chat_completions_image_content_block() {
    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("screen.jpg");
    std::fs::write(&image_path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
    let screenshot = ScreenshotMeta {
        id: 45,
        captured_at: ts("2026-05-25T09:08:00Z"),
        file_path: "2026-05-25/09-08.jpg".into(),
        width: 1280,
        height: 720,
        process_name: Some("Code.exe".into()),
        window_title: Some("visual_analysis.rs".into()),
        capture_status: "ok".into(),
    };
    let input = VisualAnalysisInput {
        screenshot: &screenshot,
        image_path: Some(image_path.as_path()),
    };
    let analyzer = MiniMaxAnalyzer::new(MiniMaxConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));

    let request = analyzer.build_chat_completions_request(&input).unwrap();

    assert_eq!(request["model"], "MiniMax-M3");
    assert_eq!(request["messages"][1]["role"], "user");
    assert_eq!(request["messages"][1]["content"][0]["type"], "text");
    assert_eq!(request["messages"][1]["content"][1]["type"], "image_url");
    assert_eq!(
        request["messages"][1]["content"][1]["image_url"]["detail"],
        "default"
    );
    assert!(
        request["messages"][1]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/jpeg;base64,")
    );
    assert_eq!(request["thinking"]["type"], "disabled");
}

#[test]
fn provider_selection_infers_minimax_when_credentials_are_present() {
    assert_eq!(
        select_visual_analyzer_provider(None, Some("secret"), Some("https://api.minimax.test")),
        "minimax"
    );
    assert_eq!(
        select_visual_analyzer_provider(None, Some("secret"), None),
        "local"
    );
    assert_eq!(
        select_visual_analyzer_provider(
            Some("local"),
            Some("secret"),
            Some("https://api.minimax.test")
        ),
        "local"
    );
}

#[test]
fn minimax_json_content_maps_to_visual_summary() {
    let screenshot = ScreenshotMeta {
        id: 46,
        captured_at: ts("2026-05-25T09:09:00Z"),
        file_path: "2026-05-25/09-09.jpg".into(),
        width: 1280,
        height: 720,
        process_name: Some("Code.exe".into()),
        window_title: Some("visual_analysis.rs".into()),
        capture_status: "ok".into(),
    };

    let summary = MiniMaxAnalyzer::summary_from_response_text(
        &screenshot,
        ts("2026-05-25T09:09:10Z"),
        "MiniMax-M3",
        r#"{
          "summaryText": "正在编辑视觉分析模块。",
          "activityCategory": "coding",
          "projectHints": ["Time State Recorder"],
          "visibleApps": ["Code.exe"],
          "visibleTextHints": ["visual_analysis.rs"],
          "riskFlags": [],
          "confidence": 0.82
        }"#,
    )
    .unwrap();

    assert_eq!(summary.screenshot_id, 46);
    assert_eq!(summary.model_provider, "minimax");
    assert_eq!(summary.model_name, "MiniMax-M3");
    assert_eq!(summary.prompt_version, "visual-summary-minimax-m3-v1");
    assert_eq!(summary.summary_text, "正在编辑视觉分析模块。");
    assert_eq!(summary.activity_category, ActivityCategory::Coding);
    assert_eq!(summary.project_hints, vec!["Time State Recorder"]);
    assert_eq!(summary.confidence, 0.82);
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
