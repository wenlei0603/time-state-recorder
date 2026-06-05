use chrono::{DateTime, Utc};
use rusqlite::params;
use tsr_collector::{
    interval::build_time_events_with_lifecycle,
    models::{
        ActivityCategory, ActivityCategoryCount, CaptureStatus, DailyActivityStats,
        DailyAppActivity, DailyBrief, DailyComparison, HighResScreenshotMeta, HourlyActivityMetric,
        InsightReport, LifecycleType, ScreenshotMeta, VisualObservation, VisualSummary,
        VisualTrajectoryPoint, VisualWindowSummary, WindowSnapshot,
    },
    storage::Store,
};

#[test]
fn persists_window_focus_events_in_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_window_focus(
            &session_id,
            &WindowSnapshot {
                captured_at: ts("2026-05-23T09:00:00Z"),
                hwnd: 100,
                pid: 42,
                process_name: "Code.exe".to_string(),
                exe_path_hash: Some("abc123".to_string()),
                window_title: Some("main.rs".to_string()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();

    let rows = store.list_window_events(10).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, session_id);
    assert_eq!(rows[0].process_name, "Code.exe");
    assert_eq!(rows[0].window_title.as_deref(), Some("main.rs"));
    assert_eq!(rows[0].capture_status, CaptureStatus::Ok);
}

#[test]
fn lists_latest_window_focus_events_in_chronological_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for minute in 0..3 {
        store
            .insert_window_focus(
                &session_id,
                &WindowSnapshot {
                    captured_at: ts(&format!("2026-05-23T09:0{minute}:00Z")),
                    hwnd: minute + 100,
                    pid: minute as u32 + 42,
                    process_name: format!("App-{minute}"),
                    exe_path_hash: None,
                    window_title: Some(format!("Title {minute}")),
                    capture_status: CaptureStatus::Ok,
                },
            )
            .unwrap();
    }

    let rows = store.list_window_events(2).unwrap();

    assert_eq!(
        rows.iter()
            .map(|row| row.process_name.as_str())
            .collect::<Vec<_>>(),
        vec!["App-1", "App-2"]
    );
}

#[test]
fn persists_screenshot_metadata_for_session() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-23T09:01:00Z"),
                file_path: "2026-05-23/09-01.jpg".into(),
                width: 640,
                height: 360,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let rows = store.list_screenshots_by_date("2026-05-23", 10).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].file_path, "2026-05-23/09-01.jpg");
    assert_eq!(rows[0].process_name.as_deref(), Some("Code.exe"));
}

#[test]
fn persists_high_res_screenshot_metadata_by_date_without_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-05-23T09:05:00Z", "2026-05-23/09-05.jpg", "ok"),
        ("2026-05-23T09:10:00Z", "", "idle"),
    ] {
        store
            .insert_high_res_screenshot(
                &session_id,
                &HighResScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 1920 } else { 0 },
                    height: if capture_status == "ok" { 1080 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let rows = store
        .list_high_res_screenshots_by_date("2026-05-23", 10)
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].file_path, "2026-05-23/09-05.jpg");
    assert_eq!(rows[0].width, 1920);
    assert_eq!(rows[0].height, 1080);
    assert_eq!(rows[0].capture_status, "ok");
}

