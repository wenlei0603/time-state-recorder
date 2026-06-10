# Hourly And Fixed Five-Hour Reports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add top-of-hour 1-hour reports, keep 5-hour reports, and switch 5-hour generation from rolling windows to fixed local windows starting on 2026-06-07: 10:00-15:00, 15:00-20:00, and 20:00-01:00.

**Architecture:** Reuse the existing `insight_reports` table and `InsightReport` model. Add `report_kind = "1h"` for hourly reports and keep `report_kind = "5h"` for five-hour reports. Both kinds are generated from `visual_window_summaries`; Daily Brief and Notion archive responses keep `fiveHourReports` and add `hourlyReports` so existing consumers are not silently repointed. UTC remains a storage/API machine-field detail only; every human-readable report prompt, fallback summary, Notion markdown line, and frontend card must render Asia/Shanghai UTC+8 local time. The database may keep coarse merged model text, but the frontend must never render report text as one unbounded paragraph; it converts reports into a structured presentation model before display.

**Tech Stack:** Rust/Axum collector, SQLite storage through `rusqlite`, Chrono local/UTC conversion, React/TypeScript/Vite frontend, Vitest and Rust integration tests.

---

## Requirements And Assumptions

- 1-hour reports start as soon as this version runs. "整点总结" means at or after `HH:00`, generate the completed local hour `[HH-1:00, HH:00)`.
- 5-hour rolling reports remain only for dates before local date `2026-06-07`.
- Starting local date `2026-06-07`, 5-hour reports use fixed owner-date slots:
  - `10:00-15:00`
  - `15:00-20:00`
  - `20:00-01:00` where the `01:00` belongs to the next local calendar day but the slot owner date is the date of `20:00`.
- Both 1h and 5h reports are derived from existing 5-minute `visual_window_summaries`, not raw screenshots or raw window events.
- The owner timezone for human interpretation is Asia/Shanghai UTC+8. Do not use UTC clock labels in report text, Daily Brief text, Notion archive markdown, or frontend cards.
- UTC timestamps may remain in persisted `DateTime<Utc>` values and JSON machine fields such as `periodStart`, `periodEnd`, and `generatedAt`. Those fields must be converted at every human-visible boundary.
- MiniMax prompts must send local fields such as `localPeriodStart`, `localPeriodEnd`, `localWindowStart`, and `localWindowEnd`; prompt instructions must explicitly say summary text must not emit `Z`, `UTC`, or `+00:00` time labels.
- `InsightReport.reportKind` values are exactly `"1h"` and `"5h"`.
- The API field `fiveHourReports` remains unchanged. Add `hourlyReports` beside it.
- `DailyBrief.fiveHourReportIds`, `DailyActivityStats.fiveHourReportCount`, and `HourlyActivityMetric.fiveHourReportIds` stay as-is for backward compatibility. The new hourly reports are displayed in the response/UI but are not stored inside `DailyBrief`.
- Frontend report presentation is structured for all three report levels: 1-hour reports, scheduled 5-hour reports, and Daily Brief narrative. The UI must show time range, summary, phases/projects, evidence metadata, uncertainty/risks, and collapsed raw text, not the screenshot's current wall-of-text layout.
- The structured frontend model is deterministic and UI-only. It should prefer structured fields if future API responses add them, but for this implementation it derives readable sections from existing `summaryText`, `dailySummaryText`, `actionTrajectory`, `categoryMix`, `projectHints`, `evidenceCount`, and timestamps.
- Report cards must cap visible text lengths and use progressive disclosure. Long raw report text belongs in a collapsed details block.
- Redacted mode still hides narrative text. It may show time range, report kind, evidence counts, category/project chips, and section labels.
- The work happens in `D:\CodexInfra\docs\projects\time-state-recorder\.worktrees\report-cadence-20260607` on branch `codex/report-cadence-20260607`.

## File Structure

- Modify `collector/src/api.rs`
  - Add report cadence constants and candidate-period helpers.
  - Replace the single rolling `maybe_generate_insight_report` path with a due-period generator that handles `"1h"` and `"5h"` independently.
  - Add `hourly_reports` to `DailyBriefResponse` and `NotionDailyArchiveResponse`.
  - Include hourly reports in Notion archive markdown.
- Modify `collector/src/insights.rs`
  - Add generic report builders that accept `report_kind`.
  - Keep existing five-hour builder wrappers so old tests and call sites remain readable.
  - Update MiniMax prompt text to describe 1-hour or 5-hour windows correctly.
  - Use UTC+8 local clock labels in local fallback Daily Brief action trajectories.
- Modify `collector/src/prompt_time.rs`
  - Expose shared UTC+8 human-report timestamp and range helpers.
  - Keep persisted UTC values unchanged while preventing UTC labels from leaking into prompts or markdown.
- Modify `collector/src/storage.rs`
  - Add `insight_report_exists(kind, start, end)`.
  - Keep the existing `insight_reports` schema; do not add a table or migration.
- Modify `collector/tests/insight_tests.rs`
  - Add hourly report builder tests.
  - Update MiniMax prompt tests for report-kind-aware prompts.
  - Assert report prompts, Daily Brief prompts, and local fallback text use UTC+8 local time labels.
- Modify `collector/tests/storage_tests.rs`
  - Add exact-kind/exact-period report-existence tests.
- Modify `collector/tests/api_tests.rs`
  - Add API coverage for `kind=1h`.
  - Add `hourlyReports` coverage in `/api/daily-brief` and `/api/notion/daily-archive`.
  - Assert Notion archive markdown uses UTC+8 local report ranges and contains no `Z` or `+00:00` report labels.
- Modify `src/types.ts`
  - Add `hourlyReports: InsightReport[]` to `DailyBriefResponse`.
- Modify `src/lib/dailyBrief.ts`
  - Parse `hourlyReports`.
- Modify `src/lib/dailyBrief.test.ts`
  - Verify hourly reports parse and existing 5h reports still parse.
- Create `src/lib/reportPresentation.ts`
  - Convert `InsightReport` and `DailyBrief` text into a bounded UI presentation model.
  - Extract numbered phases, time-span lines, project hints, evidence chips, uncertainty statements, and raw text fallback.
  - Format report ranges with `Asia/Shanghai` explicitly and normalize visible legacy UTC timestamp tokens into local clock labels.
- Create `src/lib/reportPresentation.test.ts`
  - Verify long LLM text is split into readable sections and raw text remains collapsed-only.
- Create `src/StructuredReportCard.tsx`
  - Reusable report card for 1h and 5h reports.
  - Shows overview, phase bullets, evidence chips, and collapsed raw text.
- Create `src/StructuredDailyNarrative.tsx`
  - Reusable structured renderer for Daily Brief `dailySummaryText` and `actionTrajectory`.
- Modify `src/DailyBriefPanel.tsx`
  - Show an "Hourly Reports" section above scheduled 5h reports.
  - Rename the 5h heading to "Scheduled 5h Reports".
  - Use structured report cards instead of rendering `report.summaryText` directly.
  - Use structured daily narrative instead of rendering the daily summary/action trajectory as plain paragraphs.
- Modify `src/App.test.tsx`
  - Update daily brief fixtures with `hourlyReports`.
- Modify `src/styles.css`
  - Add compact structured report card, phase list, evidence chip, and raw details styles.
- Modify `docs/api/notion-daily-archive.md`
  - Document the new `hourlyReports` field and markdown section.

---

### Task 1: Add Report Cadence Helpers

**Files:**
- Modify: `collector/src/api.rs`

- [ ] **Step 1: Write failing cadence unit tests**

Add this module near the bottom of `collector/src/api.rs`:

```rust
#[cfg(test)]
mod report_cadence_tests {
    use super::*;
    use chrono::{LocalResult, NaiveDate, TimeZone};

    fn local_ts(value: &str) -> DateTime<Local> {
        let naive = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap();
        match Local.from_local_datetime(&naive) {
            LocalResult::Single(value) => value,
            LocalResult::Ambiguous(value, _) => value,
            LocalResult::None => panic!("test timestamp cannot be represented in local timezone"),
        }
    }

    #[test]
    fn hourly_candidate_uses_previous_complete_local_hour() {
        let now = local_ts("2026-06-06 15:03:10");

        let period = completed_hourly_period(now).unwrap();

        assert_eq!(period.kind, HOURLY_REPORT_KIND);
        assert_eq!(period.period_start.with_timezone(&Local).format("%H:%M").to_string(), "14:00");
        assert_eq!(period.period_end.with_timezone(&Local).format("%H:%M").to_string(), "15:00");
    }

    #[test]
    fn fixed_five_hour_candidates_start_on_cutoff_date() {
        let cutoff = NaiveDate::from_ymd_opt(2026, 6, 7).unwrap();

        let before = completed_five_hour_periods(local_ts("2026-06-06 20:01:00"), cutoff);
        let after = completed_five_hour_periods(local_ts("2026-06-07 20:01:00"), cutoff);

        assert!(before.is_empty());
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].kind, FIVE_HOUR_REPORT_KIND);
        assert_eq!(after[0].period_start.with_timezone(&Local).format("%H:%M").to_string(), "10:00");
        assert_eq!(after[0].period_end.with_timezone(&Local).format("%H:%M").to_string(), "15:00");
        assert_eq!(after[1].period_start.with_timezone(&Local).format("%H:%M").to_string(), "15:00");
        assert_eq!(after[1].period_end.with_timezone(&Local).format("%H:%M").to_string(), "20:00");
    }

    #[test]
    fn fixed_five_hour_cross_midnight_slot_belongs_to_previous_owner_date() {
        let cutoff = NaiveDate::from_ymd_opt(2026, 6, 7).unwrap();

        let periods = completed_five_hour_periods(local_ts("2026-06-08 01:02:00"), cutoff);

        let last = periods.last().unwrap();
        assert_eq!(last.period_start.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(), "2026-06-07 20:00");
        assert_eq!(last.period_end.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(), "2026-06-08 01:00");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector report_cadence_tests
```

