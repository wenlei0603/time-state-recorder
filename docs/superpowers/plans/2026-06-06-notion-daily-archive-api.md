# Notion Daily Archive API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a stable Time State Recorder GET API that Notion Principles OS can call to archive a selected day's 5-hour reports, daily report, and descriptive statistics into the matching Daily Diary.

**Architecture:** Keep the existing `/api/daily-brief` response as the frontend contract. Add a Notion-specific read model at `/api/notion/daily-archive` that reuses the existing daily brief builder but adds Notion-facing metadata and a human-readable `archiveMarkdown` section. Notion Principles OS will consume this endpoint and write an idempotent visible diary section for `INDEX-YYYYMMDD | Daily Diary`.

**Tech Stack:** Rust collector with Axum/Serde/Chrono, existing SQLite `Store` aggregation methods, PowerShell Notion Principles OS helper through `npm run notion:os`, Rust integration tests with `reqwest`.

---

## Goal Contract

**Target state:** `Notion Principles OS` can GET one URL for a local date and receive everything needed for a diary archive: daily brief, all same-day 5-hour reports, descriptive stats, hourly metrics, comparison, and readable Markdown.

**Context:**
- TSR repo: `D:\CodexInfra\docs\projects\time-state-recorder`
- Notion OS repo/helper root: `D:\CodexInfra`
- Existing frontend/backend route: `/api/daily-brief?date=YYYY-MM-DD&tzOffsetMinutes=-480`
- Existing Notion write path: `npm run notion:os -- ensure-diary`, `Add-PrinciplesOSDiarySection`, `npm run notion:os -- verify-diary`

**Constraints:**
- Do not replace `/api/daily-brief`; keep frontend behavior stable.
- Do not require Notion OS to parse frontend UI state.
- Keep the API read-only and local-friendly; no Notion token belongs in TSR.
- Use local-date semantics through existing `DateQuery` and `tzOffsetMinutes`.
- The diary write must be idempotent by marker so repeated scheduled runs do not duplicate the same archive.

**Acceptance criteria:**
- `GET /api/notion/daily-archive?date=2026-06-05&tzOffsetMinutes=-480` returns HTTP 200.
- Response includes `date`, `source`, `archiveTitle`, `dailyDiaryTitle`, `archiveMarkdown`, `brief`, `fiveHourReports`, `descriptiveStats`, `hourlyMetrics`, and `comparison`.
- `archiveMarkdown` is human-readable and includes daily summary, parallel projects/time allocation, workflow pattern, five-hour report blocks, hourly metrics, comparison, and uncertainty notes.
- Rust API tests prove the endpoint returns same-day 5-hour reports and Markdown containing key diary sections.
- Notion Principles OS writes a visible 2026-06-05 diary section and readback verifies the marker.

---

### Task 1: Add Notion Archive API Response

**Files:**
- Modify: `collector/src/api.rs`
- Test: `collector/tests/api_tests.rs`

- [ ] **Step 1: Write the failing API test**

Add this test below `serves_daily_brief_response_with_stats_and_same_day_reports` in `collector/tests/api_tests.rs`:

