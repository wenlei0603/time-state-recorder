use chrono::{DateTime, Utc};
use tsr_collector::{
    insights::{
        LocalDailyBriefReporter, MiniMaxDailyBriefReporter, MiniMaxInsightConfig,
        MiniMaxInsightReporter, build_five_hour_report,
        build_five_hour_report_from_window_summaries, observation_from_visual_summary,
        select_insight_report_provider,
    },
    models::{
        ActivityCategory, ActivityCategoryCount, DailyActivityStats, DailyAppActivity,
        DailyComparison, HighResScreenshotMeta, HourlyActivityMetric, InsightReport,
        VisualObservation, VisualSummary, VisualTrajectoryPoint, VisualWindowSummary,
    },
    visual_analysis::{
        MiniMaxAnalyzer, MiniMaxConfig, WindowVisualAnalysisInput, WindowVisualAnalysisSample,
        select_window_samples,
    },
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
    assert_eq!(request["max_completion_tokens"], 10_000);
    assert_eq!(request["messages"][1]["role"], "user");
    let prompt = request["messages"][1]["content"].as_str().unwrap();
    assert!(prompt.contains("observations="));
    assert!(prompt.contains("periodStart=2026-06-03T13:00:00+08:00"));
    assert!(prompt.contains("periodEnd=2026-06-03T18:00:00+08:00"));
    assert!(prompt.contains(r#""capturedAt":"2026-06-03T13:05:00+08:00""#));
    assert!(!prompt.contains("2026-06-03T05:00:00Z"));
    assert!(!prompt.contains("2026-06-03T05:05:00Z"));
    assert_eq!(request["thinking"]["type"], "disabled");
}

#[test]
fn minimax_daily_brief_request_defaults_to_ten_thousand_completion_tokens() {
    let reporter = MiniMaxDailyBriefReporter::new(MiniMaxInsightConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));
    let reports = vec![sample_insight_report(
        11,
        "2026-06-03T05:00:00Z",
        "2026-06-03T10:00:00Z",
        "上午先处理课程邮件，随后进入 Overpayment do-file 编码。",
    )];
    let stats = sample_daily_stats();
    let hourly = vec![sample_hourly_metric(9, 1800, vec![11])];
    let comparison = DailyComparison {
        baseline_days: 7,
        compared_dates: vec!["2026-06-02".into()],
        active_seconds_delta: 600,
        switches_per_hour_delta: 0.2,
        input_chars_delta: 120,
        screenshot_coverage_delta: 0.1,
        dominant_category_shift: Some("research -> coding".into()),
        start_time_shift_minutes: Some(-20),
        end_time_shift_minutes: Some(15),
        explanation: "编码相关窗口较前一日增加。".into(),
    };

    let request = reporter.build_chat_completions_request(
        "2026-06-03",
        ts("2026-06-03T00:00:00Z"),
        ts("2026-06-04T00:00:00Z"),
        &stats,
        &hourly,
        &comparison,
        &reports,
    );

    assert_eq!(request["max_completion_tokens"], 10_000);
    let prompt = request["messages"][1]["content"].as_str().unwrap();
    assert!(prompt.contains("parallel projects"));
    assert!(prompt.contains("periodStart=2026-06-03T08:00:00+08:00"));
    assert!(prompt.contains("periodEnd=2026-06-04T08:00:00+08:00"));
    assert!(prompt.contains(r#""firstActivityAt":"2026-06-03T13:00:00+08:00""#));
    assert!(prompt.contains(r#""startAt":"2026-06-03T17:00:00+08:00""#));
    assert!(prompt.contains(r#""periodStart":"2026-06-03T13:00:00+08:00""#));
    assert!(!prompt.contains("2026-06-03T05:00:00Z"));
    assert!(!prompt.contains("2026-06-03T09:00:00Z"));
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
fn selects_first_third_and_fifth_minute_screenshots_for_window() {
    let screenshots = vec![
        high_res_screenshot(1, "2026-06-03T10:00:10Z", "Code.exe"),
        high_res_screenshot(2, "2026-06-03T10:01:00Z", "Terminal.exe"),
        high_res_screenshot(3, "2026-06-03T10:02:05Z", "Code.exe"),
        high_res_screenshot(4, "2026-06-03T10:03:00Z", "msedge.exe"),
        high_res_screenshot(5, "2026-06-03T10:04:15Z", "Code.exe"),
    ];

    let samples = select_window_samples(
        ts("2026-06-03T10:00:00Z"),
        ts("2026-06-03T10:05:00Z"),
        &screenshots,
    )
    .unwrap();

    assert_eq!(
        samples
            .iter()
            .map(|sample| (sample.minute_mark, sample.screenshot.id))
            .collect::<Vec<_>>(),
        vec![(1, 1), (3, 3), (5, 5)]
    );
}

#[test]
fn minimax_window_analysis_request_sends_three_images_and_previous_summary() {
    let dir = tempfile::tempdir().unwrap();
    let image_paths = [1, 3, 5]
        .iter()
        .map(|mark| {
            let path = dir.path().join(format!("minute-{mark}.jpg"));
            std::fs::write(&path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
            path
        })
        .collect::<Vec<_>>();
    let screenshots = vec![
        high_res_screenshot(1, "2026-06-03T10:00:10Z", "Code.exe"),
        high_res_screenshot(3, "2026-06-03T10:02:05Z", "Code.exe"),
        high_res_screenshot(5, "2026-06-03T10:04:15Z", "msedge.exe"),
    ];
    let previous = sample_window_summary(
        40,
        "2026-06-03T09:55:00Z",
        "2026-06-03T10:00:00Z",
        vec![31, 33, 35],
        "上一个窗口在查 MiniMax 文档。",
    );
    let input = WindowVisualAnalysisInput {
        window_start: ts("2026-06-03T10:00:00Z"),
        window_end: ts("2026-06-03T10:05:00Z"),
        samples: vec![
            WindowVisualAnalysisSample {
                minute_mark: 1,
                screenshot: &screenshots[0],
                image_path: image_paths[0].as_path(),
            },
            WindowVisualAnalysisSample {
                minute_mark: 3,
                screenshot: &screenshots[1],
                image_path: image_paths[1].as_path(),
            },
            WindowVisualAnalysisSample {
                minute_mark: 5,
                screenshot: &screenshots[2],
                image_path: image_paths[2].as_path(),
            },
        ],
        previous_summary: Some(&previous),
    };
    let analyzer = MiniMaxAnalyzer::new(MiniMaxConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));

    let request = analyzer
        .build_window_chat_completions_request(&input)
        .unwrap();
    let content = request["messages"][1]["content"].as_array().unwrap();
    let text = content[0]["text"].as_str().unwrap();
    let image_blocks = content
        .iter()
        .filter(|block| block["type"] == "image_url")
        .collect::<Vec<_>>();

    assert!(text.contains("previousWindowSummary"));
    assert!(text.contains("上一个窗口在查 MiniMax 文档"));
    assert!(text.contains("minuteMark"));
    assert_eq!(image_blocks.len(), 3);
    assert!(
        image_blocks
            .iter()
            .all(|block| block["image_url"]["max_long_side_pixel"].is_null())
    );
}

#[test]
fn minimax_window_analysis_response_maps_to_structured_window_summary() {
    let samples = vec![
        high_res_screenshot(1, "2026-06-03T10:00:10Z", "Code.exe"),
        high_res_screenshot(3, "2026-06-03T10:02:05Z", "Code.exe"),
        high_res_screenshot(5, "2026-06-03T10:04:15Z", "msedge.exe"),
    ];

    let summary = MiniMaxAnalyzer::window_summary_from_response_text(
        ts("2026-06-03T10:00:00Z"),
        ts("2026-06-03T10:05:00Z"),
        &select_window_samples(
            ts("2026-06-03T10:00:00Z"),
            ts("2026-06-03T10:05:00Z"),
            &samples,
        )
        .unwrap(),
        Some(40),
        ts("2026-06-03T10:05:30Z"),
        "MiniMax-M3",
        r#"{
          "summaryText": "持续实现 Time State Recorder 的视觉分析 worker。",
          "continuity": "continued_focus",
          "primaryActivity": "coding",
          "projectHints": ["Time State Recorder"],
          "taskIntent": "实现窗口级三图摘要",
          "trajectory": [
            {"minuteMark": 1, "observation": "编辑 Rust worker", "activityCategory": "coding"},
            {"minuteMark": 3, "observation": "查看 MiniMax 请求体", "activityCategory": "coding"},
            {"minuteMark": 5, "observation": "检查前端反馈", "activityCategory": "coding"}
          ],
          "switchingLevel": "low",
          "switchingEvidence": "都围绕同一项目。",
          "loafingLevel": "none",
          "loafingEvidence": "没有娱乐内容。",
          "visibleApps": ["Code.exe", "msedge.exe"],
          "visibleTextHints": ["visual_window_summaries"],
          "riskFlags": [],
          "confidence": 0.86
        }"#,
    )
    .unwrap();

    assert_eq!(summary.previous_summary_id, Some(40));
    assert_eq!(summary.sampled_screenshot_ids, vec![1, 3, 5]);
    assert_eq!(summary.primary_activity, ActivityCategory::Coding);
    assert_eq!(summary.trajectory.len(), 3);
    assert_eq!(summary.trajectory[2].minute_mark, 5);
    assert_eq!(summary.switching_level, "low");
    assert_eq!(summary.loafing_level, "none");
    assert!(summary.raw_summary_json["summaryText"].is_string());
}

#[test]
fn builds_five_hour_report_from_visual_window_summaries() {
    let windows = vec![
        sample_window_summary(
            1,
            "2026-06-03T05:00:00Z",
            "2026-06-03T05:05:00Z",
            vec![1, 3, 5],
            "开始实现窗口级视觉分析 worker。",
        ),
        sample_window_summary(
            2,
            "2026-06-03T05:05:00Z",
            "2026-06-03T05:10:00Z",
            vec![6, 8, 10],
            "继续调试 MiniMax 三图请求。",
        ),
    ];

    let report = build_five_hour_report_from_window_summaries(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &windows,
    );

    assert_eq!(report.report_kind, "5h");
    assert_eq!(report.evidence_count, 2);
    assert_eq!(
        report.category_mix[0].activity_category,
        ActivityCategory::Coding
    );
    assert!(report.summary_text.contains("窗口摘要"));
    assert!(report.summary_text.contains("继续调试 MiniMax 三图请求"));
}

#[test]
fn local_daily_brief_builds_neutral_action_trajectory_from_five_hour_reports() {
    let reporter = LocalDailyBriefReporter;
    let reports = vec![
        sample_insight_report(
            11,
            "2026-06-03T05:00:00Z",
            "2026-06-03T10:00:00Z",
            "上午先处理课程邮件，随后进入 Overpayment do-file 编码。",
        ),
        sample_insight_report(
            12,
            "2026-06-03T10:00:00Z",
            "2026-06-03T15:00:00Z",
            "中午继续检查 R formal analysis 输出并阅读 AMR 论文。",
        ),
    ];
    let stats = sample_daily_stats();
    let hourly = vec![sample_hourly_metric(9, 1800, vec![11])];
    let comparison = DailyComparison {
        baseline_days: 7,
        compared_dates: vec!["2026-06-02".into()],
        active_seconds_delta: 600,
        switches_per_hour_delta: 0.2,
        input_chars_delta: 120,
        screenshot_coverage_delta: 0.1,
        dominant_category_shift: Some("research -> coding".into()),
        start_time_shift_minutes: Some(-20),
        end_time_shift_minutes: Some(15),
        explanation: "编码相关窗口较前一日增加。".into(),
    };

    let brief = reporter
        .report(
            "2026-06-03",
            ts("2026-06-03T00:00:00Z"),
            ts("2026-06-04T00:00:00Z"),
            "23:40",
            &stats,
            &hourly,
            &comparison,
            &reports,
            ts("2026-06-03T15:40:00Z"),
        )
        .unwrap();

    assert_eq!(brief.status, "complete");
    assert_eq!(brief.five_hour_report_ids, vec![11, 12]);
    assert!(brief.daily_summary_text.contains("1.0 小时"));
    assert!(brief.action_trajectory.contains("课程邮件"));
    assert!(brief.action_trajectory.contains("AMR 论文"));
    assert!(!brief.action_trajectory.contains("高效"));
    assert!(!brief.action_trajectory.contains("浪费"));
}

#[test]
fn minimax_insight_report_request_uses_window_summaries() {
    let windows = vec![sample_window_summary(
        1,
        "2026-06-03T05:00:00Z",
        "2026-06-03T05:05:00Z",
        vec![1, 3, 5],
        "开始实现窗口级视觉分析 worker。",
    )];
    let reporter = MiniMaxInsightReporter::new(MiniMaxInsightConfig::new(
        "test-key",
        "https://api.minimax.test/v1",
        "MiniMax-M3",
    ));

    let request = reporter.build_window_summary_chat_completions_request(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &windows,
    );
    let prompt = request["messages"][1]["content"].as_str().unwrap();

    assert!(prompt.contains("windowSummaries="));
    assert!(!prompt.contains("observations="));
    assert!(prompt.contains("switchingLevel"));
    assert!(prompt.contains("loafingLevel"));
    assert!(prompt.contains("periodStart=2026-06-03T13:00:00+08:00"));
    assert!(prompt.contains("periodEnd=2026-06-03T18:00:00+08:00"));
    assert!(prompt.contains(r#""windowStart":"2026-06-03T13:00:00+08:00""#));
    assert!(!prompt.contains("2026-06-03T05:00:00Z"));
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
        identity_tags: vec!["software_builder".into()],
        routine_tags: vec!["coding_build".into()],
        visible_apps: vec!["Code.exe".into()],
        visible_text_hints: vec![],
        risk_flags: vec![],
        confidence: 0.82,
        created_at: ts(captured_at),
        error: None,
    }
}

fn sample_window_summary(
    id: i64,
    window_start: &str,
    window_end: &str,
    sampled_screenshot_ids: Vec<i64>,
    summary_text: &str,
) -> VisualWindowSummary {
    VisualWindowSummary {
        id,
        window_start: ts(window_start),
        window_end: ts(window_end),
        sampled_screenshot_ids: sampled_screenshot_ids.clone(),
        previous_summary_id: None,
        model_provider: "minimax".into(),
        model_name: "MiniMax-M3".into(),
        prompt_version: "visual-window-minimax-m3-v1".into(),
        summary_text: summary_text.into(),
        continuity: "continued_focus".into(),
        primary_activity: ActivityCategory::Coding,
        project_hints: vec!["Time State Recorder".into()],
        identity_tags: vec!["software_builder".into()],
        routine_tags: vec!["coding_build".into()],
        task_intent: "实现窗口级视觉分析".into(),
        trajectory: sampled_screenshot_ids
            .iter()
            .zip([1_u8, 3, 5])
            .map(|(screenshot_id, minute_mark)| VisualTrajectoryPoint {
                minute_mark,
                screenshot_id: *screenshot_id,
                observation: format!("minute {minute_mark}"),
                activity_category: ActivityCategory::Coding,
                project_hints: vec!["Time State Recorder".into()],
                identity_tags: vec!["software_builder".into()],
                routine_tags: vec!["coding_build".into()],
            })
            .collect(),
        switching_level: "low".into(),
        switching_evidence: "窗口切换少。".into(),
        loafing_level: "none".into(),
        loafing_evidence: "未见无关内容。".into(),
        visible_apps: vec!["Code.exe".into()],
        visible_text_hints: vec![],
        risk_flags: vec![],
        confidence: 0.8,
        raw_summary_json: serde_json::json!({ "summaryText": summary_text }),
        created_at: ts(window_end),
        error: None,
    }
}

fn sample_insight_report(id: i64, start: &str, end: &str, summary: &str) -> InsightReport {
    InsightReport {
        id,
        period_start: ts(start),
        period_end: ts(end),
        generated_at: ts(end),
        report_kind: "5h".into(),
        model_provider: "local_insight".into(),
        model_name: "trajectory-v1".into(),
        summary_text: summary.into(),
        category_mix: vec![ActivityCategoryCount {
            activity_category: ActivityCategory::Coding,
            count: 1,
        }],
        project_hints: vec!["Overpayment".into()],
        evidence_count: 1,
        error: None,
    }
}

fn sample_daily_stats() -> DailyActivityStats {
    DailyActivityStats {
        date: "2026-06-03".into(),
        period_start: ts("2026-06-03T00:00:00Z"),
        period_end: ts("2026-06-04T00:00:00Z"),
        active_seconds: 3600,
        active_hours: 1.0,
        window_event_count: 6,
        switch_count: 4,
        distinct_app_count: 3,
        top_apps: vec![DailyAppActivity {
            process_name: "Code.exe".into(),
            active_seconds: 2400,
            share: 0.67,
        }],
        category_mix: vec![ActivityCategoryCount {
            activity_category: ActivityCategory::Coding,
            count: 2,
        }],
        input_chars: 120,
        input_events: 140,
        screenshot_count: 6,
        high_res_screenshot_count: 3,
        visual_window_count: 4,
        five_hour_report_count: 2,
        first_activity_at: Some(ts("2026-06-03T05:00:00Z")),
        last_activity_at: Some(ts("2026-06-03T15:00:00Z")),
    }
}

fn sample_hourly_metric(
    hour: u8,
    active_seconds: i64,
    report_ids: Vec<i64>,
) -> HourlyActivityMetric {
    HourlyActivityMetric {
        hour,
        start_at: ts("2026-06-03T09:00:00Z"),
        end_at: ts("2026-06-03T10:00:00Z"),
        active_seconds,
        active_ratio: active_seconds as f64 / 3600.0,
        window_event_count: 2,
        switch_count: 1,
        distinct_app_count: 2,
        dominant_app: Some("Code.exe".into()),
        dominant_category: ActivityCategory::Coding,
        input_chars: 60,
        screenshot_count: 2,
        high_res_screenshot_count: 1,
        visual_window_count: 1,
        five_hour_report_ids: report_ids,
    }
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