#[test]
fn visual_window_summaries_round_trip_by_window() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    let mut screenshot_ids = Vec::new();
    for (minute, mark) in [("00", 1), ("02", 3), ("04", 5)] {
        let id = store
            .insert_high_res_screenshot(
                &session_id,
                &HighResScreenshotMeta {
                    id: 0,
                    captured_at: ts(&format!("2026-06-03T10:{minute}:00Z")),
                    file_path: format!("2026-06-03/10-{minute}-00.jpg"),
                    width: 1600,
                    height: 1000,
                    process_name: Some("Code.exe".into()),
                    window_title: Some(format!("activity minute {mark}")),
                    capture_status: "ok".into(),
                },
            )
            .unwrap();
        screenshot_ids.push((id, mark));
    }

    let mut summary = VisualWindowSummary {
        id: 0,
        window_start: ts("2026-06-03T10:00:00Z"),
        window_end: ts("2026-06-03T10:05:00Z"),
        sampled_screenshot_ids: screenshot_ids.iter().map(|(id, _)| *id).collect(),
        previous_summary_id: None,
        model_provider: "minimax".into(),
        model_name: "MiniMax-M3".into(),
        prompt_version: "visual-window-minimax-m3-v1".into(),
        summary_text: "5 分钟内持续实现后端 worker，并检查前端反馈。".into(),
        continuity: "continued_focus".into(),
        primary_activity: ActivityCategory::Coding,
        project_hints: vec!["Time State Recorder".into()],
        identity_tags: vec!["unknown".into()],
        routine_tags: vec!["unknown".into()],
        task_intent: "实现窗口级视觉分析链路".into(),
        trajectory: screenshot_ids
            .iter()
            .map(|(id, mark)| VisualTrajectoryPoint {
                minute_mark: *mark,
                screenshot_id: *id,
                observation: format!("第 {mark} 分钟仍在处理 Rust/React 代码"),
                activity_category: ActivityCategory::Coding,
                project_hints: vec!["Time State Recorder".into()],
                identity_tags: vec!["software_builder".into()],
                routine_tags: vec!["coding_build".into()],
            })
            .collect(),
        switching_level: "low".into(),
        switching_evidence: "三张图都围绕同一项目代码窗口。".into(),
        loafing_level: "none".into(),
        loafing_evidence: "未看到娱乐或无关浏览内容。".into(),
        visible_apps: vec!["Code.exe".into(), "msedge.exe".into()],
        visible_text_hints: vec!["visual_window_summaries".into()],
        risk_flags: vec![],
        confidence: 0.82,
        raw_summary_json: serde_json::json!({
            "summaryText": "5 分钟内持续实现后端 worker，并检查前端反馈。"
        }),
        created_at: ts("2026-06-03T10:05:15Z"),
        error: None,
    };

    summary.id = store.insert_visual_window_summary(&summary).unwrap();
    let rows = store
        .list_visual_window_summaries_between(
            ts("2026-06-03T10:00:00Z"),
            ts("2026-06-03T10:10:00Z"),
            10,
        )
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], summary);
    assert_eq!(rows[0].sampled_screenshot_ids.len(), 3);
    assert_eq!(
        rows[0]
            .trajectory
            .iter()
            .map(|point| point.minute_mark)
            .collect::<Vec<_>>(),
        vec![1, 3, 5]
    );
}

#[test]
fn visual_summaries_round_trip_by_date() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    let screenshot_id = store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-23T09:01:00Z"),
                file_path: "2026-05-23/09-01.jpg".into(),
                width: 1280,
                height: 720,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let inserted = store
        .insert_visual_summary(&VisualSummary {
            id: 0,
            screenshot_id,
            captured_at: ts("2026-05-23T09:01:00Z"),
            model_provider: "local_stub".into(),
            model_name: "metadata-v1".into(),
            prompt_version: "visual-summary-v1".into(),
            summary_text: "Code editor focused on main.rs".into(),
            activity_category: ActivityCategory::Coding,
            project_hints: vec!["Time State Recorder".into()],
            identity_tags: vec!["unknown".into()],
            routine_tags: vec!["unknown".into()],
            visible_apps: vec!["Code.exe".into()],
            visible_text_hints: vec!["main.rs".into()],
            risk_flags: vec![],
            confidence: 0.65,
            created_at: ts("2026-05-23T09:02:00Z"),
            error: None,
        })
        .unwrap();

    let rows = store
        .list_visual_summaries_by_date("2026-05-23", 10)
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, inserted);
    assert_eq!(rows[0].screenshot_id, screenshot_id);
    assert_eq!(rows[0].model_provider, "local_stub");
    assert_eq!(rows[0].activity_category, ActivityCategory::Coding);
    assert_eq!(rows[0].project_hints, vec!["Time State Recorder"]);
    assert_eq!(rows[0].visible_apps, vec!["Code.exe"]);
    assert_eq!(rows[0].visible_text_hints, vec!["main.rs"]);
    assert_eq!(rows[0].summary_text, "Code editor focused on main.rs");
}