```rust
#[tokio::test]
async fn serves_notion_daily_archive_with_human_readable_markdown() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let first_report_id = store
        .insert_insight_report(&sample_insight_report(
            0,
            "2026-05-24T05:00:00Z",
            "2026-05-24T10:00:00Z",
            "上午报告。",
        ))
        .unwrap();
    let second_report_id = store
        .insert_insight_report(&sample_insight_report(
            0,
            "2026-05-24T10:00:00Z",
            "2026-05-24T15:00:00Z",
            "下午报告。",
        ))
        .unwrap();
    let mut brief = sample_daily_brief("2026-05-24");
    brief.five_hour_report_ids = vec![first_report_id, second_report_id];
    brief.hourly_metrics[0].five_hour_report_ids = vec![first_report_id];
    store.insert_daily_brief(&brief).unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/notion/daily-archive?date=2026-05-24&tzOffsetMinutes=-480"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["date"], "2026-05-24");
    assert_eq!(body["dailyDiaryTitle"], "INDEX-20260524 | Daily Diary");
    assert_eq!(body["source"]["endpoint"], "/api/notion/daily-archive");
    assert_eq!(body["fiveHourReports"].as_array().unwrap().len(), 2);
    assert_eq!(body["descriptiveStats"]["activeSeconds"], 3600);
    let markdown = body["archiveMarkdown"].as_str().unwrap();
    assert!(markdown.contains("## Daily Summary"));
    assert!(markdown.contains("## Parallel Projects And Time Allocation"));
    assert!(markdown.contains("Time State Recorder"));
    assert!(markdown.contains("上午报告。"));
    assert!(markdown.contains("编码窗口较前一日增加。"));

    server.abort();
}
```

- [ ] **Step 2: Run the failing API test**

Run:

```powershell
& $env:USERPROFILE\.cargo\bin\cargo.exe test -p tsr-collector --test api_tests serves_notion_daily_archive_with_human_readable_markdown
```

Expected: FAIL because `/api/notion/daily-archive` does not exist.

- [ ] **Step 3: Add response structs and route**