Expected: FAIL because `HOURLY_REPORT_KIND`, `FIVE_HOUR_REPORT_KIND`, `completed_hourly_period`, and `completed_five_hour_periods` do not exist.

- [ ] **Step 3: Add cadence constants and helpers**

Add near the current interval constants in `collector/src/api.rs`:

```rust
const HOURLY_REPORT_KIND: &str = "1h";
const FIVE_HOUR_REPORT_KIND: &str = "5h";
const HOURLY_REPORT_INTERVAL: i64 = 60 * 60;
const FIVE_HOUR_REPORT_INTERVAL: i64 = 5 * 60 * 60;
const REPORT_GENERATION_BATCH_LIMIT: usize = 6;
const FIXED_FIVE_HOUR_START_DATE: &str = "2026-06-07";
const FIXED_FIVE_HOUR_SLOTS: [(u32, u32, u32, u32); 3] = [
    (10, 0, 15, 0),
    (15, 0, 20, 0),
    (20, 0, 1, 0),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportPeriod {
    kind: &'static str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
}
```

Add below `floor_to_visual_window`:

```rust
fn report_cadence_cutoff_date() -> NaiveDate {
    NaiveDate::parse_from_str(FIXED_FIVE_HOUR_START_DATE, "%Y-%m-%d")
        .expect("fixed five-hour cutoff date must be valid")
}

fn completed_hourly_period(now: DateTime<Local>) -> Option<ReportPeriod> {
    let date = now.date_naive();
    let hour_start = date.and_hms_opt(now.hour(), 0, 0)?;
    let local_end = Local.from_local_datetime(&hour_start).earliest()?;
    let local_start = local_end - chrono::Duration::seconds(HOURLY_REPORT_INTERVAL);
    Some(ReportPeriod {
        kind: HOURLY_REPORT_KIND,
        period_start: local_start.with_timezone(&Utc),
        period_end: local_end.with_timezone(&Utc),
    })
}

fn completed_five_hour_periods(
    now: DateTime<Local>,
    cutoff_date: NaiveDate,
) -> Vec<ReportPeriod> {
    let today = now.date_naive();
    let mut periods = Vec::new();
    for owner_date in [today.pred_opt(), Some(today)].into_iter().flatten() {
        if owner_date < cutoff_date {
            continue;
        }
        for (start_hour, start_minute, end_hour, end_minute) in FIXED_FIVE_HOUR_SLOTS {
            let Some(local_start) = local_time_on_date(owner_date, start_hour, start_minute) else {
                continue;
            };
            let end_date = if end_hour < start_hour {
                owner_date.succ_opt()
            } else {
                Some(owner_date)
            };
            let Some(end_date) = end_date else {
                continue;
            };
            let Some(local_end) = local_time_on_date(end_date, end_hour, end_minute) else {
                continue;
            };
            if now >= local_end {
                periods.push(ReportPeriod {
                    kind: FIVE_HOUR_REPORT_KIND,
                    period_start: local_start.with_timezone(&Utc),
                    period_end: local_end.with_timezone(&Utc),
                });
            }
        }
    }
    periods.sort_by_key(|period| period.period_end);
    periods.dedup_by(|left, right| {
        left.kind == right.kind
            && left.period_start == right.period_start
            && left.period_end == right.period_end
    });
    periods
}

fn local_time_on_date(date: NaiveDate, hour: u32, minute: u32) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    Local.from_local_datetime(&naive).earliest()
}
```

Add `Timelike` to the chrono import at the top:

```rust
use chrono::{DateTime, FixedOffset, Local, NaiveDate, TimeZone, Timelike, Utc};
```

- [ ] **Step 4: Run cadence tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector report_cadence_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/api.rs
git commit -m "feat: define report cadence windows"
```

---

### Task 2: Add Storage Idempotency By Kind And Exact Period

**Files:**
- Modify: `collector/src/storage.rs`
- Modify: `collector/tests/storage_tests.rs`

- [ ] **Step 1: Write failing storage test**

Add this test in `collector/tests/storage_tests.rs`:

```rust
#[test]
fn detects_existing_insight_report_by_kind_and_exact_period() {
    let mut store = Store::in_memory().unwrap();
    let report = sample_insight_report(
        0,
        "2026-06-07T02:00:00Z",
        "2026-06-07T03:00:00Z",
        "1h",
        "整点小时报告。",
    );
    store.insert_insight_report(&report).unwrap();

    assert!(
        store
            .insight_report_exists(
                "1h",
                ts("2026-06-07T02:00:00Z"),
                ts("2026-06-07T03:00:00Z"),
            )
            .unwrap()
    );
    assert!(
        !store
            .insight_report_exists(
                "5h",
                ts("2026-06-07T02:00:00Z"),
                ts("2026-06-07T03:00:00Z"),
            )
            .unwrap()
    );
}
```

Update the local `sample_insight_report` helper in that test file to accept `report_kind: &str` if it currently hardcodes `"5h"`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test storage_tests detects_existing_insight_report_by_kind_and_exact_period
```

Expected: FAIL because `Store::insight_report_exists` does not exist.

- [ ] **Step 3: Implement storage helper**

Add this method near the existing `insert_insight_report` / `list_insight_reports_between` methods in `collector/src/storage.rs`:

```rust
pub fn insight_report_exists(
    &self,
    kind: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<bool> {
    let exists = self
        .conn
        .query_row(
            r#"
            SELECT 1
            FROM insight_reports
            WHERE report_kind = ?1
              AND period_start = ?2
              AND period_end = ?3
            LIMIT 1
            "#,
            params![kind, start.to_rfc3339(), end.to_rfc3339()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}
```

