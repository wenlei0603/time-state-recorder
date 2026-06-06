use std::fs;

use tsr_collector::notion_smoke::{
    DEFAULT_ARTIFACT_FILE_NAME, REQUIRED_MARKDOWN_SECTIONS, SMOKE_DATE,
    run_notion_daily_archive_smoke,
};

#[tokio::test]
async fn smoke_command_writes_agent_consumable_daily_archive_artifact() {
    let temp_dir = tempfile::tempdir().unwrap();
    let artifact_path = temp_dir.path().join(DEFAULT_ARTIFACT_FILE_NAME);

    let summary = run_notion_daily_archive_smoke(&artifact_path)
        .await
        .unwrap();

    assert_eq!(summary.date, SMOKE_DATE);
    assert_eq!(summary.five_hour_report_count, 2);
    assert_eq!(
        summary.markdown_sections_checked,
        REQUIRED_MARKDOWN_SECTIONS
    );
    assert_eq!(summary.artifact_path, artifact_path);

    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&summary.artifact_path).unwrap()).unwrap();

    assert_eq!(body["date"], SMOKE_DATE);
    assert_eq!(body["dailyDiaryTitle"], "INDEX-20260524 | Daily Diary");
    assert_eq!(body["source"]["endpoint"], "/api/notion/daily-archive");
    assert_eq!(body["source"]["app"], "time-state-recorder");
    assert_eq!(body["descriptiveStats"]["fiveHourReportCount"], 2);
    assert_eq!(body["hourlyMetrics"][0]["fiveHourReportIds"][0], 1);

    let reports = body["fiveHourReports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| report["reportKind"] == "5h"));
    assert!(
        reports
            .iter()
            .any(|report| report["summaryText"] == "Morning build report.")
    );
    assert!(
        reports
            .iter()
            .any(|report| report["summaryText"] == "Afternoon documentation report.")
    );
    assert!(
        !reports
            .iter()
            .any(|report| report["summaryText"] == "Previous-day report should be excluded.")
    );

    let markdown = body["archiveMarkdown"].as_str().unwrap();
    for section in REQUIRED_MARKDOWN_SECTIONS {
        assert!(markdown.contains(section), "missing section {section}");
    }
    assert!(markdown.contains("Morning build report."));
    assert!(markdown.contains("Afternoon documentation report."));
    assert!(!markdown.contains("Previous-day report should be excluded."));
}