In `collector/src/api.rs`, add near `DailyBriefResponse`:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotionDailyArchiveResponse {
    date: String,
    generated_at: DateTime<Utc>,
    archive_title: String,
    daily_diary_title: String,
    source: NotionArchiveSource,
    status: String,
    archive_markdown: String,
    brief: Option<DailyBrief>,
    five_hour_reports: Vec<InsightReport>,
    descriptive_stats: DailyActivityStats,
    hourly_metrics: Vec<HourlyActivityMetric>,
    comparison: DailyComparison,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotionArchiveSource {
    app: String,
    endpoint: String,
    local_date: String,
    timezone: String,
}
```

Add the route in `router_from_state` after `/api/daily-brief`:

```rust
.route("/api/notion/daily-archive", get(notion_daily_archive))
```

- [ ] **Step 4: Implement the archive builder**

Add below `build_daily_brief_response`:

```rust
async fn notion_daily_archive(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date_window = match date_window_from_query(&query) {
        Ok(date_window) => date_window,
        Err(message) => return bad_request(&message),
    };
    match build_notion_daily_archive_response(&state, date_window) {
        Ok(response) => Json(response).into_response(),
        Err(err) => internal_error(err),
    }
}

fn build_notion_daily_archive_response(
    state: &AppState,
    date_window: DateWindow,
) -> Result<NotionDailyArchiveResponse> {
    let response = build_daily_brief_response(state, date_window.clone())?;
    let daily_diary_title = daily_diary_title(&response.date);
    let archive_title = format!("Time State Recorder Daily Archive | {}", response.date);
    let archive_markdown = render_notion_archive_markdown(
        &response.date,
        &archive_title,
        &daily_diary_title,
        response.brief.as_ref(),
        &response.five_hour_reports,
        &response.descriptive_stats,
        &response.hourly_metrics,
        &response.comparison,
    );

    Ok(NotionDailyArchiveResponse {
        date: response.date.clone(),
        generated_at: Utc::now(),
        archive_title,
        daily_diary_title,
        source: NotionArchiveSource {
            app: "time-state-recorder".into(),
            endpoint: "/api/notion/daily-archive".into(),
            local_date: response.date.clone(),
            timezone: "query-local-date".into(),
        },
        status: response.status,
        archive_markdown,
        brief: response.brief,
        five_hour_reports: response.five_hour_reports,
        descriptive_stats: response.descriptive_stats,
        hourly_metrics: response.hourly_metrics,
        comparison: response.comparison,
    })
}
```

- [ ] **Step 5: Add Markdown helpers**

Add private helpers in `collector/src/api.rs`:

```rust
fn daily_diary_title(date: &str) -> String {
    format!("INDEX-{} | Daily Diary", date.replace('-', ""))
}

fn render_notion_archive_markdown(
    date: &str,
    archive_title: &str,
    daily_diary_title: &str,
    brief: Option<&DailyBrief>,
    reports: &[InsightReport],
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    comparison: &DailyComparison,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {archive_title}"));
    lines.push(format!("Daily Diary: {daily_diary_title}"));
    lines.push(format!("Source date: {date}"));
    lines.push(String::new());
    lines.push("## Daily Summary".into());
    if let Some(brief) = brief {
        lines.push(brief.daily_summary_text.clone());
        lines.push(format!("Action trajectory: {}", brief.action_trajectory));
    } else {
        lines.push("No generated daily brief was found; this archive uses descriptive stats and 5-hour reports only.".into());
    }
    lines.push(String::new());
    lines.push("## Descriptive Statistics".into());
    lines.push(format!("- Active time: {:.2} hours", stats.active_hours));
    lines.push(format!("- Window switches: {}", stats.switch_count));
    lines.push(format!("- Distinct apps: {}", stats.distinct_app_count));
    lines.push(format!("- Input: {} chars across {} events", stats.input_chars, stats.input_events));
    lines.push(format!("- Visual evidence: {} visual windows, {} high-res screenshots", stats.visual_window_count, stats.high_res_screenshot_count));
    lines.push(format!("- First activity: {}", optional_time(stats.first_activity_at)));
    lines.push(format!("- Last activity: {}", optional_time(stats.last_activity_at)));
    lines.push(format!("- Top apps: {}", top_apps_text(&stats.top_apps)));
    lines.push(String::new());
    lines.push("## Parallel Projects And Time Allocation".into());
    lines.extend(project_lines_from_reports(reports));
    lines.push(String::new());
    lines.push("## Workflow Pattern".into());
    lines.extend(hourly_lines(hourly_metrics));
    lines.push(String::new());
    lines.push("## Five-Hour Reports".into());
    lines.extend(report_lines(reports));
    lines.push(String::new());
    lines.push("## Comparison".into());
    lines.push(comparison.explanation.clone());
    lines.push(format!("- Active time delta: {} seconds", comparison.active_seconds_delta));
    lines.push(format!("- Switches/hour delta: {:.2}", comparison.switches_per_hour_delta));
    lines.push(format!("- Input chars delta: {}", comparison.input_chars_delta));
    lines.push(String::new());
    lines.push("## Uncertainty And Review Notes".into());
    lines.push("- This archive is generated from local desktop activity records, screenshots, model summaries, and daily aggregation stats.".into());
    lines.push("- It may miss off-screen work, offline activity, or model interpretation errors; keep diary-level review available.".into());
    lines.join("\n")
}
```

Implement the helpers used above with deterministic sorting/presentation:

```rust
fn optional_time(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|time| time.to_rfc3339())
        .unwrap_or_else(|| "unknown".into())
}

