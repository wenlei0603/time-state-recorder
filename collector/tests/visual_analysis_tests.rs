use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tsr_collector::{
    models::{ActivityCategory, HighResScreenshotMeta, ScreenshotMeta},
    visual_analysis::{
        LocalMetadataAnalyzer, MiniMaxAnalyzer, MiniMaxConfig, VisualAnalysisInput, VisualAnalyzer,
        WindowScreenshotSample, WindowVisualAnalysisInput, WindowVisualAnalysisSample,
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
    let prompt = request["messages"][1]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(prompt.contains("identityTags"));
    assert!(prompt.contains("routineTags"));
    assert!(prompt.contains("software_builder"));
    assert!(prompt.contains("coding_build"));
    assert!(prompt.contains("capturedAt=2026-05-25T17:08:00+08:00"));
    assert!(!prompt.contains("capturedAt=2026-05-25T09:08:00Z"));
    assert_eq!(request["messages"][1]["content"][1]["type"], "image_url");
    assert_eq!(
        request["messages"][1]["content"][1]["image_url"]["detail"],
        "default"
    );
    assert!(request["messages"][1]["content"][1]["image_url"]["max_long_side_pixel"].is_null());
    assert!(
        request["messages"][1]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/jpeg;base64,")
    );
    assert_eq!(request["thinking"]["type"], "disabled");
}

#[test]
fn minimax_window_analysis_request_defaults_to_ten_thousand_completion_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let image_paths = [1, 3, 5]
        .iter()
        .map(|mark| {
            let path = dir.path().join(format!("minute-{mark}.jpg"));
            std::fs::write(&path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
            path
        })
        .collect::<Vec<_>>();
    let samples = vec![
        window_sample(101, 1, "2026-05-25T09:00:30Z", "Code.exe", "api.rs"),
        window_sample(102, 3, "2026-05-25T09:02:30Z", "Code.exe", "insights.rs"),
        window_sample(103, 5, "2026-05-25T09:04:30Z", "msedge.exe", "TSR UI"),
    ];
    let input = WindowVisualAnalysisInput {
        window_start: ts("2026-05-25T09:00:00Z"),
        window_end: ts("2026-05-25T09:05:00Z"),
        samples: vec![
            WindowVisualAnalysisSample {
                minute_mark: samples[0].minute_mark,
                screenshot: &samples[0].screenshot,
                image_path: image_paths[0].as_path(),
            },
            WindowVisualAnalysisSample {
                minute_mark: samples[1].minute_mark,
                screenshot: &samples[1].screenshot,
                image_path: image_paths[1].as_path(),
            },
            WindowVisualAnalysisSample {
                minute_mark: samples[2].minute_mark,
                screenshot: &samples[2].screenshot,
                image_path: image_paths[2].as_path(),
            },
        ],
        previous_summary: None,
    };
    let analyzer = MiniMaxAnalyzer::new(MiniMaxConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));

    let request = analyzer
        .build_window_chat_completions_request(&input)
        .unwrap();
    let prompt = request["messages"][1]["content"][0]["text"]
        .as_str()
        .unwrap();

    assert_eq!(request["max_completion_tokens"], 10_000);
    assert!(prompt.contains("windowStart=2026-05-25T17:00:00+08:00"));
    assert!(prompt.contains("windowEnd=2026-05-25T17:05:00+08:00"));
    assert!(prompt.contains(r#""capturedAt":"2026-05-25T17:00:30+08:00""#));
    assert!(!prompt.contains("2026-05-25T09:00:00Z"));
    assert!(!prompt.contains("2026-05-25T09:00:30Z"));
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
        r#"```json
        {
          "summaryText": "正在编辑视觉分析模块。",
          "activityCategory": "coding",
          "projectHints": ["Time State Recorder"],
          "identityTags": ["software_builder", "not_a_real_tag"],
          "routineTags": ["coding_build"],
          "visibleApps": ["Code.exe"],
          "visibleTextHints": ["visual_analysis.rs"],
          "riskFlags": [],
          "confidence": 0.82
        }
        ```"#,
    )
    .unwrap();

    assert_eq!(summary.screenshot_id, 46);
    assert_eq!(summary.model_provider, "minimax");
    assert_eq!(summary.model_name, "MiniMax-M3");
    assert_eq!(summary.prompt_version, "visual-summary-minimax-m3-v1");
    assert_eq!(summary.summary_text, "正在编辑视觉分析模块。");
    assert_eq!(summary.activity_category, ActivityCategory::Coding);
    assert_eq!(summary.project_hints, vec!["Time State Recorder"]);
    assert_eq!(summary.identity_tags, vec!["software_builder"]);
    assert_eq!(summary.routine_tags, vec!["coding_build"]);
    assert_eq!(summary.confidence, 0.82);
}

#[test]
fn minimax_window_analysis_response_maps_identity_and_routine_tags() {
    let samples = vec![
        window_sample(
            101,
            1,
            "2026-05-25T09:00:30Z",
            "Code.exe",
            "time-state-recorder models.rs",
        ),
        window_sample(
            102,
            3,
            "2026-05-25T09:02:30Z",
            "Code.exe",
            "cargo test visual_analysis",
        ),
        window_sample(
            103,
            5,
            "2026-05-25T09:04:30Z",
            "msedge.exe",
            "Rust async docs",
        ),
    ];

    let summary = MiniMaxAnalyzer::window_summary_from_response_text(
        ts("2026-05-25T09:00:00Z"),
        ts("2026-05-25T09:05:00Z"),
        &samples,
        Some(7),
        ts("2026-05-25T09:05:10Z"),
        "MiniMax-M3",
        r#"```json
        {
          "summaryText": "正在构建视觉标签功能。",
          "continuity": "延续上一窗口的编码工作。",
          "primaryActivity": "coding",
          "projectHints": ["Time State Recorder"],
          "identityTags": ["software_builder", "not_a_real_tag"],
          "routineTags": ["coding_build"],
          "taskIntent": "实现窗口级图片标签。",
          "trajectory": [
            {
              "minuteMark": 1,
              "observation": "正在编辑 Rust 模型。",
              "activityCategory": "coding",
              "projectHints": ["Time State Recorder"],
              "identityTags": ["software_builder"],
              "routineTags": ["coding_build"]
            },
            {
              "minuteMark": 3,
              "observation": "正在运行 cargo 测试。",
              "activityCategory": "coding",
              "projectHints": ["Time State Recorder"],
              "identityTags": ["software_builder"],
              "routineTags": ["coding_build"]
            },
            {
              "minuteMark": 5,
              "observation": "正在查看 Rust 文档。",
              "activityCategory": "learning",
              "projectHints": ["Rust"],
              "identityTags": ["software_builder"],
              "routineTags": ["learning_exploration", "not_a_real_tag"]
            }
          ],
          "switchingLevel": "low",
          "switchingEvidence": "三个采样点都围绕同一功能。",
          "loafingLevel": "none",
          "loafingEvidence": "没有看到娱乐或离开工作。",
          "visibleApps": ["Code.exe", "msedge.exe"],
          "visibleTextHints": ["models.rs", "Rust async docs"],
          "riskFlags": [],
          "confidence": 0.88
        }
        ```"#,
    )
    .unwrap();

    assert_eq!(summary.identity_tags, vec!["software_builder"]);
    assert_eq!(summary.routine_tags, vec!["coding_build"]);
    assert!(
        !summary
            .identity_tags
            .contains(&"not_a_real_tag".to_string())
    );
    let minute_five = summary
        .trajectory
        .iter()
        .find(|point| point.minute_mark == 5)
        .unwrap();
    assert_eq!(minute_five.routine_tags, vec!["learning_exploration"]);
}

#[test]
fn minimax_window_analysis_parse_failure_keeps_raw_content_for_diagnostics() {
    let samples = vec![
        window_sample(101, 1, "2026-05-25T09:00:30Z", "Code.exe", "api.rs"),
        window_sample(102, 3, "2026-05-25T09:02:30Z", "Code.exe", "insights.rs"),
        window_sample(103, 5, "2026-05-25T09:04:30Z", "msedge.exe", "TSR UI"),
    ];
    let content = "{\"summaryText\":\"截断";

    let summary = MiniMaxAnalyzer::window_summary_from_response_text(
        ts("2026-05-25T09:00:00Z"),
        ts("2026-05-25T09:05:00Z"),
        &samples,
        None,
        ts("2026-05-25T09:05:10Z"),
        "MiniMax-M3",
        content,
    )
    .unwrap();

    assert_eq!(summary.raw_summary_json["content"], content);
}

fn window_sample(
    screenshot_id: i64,
    minute_mark: u8,
    captured_at: &str,
    process_name: &str,
    window_title: &str,
) -> WindowScreenshotSample {
    WindowScreenshotSample {
        minute_mark,
        screenshot: HighResScreenshotMeta {
            id: screenshot_id,
            captured_at: ts(captured_at),
            file_path: format!("2026-05-25/{screenshot_id}.jpg"),
            width: 1920,
            height: 1080,
            process_name: Some(process_name.to_string()),
            window_title: Some(window_title.to_string()),
            capture_status: "ok".to_string(),
        },
    }
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
