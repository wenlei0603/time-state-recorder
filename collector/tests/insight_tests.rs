use chrono::{DateTime, Utc};
use tsr_collector::{
    insights::{
        MiniMaxInsightConfig, MiniMaxInsightReporter, build_five_hour_report,
        observation_from_visual_summary, select_insight_report_provider,
    },
    models::{ActivityCategory, HighResScreenshotMeta, VisualObservation, VisualSummary},
};

#[test]
fn converts_high_res_visual_summary_to_observation() {
    let high_res = high_res_screenshot(7, "2026-06-03T10:05:00Z", "Code.exe");
    let summary = visual_summary(
        99,
        "2026-06-03T10:05:00Z",
        ActivityCategory::Coding,
        "写代码",
    );

    let observation = observation_from_visual_summary(&high_res, &summary);

    assert_eq!(observation.high_res_screenshot_id, 7);
    assert_eq!(observation.file_path, "2026-06-03/10-05-00.jpg");
    assert_eq!(observation.model_provider, "minimax");
    assert_eq!(observation.activity_category, ActivityCategory::Coding);
    assert_eq!(observation.summary_text, "写代码");
}

#[test]
fn builds_five_hour_report_from_visual_observations() {
    let observations = vec![
        observation_from_visual_summary(
            &high_res_screenshot(1, "2026-06-03T05:05:00Z", "Code.exe"),
            &visual_summary(
                1,
                "2026-06-03T05:05:00Z",
                ActivityCategory::Coding,
                "实现后端 worker",
            ),
        ),
        observation_from_visual_summary(
            &high_res_screenshot(2, "2026-06-03T06:05:00Z", "msedge.exe"),
            &visual_summary(
                2,
                "2026-06-03T06:05:00Z",
                ActivityCategory::Research,
                "阅读 MiniMax 文档",
            ),
        ),
        observation_from_visual_summary(
            &high_res_screenshot(3, "2026-06-03T07:05:00Z", "Code.exe"),
            &visual_summary(
                3,
                "2026-06-03T07:05:00Z",
                ActivityCategory::Coding,
                "写前端反馈面板",
            ),
        ),
    ];

    let report = build_five_hour_report(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &observations,
    );

    assert_eq!(report.report_kind, "5h");
    assert_eq!(report.evidence_count, 3);
    assert_eq!(
        report.category_mix[0].activity_category,
        ActivityCategory::Coding
    );
    assert_eq!(report.category_mix[0].count, 2);
    assert!(report.summary_text.contains("实现后端 worker"));
    assert!(report.summary_text.contains("写前端反馈面板"));
}

#[test]
fn minimax_insight_report_uses_text_chat_completions() {
    let observations = sample_ascii_observations();
    let reporter = MiniMaxInsightReporter::new(MiniMaxInsightConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));

    let request = reporter.build_chat_completions_request(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &observations,
    );

    assert_eq!(request["model"], "MiniMax-M3");
    assert_eq!(request["messages"][1]["role"], "user");
    assert!(
        request["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("observations=")
    );
    assert_eq!(request["thinking"]["type"], "disabled");
}

#[test]
fn minimax_insight_response_maps_to_five_hour_report() {
    let observations = sample_ascii_observations();

    let report = MiniMaxInsightReporter::report_from_response_text(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &observations,
        ts("2026-06-03T10:01:00Z"),
        "MiniMax-M3",
        r#"{
          "summaryText": "The user moved from backend worker implementation to frontend feedback.",
          "projectHints": ["Time State Recorder"]
        }"#,
    )
    .unwrap();

    assert_eq!(report.model_provider, "minimax");
    assert_eq!(report.model_name, "MiniMax-M3");
    assert_eq!(report.evidence_count, 3);
    assert_eq!(
        report.category_mix[0].activity_category,
        ActivityCategory::Coding
    );
    assert!(report.summary_text.contains("backend worker"));
}

#[test]
fn insight_report_provider_defaults_to_minimax_when_credentials_exist() {
    assert_eq!(
        select_insight_report_provider(None, Some("secret"), Some("https://api.minimax.test")),
        "minimax"
    );
    assert_eq!(
        select_insight_report_provider(None, Some("secret"), None),
        "local"
    );
    assert_eq!(
        select_insight_report_provider(
            Some("local"),
            Some("secret"),
            Some("https://api.minimax.test")
        ),
        "local"
    );
}

fn sample_ascii_observations() -> Vec<VisualObservation> {
    vec![
        observation_from_visual_summary(
            &high_res_screenshot(10, "2026-06-03T05:05:00Z", "Code.exe"),
            &visual_summary(
                10,
                "2026-06-03T05:05:00Z",
                ActivityCategory::Coding,
                "Implemented the backend visual analysis worker.",
            ),
        ),
        observation_from_visual_summary(
            &high_res_screenshot(11, "2026-06-03T06:05:00Z", "msedge.exe"),
            &visual_summary(
                11,
                "2026-06-03T06:05:00Z",
                ActivityCategory::Research,
                "Read MiniMax OpenAI-compatible API docs.",
            ),
        ),
        observation_from_visual_summary(
            &high_res_screenshot(12, "2026-06-03T07:05:00Z", "Code.exe"),
            &visual_summary(
                12,
                "2026-06-03T07:05:00Z",
                ActivityCategory::Coding,
                "Built the frontend feedback panel.",
            ),
        ),
    ]
}

fn high_res_screenshot(id: i64, captured_at: &str, app: &str) -> HighResScreenshotMeta {
    let captured_at = ts(captured_at);
    HighResScreenshotMeta {
        id,
        captured_at,
        file_path: captured_at.format("%Y-%m-%d/%H-%M-%S.jpg").to_string(),
        width: 1600,
        height: 1000,
        process_name: Some(app.into()),
        window_title: Some("Time State Recorder".into()),
        capture_status: "ok".into(),
    }
}

fn visual_summary(
    screenshot_id: i64,
    captured_at: &str,
    activity_category: ActivityCategory,
    summary_text: &str,
) -> VisualSummary {
    VisualSummary {
        id: 0,
        screenshot_id,
        captured_at: ts(captured_at),
        model_provider: "minimax".into(),
        model_name: "MiniMax-M3".into(),
        prompt_version: "visual-summary-minimax-m3-v1".into(),
        summary_text: summary_text.into(),
        activity_category,
        project_hints: vec!["Time State Recorder".into()],
        visible_apps: vec!["Code.exe".into()],
        visible_text_hints: vec![],
        risk_flags: vec![],
        confidence: 0.82,
        created_at: ts(captured_at),
        error: None,
    }
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