#[test]
fn screenshot_queries_support_utc_day_windows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-06-03T15:59:00Z", "outside-before.jpg", "ok"),
        ("2026-06-03T16:00:00Z", "inside-start.jpg", "ok"),
        ("2026-06-04T15:59:00Z", "inside-end.jpg", "ok"),
        ("2026-06-04T16:00:00Z", "outside-after.jpg", "ok"),
        ("2026-06-04T10:00:00Z", "", "idle"),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 640 } else { 0 },
                    height: if capture_status == "ok" { 360 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
        store
            .insert_high_res_screenshot(
                &session_id,
                &HighResScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 1440 } else { 0 },
                    height: if capture_status == "ok" { 900 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let start = ts("2026-06-03T16:00:00Z");
    let end = ts("2026-06-04T16:00:00Z");

    let screenshots = store.list_screenshots_between(start, end, 10).unwrap();
    assert_eq!(
        screenshots
            .iter()
            .map(|item| item.file_path.as_str())
            .collect::<Vec<_>>(),
        vec!["inside-start.jpg", "inside-end.jpg"]
    );

    let high_res = store
        .list_high_res_screenshots_between(start, end, 10)
        .unwrap();
    assert_eq!(high_res.len(), 2);
    assert_eq!(high_res[0].file_path, "inside-start.jpg");

    let summary = store
        .get_screenshot_summary_between("2026-06-04", start, end)
        .unwrap();
    assert_eq!(summary.date, "2026-06-04");
    assert_eq!(summary.total_screenshots, 2);
    assert_eq!(summary.skipped_reasons.len(), 1);
    assert_eq!(summary.skipped_reasons[0].reason, "idle");
}

#[test]
fn visual_summary_queries_support_utc_day_windows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    let mut screenshot_ids = Vec::new();
    for (captured_at, file_path) in [
        ("2026-06-03T15:59:00Z", "outside-before.jpg"),
        ("2026-06-03T16:00:00Z", "inside-start.jpg"),
        ("2026-06-04T15:59:00Z", "inside-end.jpg"),
        ("2026-06-04T16:00:00Z", "outside-after.jpg"),
    ] {
        let screenshot_id = store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: 1280,
                    height: 720,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: "ok".into(),
                },
            )
            .unwrap();
        screenshot_ids.push((screenshot_id, captured_at));
    }

    for (screenshot_id, captured_at) in screenshot_ids {
        store
            .insert_visual_summary(&VisualSummary {
                id: 0,
                screenshot_id,
                captured_at: ts(captured_at),
                model_provider: "local_stub".into(),
                model_name: "metadata-v1".into(),
                prompt_version: "visual-summary-v1".into(),
                summary_text: format!("Summary at {captured_at}"),
                activity_category: ActivityCategory::Coding,
                project_hints: vec!["Time State Recorder".into()],
                identity_tags: vec!["unknown".into()],
                routine_tags: vec!["unknown".into()],
                visible_apps: vec!["Code.exe".into()],
                visible_text_hints: vec!["main.rs".into()],
                risk_flags: vec![],
                confidence: 0.65,
                created_at: ts(captured_at),
                error: None,
            })
            .unwrap();
    }

    let rows = store
        .list_visual_summaries_between(ts("2026-06-03T16:00:00Z"), ts("2026-06-04T16:00:00Z"), 10)
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows[0].summary_text.contains("2026-06-03T16:00:00Z"));
    assert!(rows[1].summary_text.contains("2026-06-04T15:59:00Z"));
}