fn top_apps_text(apps: &[DailyAppActivity]) -> String {
    if apps.is_empty() {
        return "unknown".into();
    }
    apps.iter()
        .take(5)
        .map(|app| format!("{} {:.0}%", app.process_name, app.share * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn project_lines_from_reports(reports: &[InsightReport]) -> Vec<String> {
    if reports.is_empty() {
        return vec!["- No 5-hour reports found for this date.".into()];
    }
    let mut lines = Vec::new();
    for report in reports {
        let projects = if report.project_hints.is_empty() {
            "Unknown project".into()
        } else {
            report.project_hints.join(", ")
        };
        lines.push(format!(
            "- {} to {}: {} ({})",
            report.period_start.to_rfc3339(),
            report.period_end.to_rfc3339(),
            projects,
            category_mix_text(&report.category_mix)
        ));
    }
    lines
}

fn hourly_lines(metrics: &[HourlyActivityMetric]) -> Vec<String> {
    let active = metrics
        .iter()
        .filter(|metric| metric.active_seconds > 0)
        .collect::<Vec<_>>();
    if active.is_empty() {
        return vec!["- No active hourly metrics for this date.".into()];
    }
    active
        .into_iter()
        .map(|metric| {
            format!(
                "- {:02}:00: {:.1} active minutes, dominant app {}, category {}, reports {:?}",
                metric.hour,
                metric.active_seconds as f64 / 60.0,
                metric.dominant_app.as_deref().unwrap_or("unknown"),
                metric.dominant_category.as_str(),
                metric.five_hour_report_ids
            )
        })
        .collect()
}

fn report_lines(reports: &[InsightReport]) -> Vec<String> {
    if reports.is_empty() {
        return vec!["- No 5-hour reports available.".into()];
    }
    reports
        .iter()
        .map(|report| {
            format!(
                "- {} to {} | evidence {} | projects {} | {}",
                report.period_start.to_rfc3339(),
                report.period_end.to_rfc3339(),
                report.evidence_count,
                if report.project_hints.is_empty() { "unknown".into() } else { report.project_hints.join(", ") },
                report.summary_text
            )
        })
        .collect()
}

fn category_mix_text(mix: &[ActivityCategoryCount]) -> String {
    if mix.is_empty() {
        return "unknown category mix".into();
    }
    mix.iter()
        .map(|item| format!("{}:{}", item.activity_category.as_str(), item.count))
        .collect::<Vec<_>>()
        .join(", ")
}
```

- [ ] **Step 6: Run the API test**

Run:

```powershell
& $env:USERPROFILE\.cargo\bin\cargo.exe test -p tsr-collector --test api_tests serves_notion_daily_archive_with_human_readable_markdown
```

Expected: PASS.

---

### Task 2: Document The Notion Archive Contract

**Files:**
- Create: `docs/api/notion-daily-archive.md`

- [ ] **Step 1: Add API contract documentation**

Create `docs/api/notion-daily-archive.md`:

```markdown
# Notion Daily Archive API

## Endpoint

`GET /api/notion/daily-archive?date=YYYY-MM-DD&tzOffsetMinutes=-480`

The endpoint is read-only. It is intended for local Notion Principles OS archival jobs and reuses the same local-date behavior as the frontend daily brief route.

## Response

- `date`: selected local date.
- `generatedAt`: response generation time.
- `archiveTitle`: human title for the archive payload.
- `dailyDiaryTitle`: target INDEXv1 daily diary title.
- `source`: app and endpoint metadata for provenance.
- `status`: daily brief status, or `missing` when the generated daily brief does not exist yet.
- `archiveMarkdown`: human-readable diary text.
- `brief`: generated daily brief when available.
- `fiveHourReports`: same-day 5-hour reports.
- `descriptiveStats`: activity, input, screenshot, app, category, and report counts.
- `hourlyMetrics`: hourly workflow metrics and linked 5-hour report IDs.
- `comparison`: comparison against recent baseline days.

## Notion Principles OS Use

1. Ensure the target Daily Diary exists: `npm run notion:os -- ensure-diary 2026-06-05`.
2. Fetch the archive endpoint from the running TSR collector.
3. Append a diary section with marker `TSR Daily Archive | 2026-06-05`.
4. Verify the diary marker with `npm run notion:os -- verify-diary 2026-06-05` and block readback.

The Notion write job should be idempotent and must not create durable Tasks from report text automatically.
```

- [ ] **Step 2: Check docs for incomplete markers**

Run:

```powershell
rg -n "TB[D]|TO[D]O|fi[l]l in" docs/api/notion-daily-archive.md
```

Expected: no matches.

---

### Task 3: Verify And Commit The API

**Files:**
- Modify: `collector/src/api.rs`
- Modify: `collector/tests/api_tests.rs`
- Create: `docs/api/notion-daily-archive.md`

- [ ] **Step 1: Run focused backend tests**

Run:

```powershell
& $env:USERPROFILE\.cargo\bin\cargo.exe test -p tsr-collector --test api_tests serves_notion_daily_archive_with_human_readable_markdown serves_daily_brief_response_with_stats_and_same_day_reports
```

Expected: both tests PASS.

- [ ] **Step 2: Run API test suite**

Run:

```powershell
& $env:USERPROFILE\.cargo\bin\cargo.exe test -p tsr-collector --test api_tests
```

Expected: all API tests PASS.

- [ ] **Step 3: Check diff hygiene**

Run:

```powershell
git diff --check
git status --short
```

Expected: no whitespace errors; only Task 3 files modified.

- [ ] **Step 4: Commit**

Run:

```powershell
git add collector/src/api.rs collector/tests/api_tests.rs docs/api/notion-daily-archive.md
git commit -m "feat: add Notion daily archive API"
```

---

### Task 4: Write 2026-06-05 Diary Through Notion Principles OS

**Files:**
- Create temporary runtime payload under `D:\CodexInfra\tools\Notion Principles OS\reports\tsr-daily-archive-20260605.json`
- Do not commit secrets or Notion tokens.

- [ ] **Step 1: Ensure the diary exists**

Run from `D:\CodexInfra`:

```powershell
npm run --silent notion:os -- ensure-diary 2026-06-05
```

Expected: JSON page for `INDEX-20260605 | Daily Diary`.

- [ ] **Step 2: Fetch live archive JSON**

With TSR collector running, fetch:

```powershell
Invoke-RestMethod "http://127.0.0.1:5178/api/notion/daily-archive?date=2026-06-05&tzOffsetMinutes=-480"
```

If the collector is running on another port, use the active collector port from the existing dev process or health endpoint.

- [ ] **Step 3: Convert archive Markdown into diary blocks**

Build a payload with marker:

```json
{
  "date": "2026-06-05",
  "visible_section": {
    "marker": "TSR Daily Archive | 2026-06-05",
    "blocks": [
      { "type": "heading_2", "text": "TSR Daily Archive | 2026-06-05" },
      { "type": "paragraph", "text": "..." }
    ]
  }
}
```

Use concise blocks rather than a raw JSON dump. Include sections for daily summary, parallel projects/time allocation, workflow pattern, design/build notes, five-hour reports, descriptive stats, and uncertainty.

- [ ] **Step 4: Append an idempotent visible diary section**

Use the existing `Add-PrinciplesOSDiarySection` helper through a small PowerShell execution block importing `PrinciplesOSRecords.psm1`, or call the direct `api PATCH blocks/<diary_id>/children` route after checking existing children for the marker.

Expected: returns `true` on first append or `false` if the marker already exists.

- [ ] **Step 5: Verify readback**

Run from `D:\CodexInfra`:

```powershell
npm run --silent notion:os -- verify-diary 2026-06-05
npm run --silent notion:os -- blocks <diary_page_id>
```

Expected: diary verification `ok = true`, and block readback contains `TSR Daily Archive | 2026-06-05`.

---

### Task 5: Review Completion

**Files:**
- No new files unless a verification artifact is useful.

- [ ] **Step 1: Re-run completion checks**

Run:

```powershell
& $env:USERPROFILE\.cargo\bin\cargo.exe test -p tsr-collector --test api_tests
git status --short
```

Expected: tests pass and worktree state is understood.

- [ ] **Step 2: Summarize evidence**

Report:
- API endpoint path and exact query for 2026-06-05.
- Commit hash for the TSR API change.
- Test commands and pass counts.
- Notion Daily Diary title, page URL when available, and marker verification result.
- Any residual risk, especially if live TSR data for 2026-06-05 is missing or collector was not running.
