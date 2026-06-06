# Hourly And Fixed Five-Hour Reports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add top-of-hour 1-hour reports, keep 5-hour reports, and switch 5-hour generation from rolling windows to fixed local windows starting on 2026-06-07: 10:00-15:00, 15:00-20:00, and 20:00-01:00.

**Architecture:** Reuse the existing `insight_reports` table and `InsightReport` model. Add `report_kind = "1h"` for hourly reports and keep `report_kind = "5h"` for five-hour reports. Both kinds are generated from `visual_window_summaries`; Daily Brief and Notion archive responses keep `fiveHourReports` and add `hourlyReports` so existing consumers are not silently repointed.

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
- `InsightReport.reportKind` values are exactly `"1h"` and `"5h"`.
- The API field `fiveHourReports` remains unchanged. Add `hourlyReports` beside it.
- `DailyBrief.fiveHourReportIds`, `DailyActivityStats.fiveHourReportCount`, and `HourlyActivityMetric.fiveHourReportIds` stay as-is for backward compatibility. The new hourly reports are displayed in the response/UI but are not stored inside `DailyBrief`.
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
- Modify `collector/src/storage.rs`
  - Add `insight_report_exists(kind, start, end)`.
  - Keep the existing `insight_reports` schema; do not add a table or migration.
- Modify `collector/tests/insight_tests.rs`
  - Add hourly report builder tests.
  - Update MiniMax prompt tests for report-kind-aware prompts.
- Modify `collector/tests/storage_tests.rs`
  - Add exact-kind/exact-period report-existence tests.
- Modify `collector/tests/api_tests.rs`
  - Add API coverage for `kind=1h`.
  - Add `hourlyReports` coverage in `/api/daily-brief` and `/api/notion/daily-archive`.
- Modify `src/types.ts`
  - Add `hourlyReports: InsightReport[]` to `DailyBriefResponse`.
- Modify `src/lib/dailyBrief.ts`
  - Parse `hourlyReports`.
- Modify `src/lib/dailyBrief.test.ts`
  - Verify hourly reports parse and existing 5h reports still parse.
- Modify `src/DailyBriefPanel.tsx`
  - Show an "Hourly Reports" section above scheduled 5h reports.
  - Rename the 5h heading to "Scheduled 5h Reports".
- Modify `src/App.test.tsx`
  - Update daily brief fixtures with `hourlyReports`.
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

### Task 6: Update Frontend Types And API Parsing

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

### Task 7: Render Hourly Reports In Daily Brief UI

**Files:**
- Modify: `src/DailyBriefPanel.tsx`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Write failing UI assertion**

Add an assertion in the App test that opens or observes the Daily Brief panel:

```ts
expect(screen.getByRole("heading", { name: /hourly reports/i })).toBeInTheDocument();
expect(screen.getByText("09点小时报告。")).toBeInTheDocument();
expect(screen.getByRole("heading", { name: /scheduled 5h reports/i })).toBeInTheDocument();
```

If the test is in redacted mode, switch to Raw first:

```ts
fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));
```

- [ ] **Step 2: Run UI test to verify it fails**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: FAIL because the panel does not render hourly reports.

- [ ] **Step 3: Update `DailyBriefPanel`**

In `src/DailyBriefPanel.tsx`, split report arrays:

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

Add this section before the five-hour section:

```tsx
<div className="dailyBriefSection">
  <h3>Hourly Reports</h3>
  {hourlyReports.length > 0 ? (
    <div className="dailyReportList">
      {hourlyReports.map((report) => (
        <ReportRow key={report.id} report={report} canShowText={canShowText} />
      ))}
    </div>
  ) : (
    <p className="emptyState">No hourly reports for this date yet.</p>
  )}
</div>
```

Rename the existing 5h section:

```tsx
<h3>Scheduled 5h Reports</h3>
```

Change its array references to `fiveHourReports`.

- [ ] **Step 4: Run UI test**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src/DailyBriefPanel.tsx src/App.test.tsx
git commit -m "feat: show hourly reports in daily brief"
```

---

### Task 8: Full Verification And PR-Ready Cleanup

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
- Redacted mode hides hourly and 5h summary text.
- Raw mode reveals hourly and 5h summary text.
- Mobile viewport has no horizontal page overflow.

- [ ] **Step 6: Final semantic commit if any cleanup remains**

If verification changes source or test files, commit the concrete files changed by verification with:

```powershell
git add collector/src/api.rs collector/src/insights.rs collector/src/storage.rs collector/tests/api_tests.rs collector/tests/insight_tests.rs collector/tests/storage_tests.rs collector/tests/notion_daily_archive_smoke_tests.rs src/types.ts src/lib/dailyBrief.ts src/lib/dailyBrief.test.ts src/DailyBriefPanel.tsx src/App.test.tsx docs/api/notion-daily-archive.md
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
- Adds `hourlyReports` to Daily Brief / Notion archive responses and displays them in the frontend.

## Validation

- `cargo test -p tsr-collector`
- `npm test -- --run`
- `npm run build`
- `git diff --check`
- Browser smoke on Daily Brief report sections
```

---

## Self-Review

- Spec coverage: The plan covers hourly reports, fixed five-hour slots from 2026-06-07, retention of existing five-hour reports, 5-minute visual-window source data, backend scheduling, API response updates, frontend rendering, tests, docs, and PR flow.
- Red-flag scan: No task uses unspecified future work. Each implementation task names files, includes concrete code shapes, and lists exact commands.
- Type consistency: Backend uses `InsightReport.report_kind = "1h" | "5h"`; frontend uses `InsightReport.reportKind` and `DailyBriefResponse.hourlyReports | fiveHourReports`.
- Risk note: Existing dirty changes in the main checkout are not part of this work. All implementation should stay inside `.worktrees/report-cadence-20260607`.