#[test]
fn visual_observations_round_trip_by_high_res_screenshot() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    let high_res_id = store
        .insert_high_res_screenshot(
            &session_id,
            &HighResScreenshotMeta {
                id: 0,
                captured_at: ts("2026-06-03T10:05:00Z"),
                file_path: "2026-06-03/10-05-00.jpg".into(),
                width: 1600,
                height: 1000,
                process_name: Some("Code.exe".into()),
                window_title: Some("analysis worker".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let observation_id = store
        .insert_visual_observation(&VisualObservation {
            id: 0,
            high_res_screenshot_id: high_res_id,
            captured_at: ts("2026-06-03T10:05:00Z"),
            file_path: "2026-06-03/10-05-00.jpg".into(),
            model_provider: "minimax".into(),
            model_name: "MiniMax-M3".into(),
            prompt_version: "visual-summary-minimax-m3-v1".into(),
            summary_text: "正在实现自动视觉分析 worker".into(),
            activity_category: ActivityCategory::Coding,
            project_hints: vec!["Time State Recorder".into()],
            identity_tags: vec!["unknown".into()],
            routine_tags: vec!["unknown".into()],
            visible_apps: vec!["Code.exe".into()],
            visible_text_hints: vec!["analysis worker".into()],
            risk_flags: vec![],
            confidence: 0.86,
            created_at: ts("2026-06-03T10:05:20Z"),
            error: None,
        })
        .unwrap();

    let rows = store
        .list_visual_observations_between(
            ts("2026-06-03T10:00:00Z"),
            ts("2026-06-03T10:10:00Z"),
            10,
        )
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, observation_id);
    assert_eq!(rows[0].high_res_screenshot_id, high_res_id);
    assert_eq!(rows[0].summary_text, "正在实现自动视觉分析 worker");
}

#[test]
fn insight_reports_round_trip_by_period() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();

    let report_id = store
        .insert_insight_report(&InsightReport {
            id: 0,
            period_start: ts("2026-06-03T05:00:00Z"),
            period_end: ts("2026-06-03T10:00:00Z"),
            generated_at: ts("2026-06-03T10:01:00Z"),
            report_kind: "5h".into(),
            model_provider: "local_insight".into(),
            model_name: "trajectory-v1".into(),
            summary_text: "5 小时内主要在实现 Time State Recorder。".into(),
            category_mix: vec![
                ActivityCategoryCount {
                    activity_category: ActivityCategory::Coding,
                    count: 3,
                },
                ActivityCategoryCount {
                    activity_category: ActivityCategory::Research,
                    count: 1,
                },
            ],
            project_hints: vec!["Time State Recorder".into()],
            evidence_count: 4,
            error: None,
        })
        .unwrap();

    let reports = store.list_insight_reports(5).unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].id, report_id);
    assert_eq!(reports[0].report_kind, "5h");
    assert_eq!(reports[0].evidence_count, 4);
    assert_eq!(
        reports[0].category_mix[0].activity_category,
        ActivityCategory::Coding
    );
    assert_eq!(reports[0].category_mix[0].count, 3);
}

#[test]
fn lists_insight_reports_between_chronologically_for_selected_day() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();

    for (start, end, summary) in [
        (
            "2026-06-02T19:00:00Z",
            "2026-06-03T00:30:00Z",
            "跨入目标日期的夜间报告。",
        ),
        (
            "2026-06-03T05:00:00Z",
            "2026-06-03T10:00:00Z",
            "上午编码报告。",
        ),
        (
            "2026-06-04T00:30:00Z",
            "2026-06-04T05:00:00Z",
            "下一日报告。",
        ),
    ] {
        store
            .insert_insight_report(&sample_insight_report(start, end, summary))
            .unwrap();
    }

    let reports = store
        .list_insight_reports_between(
            ts("2026-06-03T00:00:00Z"),
            ts("2026-06-04T00:00:00Z"),
            Some("5h"),
            10,
        )
        .unwrap();

    assert_eq!(
        reports
            .iter()
            .map(|report| report.summary_text.as_str())
            .collect::<Vec<_>>(),
        vec!["跨入目标日期的夜间报告。", "上午编码报告。"]
    );
}