- [ ] **Step 4: Run storage test**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test storage_tests detects_existing_insight_report_by_kind_and_exact_period
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/storage.rs collector/tests/storage_tests.rs
git commit -m "feat: check report existence by cadence"
```

---

### Task 3: Make Insight Report Builders Report-Kind Aware

**Files:**
- Modify: `collector/src/insights.rs`
- Modify: `collector/tests/insight_tests.rs`

- [ ] **Step 1: Write failing hourly builder test**

Add this test after `builds_five_hour_report_from_visual_window_summaries`:

```rust
#[test]
fn builds_hourly_report_from_visual_window_summaries() {
    let windows = vec![
        sample_window_summary(
            1,
            "2026-06-07T02:00:00Z",
            "2026-06-07T02:05:00Z",
            vec![1, 3, 5],
            "整点后开始整理前端计划。",
        ),
        sample_window_summary(
            2,
            "2026-06-07T02:55:00Z",
            "2026-06-07T03:00:00Z",
            vec![6, 8, 10],
            "完成后端报告 cadence 设计。",
        ),
    ];

    let report = build_hourly_report_from_window_summaries(
        ts("2026-06-07T02:00:00Z"),
        ts("2026-06-07T03:00:00Z"),
        &windows,
    );

    assert_eq!(report.report_kind, "1h");
    assert_eq!(report.evidence_count, 2);
    assert!(report.summary_text.contains("1 小时"));
    assert!(report.summary_text.contains("完成后端报告 cadence 设计"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test insight_tests builds_hourly_report_from_visual_window_summaries
```

Expected: FAIL because `build_hourly_report_from_window_summaries` does not exist.

- [ ] **Step 3: Add generic local builders**

In `collector/src/insights.rs`, add:

```rust
pub fn build_hourly_report_from_window_summaries(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> InsightReport {
    build_report_from_window_summaries("1h", period_start, period_end, window_summaries)
}

pub fn build_report_from_window_summaries(
    report_kind: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> InsightReport {
    let category_mix = category_mix_from_window_summaries(window_summaries);
    let project_hints = top_project_hints_from_window_summaries(window_summaries);
    let summary_text = window_report_summary_text(report_kind, window_summaries, &category_mix);

    InsightReport {
        id: 0,
        period_start,
        period_end,
        generated_at: Utc::now(),
        report_kind: report_kind.into(),
        model_provider: "local_insight".into(),
        model_name: LOCAL_REPORT_PROMPT_VERSION.into(),
        summary_text,
        category_mix,
        project_hints,
        evidence_count: window_summaries.len(),
        error: None,
    }
}
```

Change `build_five_hour_report_from_window_summaries` to call the generic builder:

```rust
pub fn build_five_hour_report_from_window_summaries(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> InsightReport {
    build_report_from_window_summaries("5h", period_start, period_end, window_summaries)
}
```

Change `window_report_summary_text` signature and duration text:

```rust
fn window_report_summary_text(
    report_kind: &str,
    window_summaries: &[VisualWindowSummary],
    category_mix: &[ActivityCategoryCount],
) -> String {
    let duration_label = match report_kind {
        "1h" => "1 小时",
        "5h" => "5 小时",
        other => other,
    };
    if window_summaries.is_empty() {
        return format!("{duration_label}内没有可用的 5 分钟窗口摘要，暂时无法推断工作轨迹。");
    }
    let dominant = category_mix
        .first()
        .map(|item| item.activity_category.as_str())
        .unwrap_or(ActivityCategory::Unknown.as_str());
    let first = window_summaries
        .first()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");
    let last = window_summaries
        .last()
        .map(|item| item.summary_text.as_str())
        .unwrap_or("");
    format!(
        "{duration_label}内共分析 {} 条 5 分钟窗口摘要，主要活动类型是 {}。起点：{}。最近状态：{}。",
        window_summaries.len(),
        dominant,
        first,
        last
    )
}
```

- [ ] **Step 4: Update reporter methods to accept `report_kind`**

Change `ConfiguredInsightReporter::report_from_window_summaries`, `LocalInsightReporter::report_from_window_summaries`, `MiniMaxInsightReporter::build_window_summary_chat_completions_request`, `MiniMaxInsightReporter::report_from_window_summaries`, and `MiniMaxInsightReporter::report_from_window_summary_response_text` to include `report_kind: &str`.

Use this local reporter implementation:

```rust
pub fn report_from_window_summaries(
    &self,
    report_kind: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    window_summaries: &[VisualWindowSummary],
) -> Result<InsightReport> {
    Ok(build_report_from_window_summaries(
        report_kind,
        period_start,
        period_end,
        window_summaries,
    ))
}
```

Use this MiniMax prompt call shape:

```rust
let body = self.build_window_summary_chat_completions_request(
    report_kind,
    period_start,
    period_end,
    window_summaries,
);
```

Use this response mapping:

```rust
let local = build_report_from_window_summaries(
    report_kind,
    period_start,
    period_end,
    window_summaries,
);
Ok(InsightReport {
    id: 0,
    period_start,
    period_end,
    generated_at,
    report_kind: report_kind.into(),
    model_provider: "minimax".into(),
    model_name: model_name.into(),
    summary_text: parsed
        .as_ref()
        .and_then(|value| value.summary_text.clone())
        .unwrap_or_else(|| content.trim().to_string()),
    category_mix: local.category_mix,
    project_hints: parsed
        .as_ref()
        .and_then(|value| value.project_hints.clone())
        .unwrap_or(local.project_hints),
    evidence_count: window_summaries.len(),
    error: None,
})
```

- [ ] **Step 5: Update prompt text**

Change `window_summary_report_prompt` to accept `report_kind: &str` and use this instruction:

```rust
let duration_label = match report_kind {
    "1h" => "1-hour",
    "5h" => "5-hour",
    other => other,
};
format!(
    "Infer the user's work trajectory for this {duration_label} report from 5-minute structured summaries. Return JSON only with keys summaryText and projectHints. summaryText must be complete, structured, human-readable Chinese and cover: project-based work path, time allocation, possible loafing, switching frequency, long-run pattern, and uncertainty. Do not produce a 5-minute log. reportKind={}, periodStart={}, periodEnd={}, windowSummaries={}",
    report_kind,
    minimax_prompt_timestamp(period_start),
    minimax_prompt_timestamp(period_end),
    serde_json::to_string(&summaries_json).unwrap_or_else(|_| "[]".to_string())
)
```

Update existing MiniMax tests so 5h calls pass `"5h"` and add an assertion that a 1h request contains `reportKind=1h`.

- [ ] **Step 6: Run insight tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test insight_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add collector/src/insights.rs collector/tests/insight_tests.rs
git commit -m "feat: build hourly insight reports"
```

---

### Task 4: Generate Due Hourly And Fixed Five-Hour Reports

**Files:**
- Modify: `collector/src/api.rs`
- Modify: `collector/tests/api_tests.rs`

- [ ] **Step 1: Write failing API test for hourly report query**

Add this test near `serves_insight_reports` in `collector/tests/api_tests.rs`:

```rust
#[tokio::test]
async fn serves_hourly_insight_reports_by_date_and_kind() {
    let mut store = Store::in_memory().unwrap();
    store
        .insert_insight_report(&InsightReport {
            id: 0,
            period_start: ts("2026-06-07T02:00:00Z"),
            period_end: ts("2026-06-07T03:00:00Z"),
            generated_at: ts("2026-06-07T03:00:05Z"),
            report_kind: "1h".into(),
            model_provider: "local_insight".into(),
            model_name: "trajectory-v1".into(),
            summary_text: "整点小时报告。".into(),
            category_mix: vec![ActivityCategoryCount {
                activity_category: ActivityCategory::Coding,
                count: 12,
            }],
            project_hints: vec!["Time State Recorder".into()],
            evidence_count: 12,
            error: None,
        })
        .unwrap();
    store
        .insert_insight_report(&sample_insight_report(
            0,
            "2026-06-07T02:00:00Z",
            "2026-06-07T07:00:00Z",
            "5小时报告。",
        ))
        .unwrap();
    let addr = spawn_test_server(store).await;

    let body: serde_json::Value = reqwest::get(format!(
        "http://{addr}/api/insight-reports?date=2026-06-07&kind=1h&limit=10"
    ))
    .await
    .unwrap()
    .json()
    .await
    .unwrap();

    assert_eq!(body["reports"].as_array().unwrap().len(), 1);
    assert_eq!(body["reports"][0]["reportKind"], "1h");
    assert_eq!(body["reports"][0]["summaryText"], "整点小时报告。");
}
```

If `sample_insight_report` in this file hardcodes an `id`, keep it for 5h reports and use the explicit struct for 1h.

- [ ] **Step 2: Run API test**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_hourly_insight_reports_by_date_and_kind
```

Expected: PASS after the explicit hourly fixture is inserted. The route already accepts `kind`, so this test documents the new `"1h"` contract before the scheduler changes.

- [ ] **Step 3: Replace single rolling report generation with due-period generation**

Replace `maybe_generate_insight_report` with:

```rust
async fn maybe_generate_due_insight_reports(
    state: &AppState,
    now: DateTime<Utc>,
) -> Result<Vec<InsightReport>> {
    let mut generated = Vec::new();
    for period in due_report_periods(state, now)? {
        if let Some(report) = generate_insight_report_for_period(state, &period).await? {
            generated.push(report);
        }
        if generated.len() >= REPORT_GENERATION_BATCH_LIMIT {
            break;
        }
    }
    Ok(generated)
}

fn due_report_periods(state: &AppState, now: DateTime<Utc>) -> Result<Vec<ReportPeriod>> {
    let now_local = now.with_timezone(&Local);
    let mut candidates = Vec::new();
    if let Some(hourly) = completed_hourly_period(now_local) {
        candidates.push(hourly);
    }
    candidates.extend(completed_five_hour_periods(now_local, report_cadence_cutoff_date()));
    candidates.sort_by_key(|period| period.period_end);

    let store = state
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
    let mut due = Vec::new();
    for period in candidates {
        if !store.insight_report_exists(period.kind, period.period_start, period.period_end)? {
            due.push(period);
        }
    }
    Ok(due)
}

async fn generate_insight_report_for_period(
    state: &AppState,
    period: &ReportPeriod,
) -> Result<Option<InsightReport>> {
    let window_summaries = {
        let store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        store
            .list_visual_window_summaries_between(period.period_start, period.period_end, 1000)?
            .into_iter()
            .filter(|summary| summary.error.is_none())
            .collect::<Vec<_>>()
    };
    if window_summaries.is_empty() {
        return Ok(None);
    }

    let mut report = ConfiguredInsightReporter::from_env()?
        .report_from_window_summaries(
            period.kind,
            period.period_start,
            period.period_end,
            &window_summaries,
        )
        .await?;
    let report_id = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("store lock poisoned"))?;
        store.insert_insight_report(&report)?
    };
    report.id = report_id;
    Ok(Some(report))
}
```

Update `spawn_insight_report_loop`:

```rust
match maybe_generate_due_insight_reports(&state, started_at).await {
    Ok(reports) if !reports.is_empty() => {
        let latest = reports.last().cloned().expect("reports is not empty");
        let next_report_at = next_report_status_time(Utc::now());
        update_report_success(&state, Utc::now(), next_report_at, latest);
    }
    Ok(_) => {
        if let Ok(mut status) = state.analysis_status.lock() {
            status.report.status = "idle".into();
            status.report.next_run_at = Some(next_report_status_time(Utc::now()));
        }
    }
    Err(error) => {
        update_report_error(&state, Utc::now(), next_run_at, format!("{error:#}"));
    }
}
```

Add:

```rust
fn next_report_status_time(now: DateTime<Utc>) -> DateTime<Utc> {
    let now_local = now.with_timezone(&Local);
    let next_hour = now_local
        .date_naive()
        .and_hms_opt(now_local.hour(), 0, 0)
        .and_then(|value| value.checked_add_signed(chrono::Duration::hours(1)))
        .and_then(|value| Local.from_local_datetime(&value).earliest())
        .unwrap_or_else(|| now_local + chrono::Duration::seconds(INSIGHT_REPORT_CHECK_INTERVAL as i64));
    next_hour.with_timezone(&Utc)
}
```

- [ ] **Step 4: Run collector API and insight tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_hourly_insight_reports_by_date_and_kind
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test insight_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/api.rs collector/tests/api_tests.rs
git commit -m "feat: schedule hourly and fixed five hour reports"
```

---

### Task 5: Add Hourly Reports To Daily Brief And Notion API Responses

**Files:**
- Modify: `collector/src/api.rs`
- Modify: `collector/tests/api_tests.rs`
- Modify: `collector/tests/notion_daily_archive_smoke_tests.rs`
- Modify: `docs/api/notion-daily-archive.md`

- [ ] **Step 1: Write failing Daily Brief API assertion**

In `serves_daily_brief_response_with_stats_and_same_day_reports`, insert an hourly report and assert `hourlyReports`:

```rust
let hourly_report_id = store
    .insert_insight_report(&InsightReport {
        id: 0,
        period_start: ts("2026-05-24T09:00:00Z"),
        period_end: ts("2026-05-24T10:00:00Z"),
        generated_at: ts("2026-05-24T10:00:05Z"),
        report_kind: "1h".into(),
        model_provider: "local_insight".into(),
        model_name: "trajectory-v1".into(),
        summary_text: "09点小时报告。".into(),
        category_mix: vec![ActivityCategoryCount {
            activity_category: ActivityCategory::Coding,
            count: 12,
        }],
        project_hints: vec!["Time State Recorder".into()],
        evidence_count: 12,
        error: None,
    })
    .unwrap();
assert!(hourly_report_id > 0);
```

After the response body assertions, add:

```rust
assert_eq!(body["hourlyReports"].as_array().unwrap().len(), 1);
assert_eq!(body["hourlyReports"][0]["reportKind"], "1h");
assert_eq!(body["hourlyReports"][0]["summaryText"], "09点小时报告。");
assert_eq!(body["fiveHourReports"].as_array().unwrap().len(), 2);
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_daily_brief_response_with_stats_and_same_day_reports
```

Expected: FAIL because `hourlyReports` is absent.

- [ ] **Step 3: Add response fields**

In `collector/src/api.rs`, update both response structs:

```rust
struct DailyBriefResponse {
    date: String,
    status: String,
    next_run_at: Option<DateTime<Utc>>,
    brief: Option<DailyBrief>,
    hourly_reports: Vec<InsightReport>,
    five_hour_reports: Vec<InsightReport>,
    descriptive_stats: DailyActivityStats,
    hourly_metrics: Vec<HourlyActivityMetric>,
    comparison: DailyComparison,
}

struct NotionDailyArchiveResponse {
    date: String,
    generated_at: DateTime<Utc>,
    archive_title: String,
    daily_diary_title: String,
    source: NotionArchiveSource,
    status: String,
    archive_markdown: String,
    brief: Option<DailyBrief>,
    hourly_reports: Vec<InsightReport>,
    five_hour_reports: Vec<InsightReport>,
    descriptive_stats: DailyActivityStats,
    hourly_metrics: Vec<HourlyActivityMetric>,
    comparison: DailyComparison,
}
```

In `build_daily_brief_response`, load hourly reports beside 5h reports:

```rust
let hourly_reports = store.list_insight_reports_between(
    date_window.start_utc,
    date_window.end_utc,
    Some(HOURLY_REPORT_KIND),
    500,
)?;
let five_hour_reports = store.list_insight_reports_between(
    date_window.start_utc,
    date_window.end_utc,
    Some(FIVE_HOUR_REPORT_KIND),
    100,
)?;
```

Use `five_hour_reports` for `build_daily_activity_stats`, `build_hourly_activity_metrics`, `ConfiguredDailyBriefReporter::report`, and `five_hour_reports` response field. Add `hourly_reports` to the returned `DailyBriefResponse`.

In `build_notion_daily_archive_response`, pass and return `response.hourly_reports`.

- [ ] **Step 4: Add hourly markdown section**

Change `render_notion_archive_markdown` signature:

```rust
fn render_notion_archive_markdown(
    date: &str,
    archive_title: &str,
    daily_diary_title: &str,
    brief: Option<&DailyBrief>,
    hourly_reports: &[InsightReport],
    five_hour_reports: &[InsightReport],
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    comparison: &DailyComparison,
) -> String
```

Add this section before the existing 5-hour report section:

```rust
lines.push(String::new());
lines.push("## Hourly Reports".into());
if hourly_reports.is_empty() {
    lines.push("- No hourly reports for this date yet.".into());
} else {
    for report in hourly_reports {
        lines.push(format!(
            "- {} - {}: {}",
            report.period_start.with_timezone(&Local).format("%H:%M"),
            report.period_end.with_timezone(&Local).format("%H:%M"),
            report.summary_text
        ));
    }
}
```

Keep the existing 5-hour section and rename its heading to:

```rust
lines.push("## Scheduled 5h Reports".into());
```

- [ ] **Step 5: Update docs**

In `docs/api/notion-daily-archive.md`, add:

```markdown
- `hourlyReports`: same-day 1-hour reports generated at top-of-hour boundaries from 5-minute visual window summaries.
- `fiveHourReports`: same-day scheduled 5-hour reports. Starting 2026-06-07 local date, expected slots are 10:00-15:00, 15:00-20:00, and 20:00-01:00.
```

Add a note that `archiveMarkdown` contains separate `Hourly Reports` and `Scheduled 5h Reports` sections.

Add a note that `periodStart`, `periodEnd`, and `generatedAt` remain UTC JSON machine fields, while `archiveMarkdown` and any human-facing report text use Asia/Shanghai UTC+8 local clock labels.

- [ ] **Step 6: Run API and Notion smoke tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_daily_brief_response_with_stats_and_same_day_reports serves_notion_daily_archive_with_human_readable_markdown
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test notion_daily_archive_smoke_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add collector/src/api.rs collector/tests/api_tests.rs collector/tests/notion_daily_archive_smoke_tests.rs docs/api/notion-daily-archive.md
git commit -m "feat: expose hourly reports in daily brief"
```

---

### Task 6: Enforce UTC+8 Human Report Time Semantics

**Files:**
- Modify: `collector/src/prompt_time.rs`
- Modify: `collector/src/insights.rs`
- Modify: `collector/src/api.rs`
- Modify: `collector/tests/insight_tests.rs`
- Modify: `collector/tests/api_tests.rs`

- [ ] **Step 1: Write failing shared time helper tests**

In `collector/src/prompt_time.rs`, add:

```rust
#[cfg(test)]
mod prompt_time_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn human_report_times_use_owner_utc_plus_eight_clock() {
        let start = Utc.with_ymd_and_hms(2026, 6, 7, 2, 0, 0).single().unwrap();
        let end = Utc.with_ymd_and_hms(2026, 6, 7, 7, 0, 0).single().unwrap();

        assert_eq!(human_report_timestamp(start), "2026-06-07T10:00:00+08:00");
        assert_eq!(human_report_clock(start), "10:00");
        assert_eq!(human_report_range(start, end), "10:00-15:00");
        assert_eq!(human_report_timezone_label(), "Asia/Shanghai UTC+8");
        assert_eq!(minimax_prompt_timestamp(start), human_report_timestamp(start));
    }
}
```

- [ ] **Step 2: Run helper test to verify it fails**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector prompt_time_tests
```

Expected: FAIL because `human_report_timestamp`, `human_report_clock`, `human_report_range`, and `human_report_timezone_label` do not exist.

- [ ] **Step 3: Implement shared human-report time helpers**

Replace `collector/src/prompt_time.rs` with:

```rust
use chrono::{DateTime, FixedOffset, Utc};

const OWNER_LOCAL_UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;
const OWNER_LOCAL_TIMEZONE_LABEL: &str = "Asia/Shanghai UTC+8";

pub(crate) fn minimax_prompt_timestamp(value: DateTime<Utc>) -> String {
    human_report_timestamp(value)
}

pub(crate) fn human_report_timestamp(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&owner_local_offset())
        .format("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

pub(crate) fn human_report_clock(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&owner_local_offset())
        .format("%H:%M")
        .to_string()
}

pub(crate) fn human_report_range(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    format!("{}-{}", human_report_clock(start), human_report_clock(end))
}

pub(crate) fn human_report_timezone_label() -> &'static str {
    OWNER_LOCAL_TIMEZONE_LABEL
}

fn owner_local_offset() -> FixedOffset {
    FixedOffset::east_opt(OWNER_LOCAL_UTC_OFFSET_SECONDS)
        .expect("UTC+8 offset must be valid")
}
```

Keep the test module from Step 1 at the bottom of the file.

- [ ] **Step 4: Write failing insight prompt and fallback assertions**

In `collector/tests/insight_tests.rs`, update `minimax_insight_report_uses_text_chat_completions` to require local field names and a no-UTC instruction:

```rust
assert!(prompt.contains("Asia/Shanghai UTC+8"));
assert!(prompt.contains("localPeriodStart=2026-06-03T13:00:00+08:00"));
assert!(prompt.contains("localPeriodEnd=2026-06-03T18:00:00+08:00"));
assert!(prompt.contains(r#""capturedAt":"2026-06-03T13:05:00+08:00""#));
assert!(prompt.contains("must not emit UTC, Z, or +00:00"));
assert!(!prompt.contains("periodStart=2026-06-03T05:00:00Z"));
```

In `minimax_insight_report_request_uses_window_summaries`, add:

```rust
assert!(prompt.contains("Asia/Shanghai UTC+8"));
assert!(prompt.contains("localPeriodStart=2026-06-03T13:00:00+08:00"));
assert!(prompt.contains("localPeriodEnd=2026-06-03T18:00:00+08:00"));
assert!(prompt.contains(r#""localWindowStart":"2026-06-03T13:00:00+08:00""#));
assert!(prompt.contains(r#""localWindowEnd":"2026-06-03T13:05:00+08:00""#));
assert!(prompt.contains("must not emit UTC, Z, or +00:00"));
assert!(!prompt.contains(r#""windowStart":"2026-06-03T05:00:00Z""#));
```

In `minimax_daily_brief_request_defaults_to_ten_thousand_completion_tokens`, add:

```rust
assert!(prompt.contains("Asia/Shanghai UTC+8"));
assert!(prompt.contains("localPeriodStart=2026-06-03T08:00:00+08:00"));
assert!(prompt.contains("localPeriodEnd=2026-06-04T08:00:00+08:00"));
assert!(prompt.contains(r#""localFirstActivityAt":"2026-06-03T13:00:00+08:00""#));
assert!(prompt.contains(r#""localStartAt":"2026-06-03T17:00:00+08:00""#));
assert!(prompt.contains(r#""localPeriodStart":"2026-06-03T13:00:00+08:00""#));
assert!(prompt.contains("must not emit UTC, Z, or +00:00"));
assert!(!prompt.contains(r#""periodStart":"2026-06-03T05:00:00Z""#));
```

In `local_daily_brief_builds_neutral_action_trajectory_from_five_hour_reports`, add:

```rust
assert!(brief.action_trajectory.contains("13:00-18:00"));
assert!(brief.action_trajectory.contains("18:00-23:00"));
assert!(!brief.action_trajectory.contains("05:00 - 10:00"));
assert!(!brief.action_trajectory.contains("10:00 - 15:00"));
```

- [ ] **Step 5: Run insight tests to verify they fail**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test insight_tests minimax_insight_report_uses_text_chat_completions minimax_insight_report_request_uses_window_summaries minimax_daily_brief_request_defaults_to_ten_thousand_completion_tokens local_daily_brief_builds_neutral_action_trajectory_from_five_hour_reports
```

Expected: FAIL because prompts still use old `periodStart` names and local fallback still formats `DateTime<Utc>` with the UTC clock.

- [ ] **Step 6: Update insight prompts and local fallback formatting**

In `collector/src/insights.rs`, change the import:

```rust
use crate::prompt_time::{
    human_report_range, human_report_timezone_label, minimax_prompt_timestamp,
};
```

In `LocalDailyBriefReporter::report`, replace the report range formatting with:

```rust
format!(
    "{}：{}",
    human_report_range(report.period_start, report.period_end),
    report.summary_text
)
```

In `report_prompt`, rename the human time labels in the prompt and add the no-UTC instruction:

```rust
"Infer the user's work trajectory for this 5-hour window. All time fields are {} local time. Return JSON only with keys summaryText and projectHints. summaryText must be complete, structured, human-readable Chinese and must not emit UTC, Z, or +00:00 time labels. Focus on projects, time allocation, workflow pattern, switching or loafing signs, and uncertainty; do not list every 5-minute window. localPeriodStart={}, localPeriodEnd={}, observations={}"
```

Pass `human_report_timezone_label()` before the local period timestamps.

In `window_summary_report_prompt`, change summary JSON keys:

```rust
serde_json::json!({
    "localWindowStart": minimax_prompt_timestamp(summary.window_start),
    "localWindowEnd": minimax_prompt_timestamp(summary.window_end),
    "summaryText": summary.summary_text,
    "continuity": summary.continuity,
    "primaryActivity": summary.primary_activity.as_str(),
    "projectHints": summary.project_hints,
    "taskIntent": summary.task_intent,
    "trajectory": summary.trajectory,
    "switchingLevel": summary.switching_level,
    "switchingEvidence": summary.switching_evidence,
    "loafingLevel": summary.loafing_level,
    "loafingEvidence": summary.loafing_evidence,
    "visibleApps": summary.visible_apps,
    "visibleTextHints": summary.visible_text_hints,
    "riskFlags": summary.risk_flags,
    "confidence": summary.confidence
})
```

Update that prompt string to use `localPeriodStart`, `localPeriodEnd`, `human_report_timezone_label()`, and the same `must not emit UTC, Z, or +00:00` instruction.

In `daily_activity_stats_prompt_value`, rename human-visible fields:

```rust
"localPeriodStart": minimax_prompt_timestamp(stats.period_start),
"localPeriodEnd": minimax_prompt_timestamp(stats.period_end),
"localFirstActivityAt": stats.first_activity_at.map(minimax_prompt_timestamp),
"localLastActivityAt": stats.last_activity_at.map(minimax_prompt_timestamp),
```

In `hourly_metrics_prompt_values`, rename:

```rust
"localStartAt": minimax_prompt_timestamp(metric.start_at),
"localEndAt": minimax_prompt_timestamp(metric.end_at),
```

In `daily_brief_prompt`, rename `ReportPromptRow` fields:

```rust
local_period_start: String,
local_period_end: String,
```

Set them with `minimax_prompt_timestamp(report.period_start)` and `minimax_prompt_timestamp(report.period_end)`. Update the prompt string to:

```rust
"Write a neutral daily brief in Chinese. All time fields are {} local time. Return JSON only with keys dailySummaryText, actionTrajectory, comparisonExplanation. Do not include advice, praise, criticism, ranking, or value judgment. Avoid words equivalent to productive, wasted, efficient, inefficient, good, bad, should. actionTrajectory must be complete, human-readable, structured around parallel projects, time allocation, workflow pattern, work mode, design/tooling activity, and important evidence, and must not emit UTC, Z, or +00:00 time labels; do not create a raw 5-minute log. Use uncertainty when evidence is incomplete. date={}, localPeriodStart={}, localPeriodEnd={}, descriptiveStats={}, hourlyMetrics={}, comparison={}, fiveHourReports={}"
```

Pass `human_report_timezone_label()` and local timestamps in the new format-argument order.

- [ ] **Step 7: Write failing Notion archive markdown assertions**

In `collector/tests/api_tests.rs`, update `serves_notion_daily_archive_with_human_readable_markdown`:

```rust
let markdown = body["archiveMarkdown"].as_str().unwrap();
assert_eq!(body["source"]["timezone"], "Asia/Shanghai UTC+8");
assert!(markdown.contains("13:00-18:00"));
assert!(markdown.contains("18:00-23:00"));
assert!(markdown.contains("First activity: 2026-05-24T13:00:00+08:00"));
assert!(!markdown.contains("2026-05-24T05:00:00Z"));
assert!(!markdown.contains("2026-05-24T10:00:00Z"));
assert!(!markdown.contains("+00:00"));
```

- [ ] **Step 8: Run Notion archive test to verify it fails**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_notion_daily_archive_with_human_readable_markdown
```

Expected: FAIL because `optional_time`, `project_lines_from_reports`, and `report_lines` still emit UTC RFC3339 strings.

- [ ] **Step 9: Update Notion archive markdown formatting**

In `collector/src/api.rs`, import:

```rust
use crate::prompt_time::{human_report_range, human_report_timestamp};
```

Replace `optional_time` with:

```rust
fn optional_human_time(value: Option<DateTime<Utc>>) -> String {
    value
        .map(human_report_timestamp)
        .unwrap_or_else(|| "unknown".into())
}
```

Update callers:

```rust
optional_human_time(stats.first_activity_at)
optional_human_time(stats.last_activity_at)
```

In `project_lines_from_reports`, replace the range formatting:

```rust
format!(
    "- {}: {} ({})",
    human_report_range(report.period_start, report.period_end),
    projects,
    category_mix_text(&report.category_mix)
)
```

In `report_lines`, replace the range formatting:

```rust
format!(
    "- {} | evidence {} | projects {} | {}",
    human_report_range(report.period_start, report.period_end),
    report.evidence_count,
    projects,
    report.summary_text
)
```

In the `Hourly Reports` markdown section added in Task 5, replace separate `with_timezone(&Local).format("%H:%M")` calls with the same shared range helper:

```rust
lines.push(format!(
    "- {}: {}",
    human_report_range(report.period_start, report.period_end),
    report.summary_text
));
```

In `NotionArchiveSource`, set the human timezone label explicitly:

```rust
timezone: "Asia/Shanghai UTC+8".into(),
```

- [ ] **Step 10: Run timezone-focused tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector prompt_time_tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test insight_tests minimax_insight_report_uses_text_chat_completions minimax_insight_report_request_uses_window_summaries minimax_daily_brief_request_defaults_to_ten_thousand_completion_tokens local_daily_brief_builds_neutral_action_trajectory_from_five_hour_reports
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests serves_notion_daily_archive_with_human_readable_markdown
```

Expected: PASS.

- [ ] **Step 11: Commit**

```powershell
git add collector/src/prompt_time.rs collector/src/insights.rs collector/src/api.rs collector/tests/insight_tests.rs collector/tests/api_tests.rs
git commit -m "fix: use local time in human reports"
```

---

### Task 7: Update Frontend Types And API Parsing

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/dailyBrief.ts`
- Modify: `src/lib/dailyBrief.test.ts`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Write failing frontend parser test**

In `src/lib/dailyBrief.test.ts`, update the existing parser test:

```ts
expect(response.hourlyReports[0].reportKind).toBe("1h");
expect(response.hourlyReports[0].summaryText).toBe("09点小时报告。");
expect(response.fiveHourReports[0].reportKind).toBe("5h");
```

Add this helper:

```ts
function hourlyReport() {
  return {
    id: 9,
    periodStart: "2026-06-03T09:00:00Z",
    periodEnd: "2026-06-03T10:00:00Z",
    generatedAt: "2026-06-03T10:00:05Z",
    reportKind: "1h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText: "09点小时报告。",
    categoryMix: [{ activityCategory: "coding", count: 12 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 12,
    error: null,
  };
}
```

Update `dailyBriefResponse()` in the same file:

```ts
hourlyReports: [hourlyReport()],
fiveHourReports: [insightReport()],
```

- [ ] **Step 2: Run parser test to verify it fails**

Run:

```powershell
npm test -- --run src/lib/dailyBrief.test.ts
```

Expected: FAIL because `DailyBriefResponse` has no `hourlyReports` field.

- [ ] **Step 3: Update TypeScript types and parser**

In `src/types.ts`, update:

```ts
export type DailyBriefResponse = {
  date: string;
  status: "missing" | "pending" | "running" | "complete" | "error" | string;
  nextRunAt?: string;
  brief?: DailyBrief;
  hourlyReports: InsightReport[];
  fiveHourReports: InsightReport[];
  descriptiveStats: DailyActivityStats;
  hourlyMetrics: HourlyActivityMetric[];
  comparison: DailyComparison;
};
```

In `src/lib/dailyBrief.ts`, update `toDailyBriefResponse`:

```ts
return {
  date: readString(value, "date"),
  status: readString(value, "status"),
  nextRunAt: readOptionalString(value, "nextRunAt"),
  brief: readOptionalRecord(value, "brief", toDailyBrief),
  hourlyReports: readArray(value, "hourlyReports", toInsightReport),
  fiveHourReports: readArray(value, "fiveHourReports", toInsightReport),
  descriptiveStats: readDailyActivityStats(value, "descriptiveStats"),
  hourlyMetrics: readArray(value, "hourlyMetrics", toHourlyActivityMetric),
  comparison: readDailyComparison(value, "comparison"),
};
```

In `src/App.test.tsx`, add `hourlyReports: [...]` to every `dailyBriefResponse()` fixture object. Use this row:

```ts
hourlyReports: [
  {
    id: 9,
    periodStart: "2026-05-24T09:00:00Z",
    periodEnd: "2026-05-24T10:00:00Z",
    generatedAt: "2026-05-24T10:00:05Z",
    reportKind: "1h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText: "09点小时报告。",
    categoryMix: [{ activityCategory: "coding", count: 12 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 12,
    error: null
  }
],
```

- [ ] **Step 4: Run frontend parser and App tests**

Run:

```powershell
npm test -- --run src/lib/dailyBrief.test.ts src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src/types.ts src/lib/dailyBrief.ts src/lib/dailyBrief.test.ts src/App.test.tsx
git commit -m "feat: parse hourly daily brief reports"
```

---

### Task 8: Build Structured Report Presentation Model

**Files:**
- Create: `src/lib/reportPresentation.ts`
- Create: `src/lib/reportPresentation.test.ts`

- [ ] **Step 1: Write failing presentation tests**

Create `src/lib/reportPresentation.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { DailyBrief, InsightReport } from "../types";
import {
  normalizeVisibleTimeText,
  presentDailyNarrative,
  presentInsightReport,
  splitReportText,
  truncateText,
} from "./reportPresentation";

function report(overrides: Partial<InsightReport> = {}): InsightReport {
  return {
    id: 9,
    periodStart: "2026-06-07T02:00:00Z",
    periodEnd: "2026-06-07T03:00:00Z",
    generatedAt: "2026-06-07T03:00:05Z",
    reportKind: "1h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText:
      "1) Notion RAW 字段整理，补全 raw_link 与 note_product_link。2) Codex 前端计划，处理 hourlyReports 与 5h Reports。3) ERROR 待排查：长文本直接渲染造成巨大段落。",
    categoryMix: [
      { activityCategory: "coding", count: 8 },
      { activityCategory: "research", count: 4 },
    ],
    projectHints: ["Time State Recorder", "Notion OS"],
    evidenceCount: 12,
    error: null,
    ...overrides,
  };
}

function brief(overrides: Partial<DailyBrief> = {}): DailyBrief {
  return {
    id: 1,
    date: "2026-06-07",
    generatedAt: "2026-06-07T23:50:00Z",
    modelProvider: "local_insight",
    modelName: "daily-brief-v1",
    dailySummaryText:
      "上午：推进报告 cadence。下午：整理前端结构化呈现。晚上：验证 API 与 UI。",
    actionTrajectory:
      "09:00-10:00 处理 hourly 报告。15:00-20:00 处理 scheduled 5h 报告。待排查：移动端溢出。",
    keyTransitions: [],
    suggestedNextActions: [],
    fiveHourReportIds: [2],
    error: null,
    ...overrides,
  };
}

describe("reportPresentation", () => {
  it("splits numbered long report text into bounded visible sections", () => {
    const presentation = presentInsightReport(report());

    expect(presentation.title).toBe("1h Report");
    expect(presentation.timeRange).toBe("10:00 - 11:00");
    expect(presentation.overview.length).toBeLessThanOrEqual(180);
    expect(presentation.phases.length).toBeGreaterThanOrEqual(3);
    expect(presentation.phases[0].body).toContain("Notion RAW");
    expect(presentation.chips).toContain("Time State Recorder");
    expect(presentation.chips).toContain("12 windows");
    expect(presentation.uncertainty.join(" ")).toContain("ERROR");
    expect(presentation.rawText).toContain("长文本直接渲染");
  });

  it("extracts sections from Chinese punctuation and time spans", () => {
    const sections = splitReportText(
      "10:00-15:00 文献与 Notion RAW；15:00-20:00 Codex 前端设计；20:00-01:00 验证与 PR 准备。"
    );

    expect(sections).toHaveLength(3);
    expect(sections[1]).toContain("Codex 前端设计");
  });

  it("structures daily narrative without losing raw text", () => {
    const presentation = presentDailyNarrative(brief());

    expect(presentation.title).toBe("Daily Action Trajectory");
    expect(presentation.phases.length).toBeGreaterThanOrEqual(3);
    expect(presentation.uncertainty.join(" ")).toContain("待排查");
    expect(presentation.rawText).toContain("09:00-10:00");
  });

  it("normalizes visible legacy UTC timestamp tokens but preserves raw text", () => {
    const presentation = presentInsightReport(
      report({
        periodStart: "2026-06-07T02:00:00Z",
        periodEnd: "2026-06-07T07:00:00Z",
        summaryText:
          "2026-06-07T02:00:00Z 到 2026-06-07T07:00:00Z 处理 Time State Recorder 报告。",
      })
    );

    expect(normalizeVisibleTimeText("2026-06-07T02:00:00Z")).toBe("10:00");
    expect(presentation.overview).toContain("10:00");
    expect(presentation.overview).toContain("15:00");
    expect(presentation.overview).not.toContain("02:00:00Z");
    expect(presentation.rawText).toContain("2026-06-07T02:00:00Z");
  });

  it("truncates visible text deterministically", () => {
    expect(truncateText("a".repeat(400), 20)).toHaveLength(20);
  });
});
```

- [ ] **Step 2: Run presentation tests to verify they fail**

Run:

```powershell
npm test -- --run src/lib/reportPresentation.test.ts
```

Expected: FAIL because `src/lib/reportPresentation.ts` does not exist.

- [ ] **Step 3: Implement presentation model**

Create `src/lib/reportPresentation.ts`:

```ts
import type { DailyBrief, InsightReport } from "../types";

export type ReportPhase = {
  label: string;
  body: string;
  meta?: string;
};

export type StructuredReportPresentation = {
  title: string;
  timeRange: string;
  eyebrow: string;
  overview: string;
  phases: ReportPhase[];
  chips: string[];
  evidence: string[];
  uncertainty: string[];
  rawText: string;
};

const MAX_OVERVIEW_LENGTH = 180;
const MAX_PHASE_BODY_LENGTH = 220;
const MAX_CHIPS = 8;
const OWNER_TIME_ZONE = "Asia/Shanghai";
const RISK_PATTERN = /(不确定|可能|待|缺失|卡住|ERROR|风险|无法)/i;
const NUMBERED_SPLIT_PATTERN = /(?=(?:[①②③④⑤⑥⑦⑧⑨]|\d+[).、]))/g;
const TIME_SPAN_SPLIT_PATTERN = /(?=\d{1,2}:\d{2}\s*-\s*\d{1,2}:\d{2})/g;
const UTC_TIMESTAMP_PATTERN = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g;

export function presentInsightReport(report: InsightReport): StructuredReportPresentation {
  const visibleText = normalizeVisibleTimeText(report.summaryText);
  const sections = splitReportText(visibleText);
  const phases = sections.slice(0, 6).map((section, index) => ({
    label: phaseLabel(section, index),
    body: truncateText(stripLeadingMarker(section), MAX_PHASE_BODY_LENGTH),
    meta: index === 0 ? `${report.evidenceCount} source windows` : undefined,
  }));
  return {
    title: report.reportKind === "5h" ? "5h Report" : "1h Report",
    timeRange: formatReportRange(report.periodStart, report.periodEnd),
    eyebrow: `${report.reportKind} - ${report.modelProvider}`,
    overview: truncateText(stripLeadingMarker(sections[0] ?? visibleText), MAX_OVERVIEW_LENGTH),
    phases: phases.length > 0 ? phases : fallbackPhase(visibleText),
    chips: reportChips(report),
    evidence: [`${report.evidenceCount} windows`, ...report.categoryMix.map((item) => `${item.activityCategory} ${item.count}`)],
    uncertainty: extractUncertainty(sections),
    rawText: report.summaryText,
  };
}

export function presentDailyNarrative(brief: DailyBrief): StructuredReportPresentation {
  const rawText = [brief.dailySummaryText, brief.actionTrajectory].filter(Boolean).join("\n");
  const visibleText = normalizeVisibleTimeText(rawText);
  const sections = splitReportText(visibleText);
  return {
    title: "Daily Action Trajectory",
    timeRange: brief.date,
    eyebrow: `daily - ${brief.modelProvider}`,
    overview: truncateText(stripLeadingMarker(sections[0] ?? visibleText), MAX_OVERVIEW_LENGTH),
    phases: sections.slice(0, 8).map((section, index) => ({
      label: phaseLabel(section, index),
      body: truncateText(stripLeadingMarker(section), MAX_PHASE_BODY_LENGTH),
    })),
    chips: [`${brief.fiveHourReportIds.length} 5h reports`, ...brief.suggestedNextActions.slice(0, 3)],
    evidence: brief.keyTransitions.slice(0, 4),
    uncertainty: extractUncertainty(sections),
    rawText,
  };
}

export function splitReportText(text: string): string[] {
  return text
    .split(NUMBERED_SPLIT_PATTERN)
    .flatMap((part) => part.split(TIME_SPAN_SPLIT_PATTERN))
    .flatMap((part) => part.split(/[；;。]\s*/))
    .map((part) => part.trim())
    .filter(Boolean);
}

export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) {
    return text;
  }
  if (maxLength <= 3) {
    return text.slice(0, maxLength);
  }
  return `${text.slice(0, maxLength - 3).trimEnd()}...`;
}

export function normalizeVisibleTimeText(text: string): string {
  return text.replace(UTC_TIMESTAMP_PATTERN, (token) => formatClock(new Date(token)));
}

export function formatReportRange(startIso: string, endIso: string): string {
  const start = new Date(startIso);
  const end = new Date(endIso);
  return `${formatClock(start)} - ${formatClock(end)}`;
}

function formatClock(value: Date): string {
  return new Intl.DateTimeFormat("zh-CN", {
    timeZone: OWNER_TIME_ZONE,
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(value);
}

function reportChips(report: InsightReport): string[] {
  return [...report.projectHints, `${report.evidenceCount} windows`, ...report.categoryMix.map((item) => item.activityCategory)]
    .filter(Boolean)
    .slice(0, MAX_CHIPS);
}

function extractUncertainty(sections: string[]): string[] {
  return sections.filter((section) => RISK_PATTERN.test(section)).slice(0, 3).map((section) => truncateText(stripLeadingMarker(section), 120));
}

function phaseLabel(section: string, index: number): string {
  const timeMatch = section.match(/\d{1,2}:\d{2}\s*-\s*\d{1,2}:\d{2}/);
  if (timeMatch) {
    return timeMatch[0].replace(/\s+/g, "");
  }
  return `Phase ${index + 1}`;
}

function stripLeadingMarker(text: string): string {
  return text.replace(/^(?:[①②③④⑤⑥⑦⑧⑨]|\d+[).、])\s*/, "").trim();
}

function fallbackPhase(text: string): ReportPhase[] {
  return [{ label: "Summary", body: truncateText(text, MAX_PHASE_BODY_LENGTH) }];
}
```

- [ ] **Step 4: Run presentation tests**

Run:

```powershell
npm test -- --run src/lib/reportPresentation.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src/lib/reportPresentation.ts src/lib/reportPresentation.test.ts
git commit -m "feat: structure report presentation model"
```

---

### Task 9: Render Structured Report Cards

**Files:**
- Create: `src/StructuredReportCard.tsx`
- Create: `src/StructuredDailyNarrative.tsx`
- Modify: `src/DailyBriefPanel.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Write failing UI assertions**

Add assertions in the App test that opens or observes the Daily Brief panel:

```ts
fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));

expect(screen.getByRole("heading", { name: /hourly reports/i })).toBeInTheDocument();
expect(screen.getByRole("heading", { name: /scheduled 5h reports/i })).toBeInTheDocument();
expect(screen.getByRole("heading", { name: /daily action trajectory/i })).toBeInTheDocument();
expect(screen.getByText("Phase 1")).toBeInTheDocument();
expect(screen.getByText("Raw report text")).toBeInTheDocument();
```

Add a regression assertion that protects against the screenshot failure mode:

```ts
const reportParagraphs = screen.getAllByTestId("structured-report-lead");
for (const paragraph of reportParagraphs) {
  expect(paragraph.textContent?.length ?? 0).toBeLessThanOrEqual(190);
}
expect(screen.queryByText(/本5小时窗口.*Time State Recorder.*Dayflow/)).not.toBeInTheDocument();
```

Use fixture text with at least six numbered Chinese clauses that mention `Time State Recorder`, `Notion`, `Codex`, `Dayflow`, `ERROR`, and a `20:00-01:00` slot so this assertion is meaningful.

- [ ] **Step 2: Run UI test to verify it fails**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: FAIL because structured report components are not wired into the panel.

- [ ] **Step 3: Create `StructuredReportCard`**

Create `src/StructuredReportCard.tsx`:

```tsx
import type { StructuredReportPresentation } from "./lib/reportPresentation";

type StructuredReportCardProps = {
  presentation: StructuredReportPresentation;
  canShowText: boolean;
};

export function StructuredReportCard({ presentation, canShowText }: StructuredReportCardProps) {
  return (
    <article className="structuredReportCard">
      <header className="structuredReportHeader">
        <div>
          <p className="eyebrow">{presentation.eyebrow}</p>
          <h4>{presentation.title}</h4>
        </div>
        <span className="statusPill">{presentation.timeRange}</span>
      </header>
      <div className="reportChipRow">
        {presentation.chips.map((chip) => (
          <span key={chip}>{chip}</span>
        ))}
      </div>
      {canShowText ? (
        <>
          <p className="structuredReportLead" data-testid="structured-report-lead">
            {presentation.overview}
          </p>
          <ol className="reportPhaseList">
            {presentation.phases.map((phase) => (
              <li key={`${phase.label}-${phase.body}`}>
                <strong>{phase.label}</strong>
                <p>{phase.body}</p>
                {phase.meta ? <span>{phase.meta}</span> : null}
              </li>
            ))}
          </ol>
          {presentation.uncertainty.length > 0 ? (
            <div className="reportUncertainty">
              {presentation.uncertainty.map((item) => (
                <span key={item}>{item}</span>
              ))}
            </div>
          ) : null}
          <details className="rawReportDetails">
            <summary>Raw report text</summary>
            <p>{presentation.rawText}</p>
          </details>
        </>
      ) : (
        <p className="redactedText insightRedacted">Report narrative hidden in redacted mode.</p>
      )}
    </article>
  );
}
```

- [ ] **Step 4: Create `StructuredDailyNarrative`**

Create `src/StructuredDailyNarrative.tsx`:

```tsx
import type { DailyBrief } from "./types";
import { presentDailyNarrative } from "./lib/reportPresentation";
import { StructuredReportCard } from "./StructuredReportCard";

type StructuredDailyNarrativeProps = {
  brief: DailyBrief;
  canShowText: boolean;
};

export function StructuredDailyNarrative({ brief, canShowText }: StructuredDailyNarrativeProps) {
  return (
    <StructuredReportCard
      presentation={presentDailyNarrative(brief)}
      canShowText={canShowText}
    />
  );
}
```

- [ ] **Step 5: Wire structured cards into `DailyBriefPanel`**

In `src/DailyBriefPanel.tsx`, import:

```tsx
import { StructuredDailyNarrative } from "./StructuredDailyNarrative";
import { StructuredReportCard } from "./StructuredReportCard";
import { presentInsightReport } from "./lib/reportPresentation";
```

Split report arrays:

```tsx
const hourlyReports = response?.hourlyReports ?? [];
const fiveHourReports = response?.fiveHourReports ?? [];
```

Update the metric:

```tsx
<Metric
  icon={<Layers size={17} />}
  label="reports"
  value={`${hourlyReports.length} hourly · ${fiveHourReports.length} 5h`}
/>
```

Replace raw daily summary and action trajectory paragraphs with:

```tsx
{response.brief ? (
  <StructuredDailyNarrative brief={response.brief} canShowText={canShowText} />
) : null}
```

Add the hourly section before the 5-hour section:

```tsx
<div className="dailyBriefSection">
  <h3>Hourly Reports</h3>
  {hourlyReports.length > 0 ? (
    <div className="dailyReportList">
      {hourlyReports.map((report) => (
        <StructuredReportCard
          key={report.id}
          presentation={presentInsightReport(report)}
          canShowText={canShowText}
        />
      ))}
    </div>
  ) : (
    <p className="emptyState">No hourly reports for this date yet.</p>
  )}
</div>
```

Rename the existing 5h section to `Scheduled 5h Reports` and render `StructuredReportCard` for each `fiveHourReports` item.

- [ ] **Step 6: Add bounded card styles**

In `src/styles.css`, add classes that keep long model text readable:

```css
.structuredReportCard {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  background: var(--surface);
  display: grid;
  gap: 12px;
  min-width: 0;
}

.structuredReportHeader {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}

.reportChipRow,
.reportUncertainty {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.reportChipRow span,
.reportUncertainty span {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 3px 8px;
  font-size: 12px;
  color: var(--muted);
  max-width: 100%;
  overflow-wrap: anywhere;
}

.structuredReportLead,
.rawReportDetails p,
.reportPhaseList p {
  overflow-wrap: anywhere;
  line-height: 1.55;
}

.structuredReportLead {
  margin: 0;
  font-size: 14px;
}

.reportPhaseList {
  display: grid;
  gap: 10px;
  margin: 0;
  padding-left: 20px;
}

.reportPhaseList li {
  min-width: 0;
}

.reportPhaseList strong {
  display: block;
  font-size: 12px;
  text-transform: uppercase;
  color: var(--muted);
}

.rawReportDetails {
  border-top: 1px solid var(--border);
  padding-top: 10px;
}
```

If the repo uses different CSS custom properties, map these classes to the existing surface, border, and muted tokens already used by `DailyBriefPanel`.

- [ ] **Step 7: Run UI test**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 8: Commit**

```powershell
git add src/StructuredReportCard.tsx src/StructuredDailyNarrative.tsx src/DailyBriefPanel.tsx src/App.test.tsx src/styles.css
git commit -m "feat: render structured report cards"
```

---

### Task 10: Full Verification And PR-Ready Cleanup

**Files:**
- Verify all changed files.

- [ ] **Step 1: Run Rust collector tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector
```

Expected: all collector tests pass.

- [ ] **Step 2: Run frontend tests**

Run:

```powershell
npm test -- --run
```

Expected: all Vitest tests pass.

- [ ] **Step 3: Run production build**

Run:

```powershell
npm run build
```

Expected: TypeScript and Vite build pass.

- [ ] **Step 4: Run whitespace check**

Run:

```powershell
git diff --check
```

Expected: no output.

- [ ] **Step 5: Browser smoke**

Start the dev server in this worktree if it is not running:

```powershell
npm run dev -- --host 127.0.0.1 --port 5179
```

Verify:

- Daily Brief shows `Hourly Reports`.
- Daily Brief shows `Scheduled 5h Reports`.
- Daily Brief shows `Daily Action Trajectory` as structured sections.
- Hourly and 5h reports render as bounded cards with chips, phases, risk notes, and collapsed raw text.
- Human-visible report ranges use Asia/Shanghai UTC+8 local clock labels such as `10:00-15:00`, never UTC labels.
- Existing report text containing legacy `2026-...Z` timestamps is normalized in visible frontend sections while the collapsed raw text preserves the original database text.
- No report text renders as one unbounded wall-of-text paragraph.
- Redacted mode hides hourly, 5h, and daily narrative text while preserving labels, time ranges, and non-sensitive counts.
- Raw mode reveals structured hourly, 5h, and daily narrative text.
- Mobile viewport has no horizontal page overflow.

- [ ] **Step 6: Final semantic commit if any cleanup remains**

If verification changes source or test files, commit the concrete files changed by verification with:

```powershell
git add collector/src/api.rs collector/src/insights.rs collector/src/prompt_time.rs collector/src/storage.rs collector/tests/api_tests.rs collector/tests/insight_tests.rs collector/tests/storage_tests.rs collector/tests/notion_daily_archive_smoke_tests.rs src/types.ts src/lib/dailyBrief.ts src/lib/dailyBrief.test.ts src/lib/reportPresentation.ts src/lib/reportPresentation.test.ts src/StructuredReportCard.tsx src/StructuredDailyNarrative.tsx src/DailyBriefPanel.tsx src/App.test.tsx src/styles.css docs/api/notion-daily-archive.md
git commit -m "test: verify report cadence UI"
```

- [ ] **Step 7: Push branch and open PR**

```powershell
git push -u origin codex/report-cadence-20260607
gh pr create --draft --base master --head codex/report-cadence-20260607 --title "[codex] add hourly and fixed five-hour reports" --body-file pr-body.md
```

Use a PR body with:

```markdown
## Summary

- Adds top-of-hour `1h` insight reports from 5-minute visual window summaries.
- Keeps `5h` reports and switches them to fixed local slots from 2026-06-07: 10:00-15:00, 15:00-20:00, 20:00-01:00.
- Keeps UTC as storage/API machine data only; all report prompts, fallback text, Notion markdown, and frontend cards display Asia/Shanghai UTC+8 local time.
- Adds `hourlyReports` to Daily Brief / Notion archive responses.
- Adds a structured frontend presentation framework for hourly, scheduled 5h, and daily narrative reports so raw model text is progressive-disclosure content, not the primary layout.

## Validation

- `cargo test -p tsr-collector`
- `cargo test -p tsr-collector prompt_time_tests`
- `npm test -- --run`
- `npm run build`
- `git diff --check`
- Browser smoke on Daily Brief report sections
```

---

## Self-Review

- Spec coverage: The plan covers hourly reports, fixed five-hour slots from 2026-06-07, retention of existing five-hour reports, 5-minute visual-window source data, backend scheduling, API response updates, structured frontend rendering, tests, docs, and PR flow.
- Presentation coverage: The plan explicitly keeps coarse merged report text in storage while requiring the frontend to derive bounded sections, chips, evidence, risks, and collapsed raw text for 1h, 5h, and daily reports.
- Time semantics coverage: The plan treats UTC as storage-only and requires UTC+8 conversion in MiniMax prompts, local fallback Daily Brief text, Notion markdown, and frontend visible sections.
- Red-flag scan: No task uses unspecified future work. Each implementation task names files, includes concrete code shapes, and lists exact commands.
- Type consistency: Backend uses `InsightReport.report_kind = "1h" | "5h"`; frontend uses `InsightReport.reportKind` and `DailyBriefResponse.hourlyReports | fiveHourReports`.
- Risk note: Existing dirty changes in the main checkout are not part of this work. All implementation should stay inside `.worktrees/report-cadence-20260607`.