#[test]
fn daily_brief_round_trips_with_hourly_metrics_and_report_ids() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();

    let brief = DailyBrief {
        id: 0,
        date: "2026-06-03".into(),
        period_start: ts("2026-06-03T00:00:00Z"),
        period_end: ts("2026-06-04T00:00:00Z"),
        generated_at: ts("2026-06-03T15:40:05Z"),
        scheduled_for_local: "23:40".into(),
        model_provider: "local_insight".into(),
        model_name: "daily-brief-local-v1".into(),
        prompt_version: "daily-brief-v1".into(),
        status: "complete".into(),
        descriptive_stats: sample_daily_stats(),
        hourly_metrics: vec![sample_hourly_metric(9, 1800, vec![11, 12])],
        comparison: DailyComparison {
            baseline_days: 7,
            compared_dates: vec!["2026-06-02".into()],
            active_seconds_delta: 600,
            switches_per_hour_delta: -0.5,
            input_chars_delta: 120,
            screenshot_coverage_delta: 0.2,
            dominant_category_shift: Some("research -> coding".into()),
            start_time_shift_minutes: Some(-15),
            end_time_shift_minutes: Some(30),
            explanation: "编码窗口较前一日增加。".into(),
        },
        five_hour_report_ids: vec![11, 12],
        daily_summary_text: "今日桌面记录显示编码和阅读交替出现。".into(),
        action_trajectory: "上午以编码为主，随后出现阅读材料窗口。".into(),
        raw_summary_json: serde_json::json!({
            "dailySummaryText": "今日桌面记录显示编码和阅读交替出现。",
            "actionTrajectory": "上午以编码为主，随后出现阅读材料窗口。"
        }),
        error: None,
    };

    let id = store.insert_daily_brief(&brief).unwrap();
    let loaded = store
        .get_daily_brief_by_date("2026-06-03", "23:40")
        .unwrap()
        .unwrap();

    assert_eq!(loaded.id, id);
    assert_eq!(loaded.status, "complete");
    assert_eq!(loaded.descriptive_stats.active_seconds, 3600);
    assert_eq!(loaded.hourly_metrics[0].five_hour_report_ids, vec![11, 12]);
    assert_eq!(loaded.comparison.explanation, "编码窗口较前一日增加。");
    assert_eq!(
        loaded.action_trajectory,
        "上午以编码为主，随后出现阅读材料窗口。"
    );
}

#[test]
fn daily_activity_stats_do_not_extend_closed_sessions_into_selected_day() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let old_session = store.create_session("0.1.0", "old").unwrap();
    store
        .insert_window_focus(
            &old_session,
            &WindowSnapshot {
                captured_at: ts("2026-06-03T10:00:00Z"),
                hwnd: 10,
                pid: 10,
                process_name: "OldApp.exe".into(),
                exe_path_hash: None,
                window_title: Some("old work".into()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();
    store
        .close_session(&old_session, ts("2026-06-03T10:10:00Z"), "service_stop")
        .unwrap();

    let current_session = store.create_session("0.1.0", "current").unwrap();
    store
        .insert_window_focus(
            &current_session,
            &WindowSnapshot {
                captured_at: ts("2026-06-04T00:00:00Z"),
                hwnd: 20,
                pid: 20,
                process_name: "Code.exe".into(),
                exe_path_hash: None,
                window_title: Some("current work".into()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();
    store
        .insert_window_focus(
            &current_session,
            &WindowSnapshot {
                captured_at: ts("2026-06-04T00:30:00Z"),
                hwnd: 21,
                pid: 21,
                process_name: "Browser.exe".into(),
                exe_path_hash: None,
                window_title: Some("current reading".into()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();
    store
        .close_session(&current_session, ts("2026-06-04T01:00:00Z"), "service_stop")
        .unwrap();

    let stats = store
        .build_daily_activity_stats(
            "2026-06-04",
            ts("2026-06-04T00:00:00Z"),
            ts("2026-06-05T00:00:00Z"),
            &[],
        )
        .unwrap();

    assert_eq!(stats.active_seconds, 3600);
    assert_eq!(stats.distinct_app_count, 2);
    assert!(
        !stats
            .top_apps
            .iter()
            .any(|app| app.process_name == "OldApp.exe")
    );
}

#[test]
fn daily_activity_stats_do_not_extend_open_sessions_into_future_hours() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "future").unwrap();
    store
        .insert_window_focus(
            &session_id,
            &WindowSnapshot {
                captured_at: ts("2099-01-01T00:00:00Z"),
                hwnd: 30,
                pid: 30,
                process_name: "FutureApp.exe".into(),
                exe_path_hash: None,
                window_title: Some("future work".into()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();

    let stats = store
        .build_daily_activity_stats(
            "2099-01-01",
            ts("2099-01-01T00:00:00Z"),
            ts("2099-01-02T00:00:00Z"),
            &[],
        )
        .unwrap();

    assert_eq!(stats.active_seconds, 0);
    assert_eq!(stats.active_hours, 0.0);
}

#[test]
fn persists_lifecycle_events_in_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:05:00Z"),
            LifecycleType::WindowsLock,
            Some("manual_lock"),
            serde_json::json!({"source": "test"}),
        )
        .unwrap();
    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:20:00Z"),
            LifecycleType::WindowsUnlock,
            None,
            serde_json::json!({}),
        )
        .unwrap();

    let rows = store.list_lifecycle_events(10).unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].session_id, session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::WindowsLock);
    assert_eq!(rows[0].reason.as_deref(), Some("manual_lock"));
    assert_eq!(rows[0].payload["source"], "test");
    assert_eq!(rows[1].lifecycle_type, LifecycleType::WindowsUnlock);
}

#[test]
fn closes_stale_sessions_as_abnormal_stop_with_collector_gap() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let stale_session_id = store.create_session("0.1.0", "test-config").unwrap();

    let closed = store
        .close_stale_sessions(ts("2026-05-23T10:00:00Z"), "abnormal_stop")
        .unwrap();

    assert_eq!(closed, vec![stale_session_id.clone()]);
    let rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, stale_session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::CollectorGap);
    assert_eq!(rows[0].reason.as_deref(), Some("abnormal_stop"));
}

#[test]
fn closes_stale_sessions_at_last_recorded_event_boundary() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let stale_session_id = store.create_session("0.1.0", "test-config").unwrap();
    store
        .insert_window_focus(
            &stale_session_id,
            &WindowSnapshot {
                captured_at: ts("2026-05-23T18:00:00Z"),
                hwnd: 100,
                pid: 42,
                process_name: "Code.exe".to_string(),
                exe_path_hash: None,
                window_title: Some("main.rs".to_string()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();

    store
        .close_stale_sessions(ts("2026-05-24T09:00:00Z"), "abnormal_stop")
        .unwrap();

    let lifecycle_rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(lifecycle_rows[0].event_ts, ts("2026-05-23T18:00:00Z"));

    let window_rows = store.list_window_events(10).unwrap();
    let intervals = build_time_events_with_lifecycle(&window_rows, &lifecycle_rows);
    assert_eq!(intervals[0].duration_seconds, Some(0));
}

#[test]
fn close_session_records_one_terminal_transition() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .close_session(&session_id, ts("2026-05-23T10:00:00Z"), "completed")
        .unwrap();
    assert!(
        store
            .close_session(&session_id, ts("2026-05-23T10:01:00Z"), "completed")
            .is_err()
    );

    let lifecycle_rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(lifecycle_rows.len(), 1);
    assert_eq!(lifecycle_rows[0].lifecycle_type, LifecycleType::SessionStop);
}

#[test]
fn rejects_session_scoped_writes_after_session_close() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .close_session(&session_id, ts("2026-05-23T10:00:00Z"), "completed")
        .unwrap();

    assert!(
        store
            .insert_window_focus(
                &session_id,
                &WindowSnapshot {
                    captured_at: ts("2026-05-23T10:00:01Z"),
                    hwnd: 100,
                    pid: 42,
                    process_name: "Code.exe".to_string(),
                    exe_path_hash: None,
                    window_title: Some("late.rs".to_string()),
                    capture_status: CaptureStatus::Ok,
                },
            )
            .is_err()
    );

    assert!(
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts("2026-05-23T10:00:02Z"),
                    file_path: "2026-05-23/10-00.jpg".into(),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("late.rs".into()),
                    capture_status: "ok".into(),
                },
            )
            .is_err()
    );

    assert!(
        store
            .insert_lifecycle_event(
                &session_id,
                ts("2026-05-23T10:00:03Z"),
                LifecycleType::WindowsLock,
                None,
                serde_json::json!({}),
            )
            .is_err()
    );
}

#[test]
fn rejects_unknown_lifecycle_types_from_storage() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("unknown-lifecycle.sqlite3");
    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    drop(store);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute(
        r#"
        INSERT INTO raw_events
          (session_id, event_ts, event_type, source, target_window_id, payload_json)
        VALUES (?1, ?2, 'lifecycle', 'test', NULL, '{}')
        "#,
        params![session_id, "2026-05-23T09:00:00Z"],
    )
    .unwrap();
    let raw_event_id = conn.last_insert_rowid();
    conn.execute(
        r#"
        INSERT INTO lifecycle_events
          (raw_event_id, lifecycle_type, reason, active_session_id, payload_json)
        VALUES (?1, 'future_shutdown', NULL, ?2, '{}')
        "#,
        params![raw_event_id, session_id],
    )
    .unwrap();
    drop(conn);

    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();

    assert!(store.list_lifecycle_events(10).is_err());
}

#[test]
fn screenshot_summary_counts_skipped_reasons() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (minute, status) in [
        ("00", "ok"),
        ("01", "idle"),
        ("02", "blocked"),
        ("03", "capture_unavailable"),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(&format!("2026-05-24T09:{minute}:00Z")),
                    file_path: format!("2026-05-24/09-{minute}.jpg"),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: status.into(),
                },
            )
            .unwrap();
    }

    let summary = store.get_screenshot_summary("2026-05-24").unwrap();

    assert_eq!(summary.total_screenshots, 1);
    assert_eq!(summary.skipped_reasons.len(), 3);
    assert_eq!(summary.skipped_reasons[0].reason, "blocked");
    assert_eq!(summary.skipped_reasons[0].count, 1);
    assert_eq!(summary.skipped_reasons[1].reason, "capture_unavailable");
    assert_eq!(summary.skipped_reasons[2].reason, "idle");
}

#[test]
fn screenshot_reads_and_success_summary_ignore_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, process_name, window_title, capture_status) in [
        (
            "2026-05-24T09:00:00Z",
            "2026-05-24/09-00.jpg",
            Some("Code.exe"),
            Some("main.rs"),
            "ok",
        ),
        ("2026-05-24T10:00:00Z", "", None, None, "idle"),
        (
            "2026-05-24T11:00:00Z",
            "",
            Some("Secret.exe"),
            Some("Sensitive window"),
            "blocked",
        ),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 640 } else { 0 },
                    height: if capture_status == "ok" { 360 } else { 0 },
                    process_name: process_name.map(String::from),
                    window_title: window_title.map(String::from),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let screenshots = store.list_screenshots_by_date("2026-05-24", 10).unwrap();
    assert_eq!(screenshots.len(), 1);
    assert_eq!(screenshots[0].capture_status, "ok");

    let summary = store.get_screenshot_summary("2026-05-24").unwrap();
    assert_eq!(summary.total_screenshots, 1);
    assert_eq!(summary.hours_covered, 1);
    assert_eq!(summary.top_apps.len(), 1);
    assert_eq!(summary.top_apps[0].process_name, "Code.exe");
    assert_eq!(summary.skipped_reasons.len(), 2);
    assert_eq!(summary.skipped_reasons[0].reason, "blocked");
    assert_eq!(summary.skipped_reasons[0].count, 1);
    assert_eq!(summary.skipped_reasons[1].reason, "idle");
    assert_eq!(summary.skipped_reasons[1].count, 1);

    let stats = store.get_db_stats().unwrap();
    assert_eq!(stats.screenshots, 1);
}

fn sample_insight_report(start: &str, end: &str, summary: &str) -> InsightReport {
    InsightReport {
        id: 0,
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
        project_hints: vec!["Time State Recorder".into()],
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
        window_event_count: 3,
        switch_count: 2,
        distinct_app_count: 2,
        top_apps: vec![DailyAppActivity {
            process_name: "Code.exe".into(),
            active_seconds: 3000,
            share: 0.83,
        }],
        category_mix: vec![ActivityCategoryCount {
            activity_category: ActivityCategory::Coding,
            count: 2,
        }],
        input_chars: 120,
        input_events: 140,
        screenshot_count: 4,
        high_res_screenshot_count: 2,
        visual_window_count: 3,
        five_hour_report_count: 2,
        first_activity_at: Some(ts("2026-06-03T09:00:00Z")),
        last_activity_at: Some(ts("2026-06-03T18:00:00Z")),
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
