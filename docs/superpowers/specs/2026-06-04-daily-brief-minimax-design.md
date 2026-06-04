# Daily Brief MiniMax Design

## Goal

Add a backend-backed Daily Brief workflow for Time State Recorder.

The feature must:

- Fix the Review Notes / summary area so live summaries are clearly driven by backend JSON, not local-only presentation state.
- Generate one MiniMax daily brief at 23:40 local time each day.
- Present a Daily Brief column that includes descriptive daily statistics, hourly activity heatmap metrics, past-day comparison with neutral explanation, all same-day 5h reports, and one final daily action-trajectory report.
- Keep narrative language descriptive and neutral. Do not rank, judge, praise, criticize, or coach the user's work.

## Current State

Frontend:

- `src/App.tsx` calls `refreshAnalysisFeedback()` on live refresh and then polls every 15 seconds.
- That function fetches `/api/analysis-status` and `/api/insight-reports?limit=5`.
- `InsightFeedback` receives `analysisStatus` and `reports` and renders `Review Notes`.
- The current fetch is not date-scoped. It can show latest backend reports, but it does not reliably communicate "for the selected date" and does not support a daily brief.

Backend:

- `collector/src/api.rs` exposes `/api/analysis-status` and `/api/insight-reports`.
- `spawn_insight_report_loop` checks every 5 minutes and calls `maybe_generate_insight_report`.
- `maybe_generate_insight_report` creates 5h reports from `visual_window_summaries`, then persists them in `insight_reports`.
- `insight_reports` currently has no unique 5h period constraint and no date-filtered API.
- There is no daily brief model, table, scheduler, or endpoint.

## Design Direction

Use backend-first aggregation.

The frontend should not construct the Daily Brief from raw local state. It should request a typed backend response and render it. The backend owns:

- date-window resolution,
- hourly metric calculation,
- same-day 5h report selection,
- daily brief prompt construction,
- MiniMax invocation,
- persistence and idempotency.

This matches the existing collector architecture: frontend parsers live in `src/lib/*`, backend storage and model generation live in `collector/src/*`.

## Backend Data Model

Add daily brief models in `collector/src/models.rs`.

### `DailyActivityStats`

Fields:

- `date`
- `periodStart`
- `periodEnd`
- `activeSeconds`
- `activeHours`
- `windowEventCount`
- `switchCount`
- `distinctAppCount`
- `topApps`
- `categoryMix`
- `inputChars`
- `inputEvents`
- `screenshotCount`
- `highResScreenshotCount`
- `visualWindowCount`
- `fiveHourReportCount`
- `firstActivityAt`
- `lastActivityAt`

### `HourlyActivityMetric`

One row per local hour, 0 through 23.

Fields:

- `hour`
- `startAt`
- `endAt`
- `activeSeconds`
- `activeRatio`
- `windowEventCount`
- `switchCount`
- `distinctAppCount`
- `dominantApp`
- `dominantCategory`
- `inputChars`
- `screenshotCount`
- `highResScreenshotCount`
- `visualWindowCount`
- `fiveHourReportIds`

Calculation rules:

- Split events by exact overlap with each local-hour window.
- `activeSeconds` is the sum of foreground/lifecycle active overlap, capped at 3600 per hour.
- `activeRatio = activeSeconds / 3600`.
- `switchCount` is counted from foreground app/title transitions inside the hour.
- `dominantApp` is the app with the largest overlapped active seconds.
- `dominantCategory` is derived from weighted activity buckets or visual window summaries when available; otherwise `unknown`.
- Heatmap intensity is `activeRatio`, not screenshot count.

### `DailyComparison`

Fields:

- `baselineDays`
- `comparedDates`
- `activeSecondsDelta`
- `switchesPerHourDelta`
- `inputCharsDelta`
- `screenshotCoverageDelta`
- `dominantCategoryShift`
- `startTimeShiftMinutes`
- `endTimeShiftMinutes`
- `explanation`

Calculation rules:

- Default baseline is the previous 7 local dates with any activity.
- Deltas compare today against the median baseline value.
- `explanation` is a neutral description of changed patterns, not advice.

### `DailyBrief`

Fields:

- `id`
- `date`
- `periodStart`
- `periodEnd`
- `generatedAt`
- `scheduledForLocal`
- `modelProvider`
- `modelName`
- `promptVersion`
- `status`
- `descriptiveStats`
- `hourlyMetrics`
- `comparison`
- `fiveHourReports`
- `dailySummaryText`
- `actionTrajectory`
- `rawSummaryJson`
- `error`

`dailySummaryText` should be a short neutral overview. `actionTrajectory` should be a chronological account of desktop work inferred from same-day 5h reports. It must not include value judgments such as "productive", "wasted", "good", "bad", "efficient", "inefficient", "should", or coaching language.

## Storage

Add table `daily_briefs` in `collector/src/storage.rs`.

Columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `date TEXT NOT NULL`
- `period_start TEXT NOT NULL`
- `period_end TEXT NOT NULL`
- `generated_at TEXT NOT NULL`
- `scheduled_for_local TEXT NOT NULL`
- `model_provider TEXT NOT NULL`
- `model_name TEXT NOT NULL`
- `prompt_version TEXT NOT NULL`
- `status TEXT NOT NULL`
- `descriptive_stats_json TEXT NOT NULL`
- `hourly_metrics_json TEXT NOT NULL`
- `comparison_json TEXT NOT NULL`
- `five_hour_report_ids_json TEXT NOT NULL`
- `daily_summary_text TEXT NOT NULL`
- `action_trajectory TEXT NOT NULL`
- `raw_summary_json TEXT NOT NULL`
- `error TEXT`

Indexes:

- unique index on `(date, scheduled_for_local)`
- index on `generated_at`

Required storage methods:

- `insert_daily_brief`
- `upsert_daily_brief_error`
- `get_daily_brief_by_date`
- `latest_daily_brief`
- `list_insight_reports_between(start, end, limit)`
- `daily_brief_exists(date, scheduled_for_local)`

Add a unique period index for 5h reports:

- unique `(report_kind, period_start, period_end)`

If this conflicts with historical duplicate rows, implementation should use an insert-or-ignore/upsert strategy for new writes and avoid deleting old rows during this feature.

## Backend Scheduler

Add `spawn_daily_brief_loop`.

Scheduling:

- Default local trigger time: `23:40`.
- Configurable environment variable: `DAILY_BRIEF_LOCAL_TIME`, format `HH:MM`, default `23:40`.
- Use local day boundaries by default, matching existing collector date behavior.
- Check every 60 seconds.
- Generate at or after 23:40 when the local date has no completed daily brief.
- If the collector starts after 23:40, generate the same local date once.
- Do not generate multiple briefs for the same `(date, scheduledForLocal)`.

Manual backfill:

- Add `POST /api/daily-brief/generate?date=YYYY-MM-DD&tzOffsetMinutes=...`.
- Manual generation uses the same idempotent upsert behavior.
- This endpoint is useful when MiniMax credentials were absent at 23:40 or the collector was not running.

Failure handling:

- MiniMax failures should store an error daily brief row with `status = "error"` and enough computed stats to render the page.
- `analysis-status` should expose daily worker status so the UI can show "pending", "running", "error", or "complete".

## MiniMax Daily Brief Reporter

Add a daily reporter in `collector/src/insights.rs`.

Recommended shape:

- `ConfiguredDailyBriefReporter`
- `LocalDailyBriefReporter`
- `MiniMaxDailyBriefReporter`

Provider selection:

- Reuse `MINIMAX_API_KEY`, `MINIMAX_BASE_URL`, and `MINIMAX_MODEL`.
- Add `DAILY_BRIEF_PROVIDER`, defaulting to `minimax` when MiniMax credentials exist, otherwise `local`.
- Add `MINIMAX_DAILY_BRIEF_MAX_COMPLETION_TOKENS`, default `1400`.

Prompt input:

- date and local period bounds,
- descriptive stats,
- hourly metrics,
- comparison object,
- all same-day 5h reports in chronological order.

Prompt output JSON:

```json
{
  "dailySummaryText": "string",
  "actionTrajectory": "string",
  "comparisonExplanation": "string"
}
```

Prompt rules:

- Chinese output.
- No value judgment.
- No advice or coaching.
- Describe what happened, in chronological order.
- Use uncertainty language when evidence is incomplete.
- Prefer "appears", "shows", "is consistent with" over definitive claims.
- Do not include raw screenshot text or keyboard text.

Local fallback:

- Build `dailySummaryText` from stats and dominant categories.
- Build `actionTrajectory` by concatenating same-day 5h report summaries in chronological order.
- Set `modelProvider = "local_insight"`.

## API Contract

### `GET /api/insight-reports`

Extend query parameters:

- `date`
- `tzOffsetMinutes`
- `kind`, default `5h`
- `limit`

If `date` is present, return reports whose `periodStart` / `periodEnd` overlap the selected local day window. Order should be chronological for date-scoped requests and newest-first for legacy limit-only requests.

### `GET /api/daily-brief`

Query:

- `date=YYYY-MM-DD`
- `tzOffsetMinutes`

Response:

```json
{
  "date": "2026-06-04",
  "status": "missing|pending|running|complete|error",
  "nextRunAt": "2026-06-04T15:40:00Z",
  "brief": null,
  "fiveHourReports": [],
  "descriptiveStats": {},
  "hourlyMetrics": [],
  "comparison": {}
}
```

When `brief` exists, it includes the `DailyBrief` fields listed above. `fiveHourReports`, `descriptiveStats`, `hourlyMetrics`, and `comparison` should still be present even when the MiniMax narrative is missing or failed.

### `GET /api/analysis-status`

Extend response:

- `daily`
- `latestDailyBrief`

Older frontend parsers must tolerate these fields being absent.

## Frontend Data Flow

Add frontend types in `src/types.ts`:

- `DailyActivityStats`
- `HourlyActivityMetric`
- `DailyComparison`
- `DailyBrief`
- `DailyBriefResponse`

Add parser/fetcher:

- `src/lib/dailyBrief.ts`
- `fetchDailyBrief(date, fetcher = fetch)`

Update `src/lib/insights.ts`:

- `fetchInsightReports({ date, limit, kind })`
- preserve existing `fetchInsightReports(5)` compatibility only if needed by tests.

Update `src/App.tsx`:

- Store `dailyBriefResponse`.
- Fetch daily brief whenever live source loads, query date changes, or Refresh/Query is clicked.
- Pass `queryDate` into Review Notes and Daily Brief.
- Review Notes should request date-scoped 5h reports instead of latest global reports.
- Error banners should distinguish "Review Notes unavailable" from "Daily Brief unavailable".

This directly addresses the summary communication issue: the summary area should show backend connection state and date-scoped backend data. It should not appear as a static local summary panel.

## Frontend Presentation

Add a Daily Brief column in the Today view and in the Daily Tracking page.

Recommended layout:

- Today view:
  - top: `Review Notes`
  - second: two-column workspace:
    - left: `Daily Brief`
    - right: `Today Flow Board` / evidence drawer
- Daily Tracking tab:
  - rename visible heading to `Daily Brief`
  - keep screenshot timeline as a lower section named `Screenshot Timeline`

Daily Brief sections:

- `Daily Summary`
  - status, generation time, model provider.
  - short neutral `dailySummaryText`.
- `Activity Statistics`
  - active hours, switch count, distinct apps, input chars, screenshot coverage, visual windows, 5h reports.
- `Hourly Heatmap`
  - 24-hour grid or compact rows.
  - intensity from `activeRatio`.
  - each hour shows active duration, dominant app/category, switches, input chars.
- `Past Comparison`
  - deltas against previous 7 active days.
  - neutral explanation.
- `5h Reports`
  - all same-day 5h reports in chronological order.
  - each report is collapsed by default, showing time range and one-line summary.
- `Daily Action Trajectory`
  - model-generated neutral trajectory based on the 5h report collection.
  - collapsible full text if long.

Privacy:

- Stats and heatmap can show in redacted mode because they are aggregate metadata.
- Generated text (`dailySummaryText`, `actionTrajectory`, 5h report body) follows the same rule as Review Notes: hide in redacted mode, show in raw mode.
- Raw screenshots and raw input rows remain separately gated by existing `privacyMode` and layer controls.

Visual style:

- Match the Academic Mac green theme from the Review Notes redesign.
- Use serif typography for generated narrative sections.
- Use compact sans-serif labels for stats and heatmap cells.
- Avoid full-paragraph bold text.
- No raw JSON visible by default; raw response only inside developer-oriented collapsible details if needed.

## Testing Plan

Backend tests:

- `DailyActivityStats` splits activity across hour boundaries correctly.
- Hourly heatmap uses active duration, not screenshot count.
- Daily comparison uses previous active-day median.
- Daily scheduler computes next 23:40 local run correctly.
- Scheduler is idempotent for the same date and local scheduled time.
- Manual `POST /api/daily-brief/generate` creates or updates the expected date.
- `GET /api/daily-brief` returns stats even when MiniMax generation failed.
- `GET /api/insight-reports?date=...` returns same-day reports chronologically.
- MiniMax daily prompt contains only stats and 5h report summaries, not raw screenshots or keyboard text.
- Daily prompt parser rejects missing `dailySummaryText` or `actionTrajectory`.

Frontend tests:

- `fetchDailyBrief` validates the response shape and falls back intentionally on missing optional fields.
- App fetches daily brief on live load and query date change.
- Review Notes uses date-scoped insight reports.
- Daily Brief renders stats and heatmap in redacted mode.
- Daily Brief hides generated narrative in redacted mode.
- Daily Brief shows daily summary, action trajectory, and all same-day 5h reports in raw mode.
- Hourly heatmap renders 24 stable cells without layout shift.

Verification commands:

- `cargo test -p tsr-collector`
- `npm test`
- `npm run build`
- Playwright screenshots for desktop and mobile Daily Brief views
- `git diff --check`

## Acceptance Criteria

- Review Notes and Daily Brief both consume backend JSON in live mode.
- Querying a different date refreshes date-scoped Review Notes, 5h reports, and Daily Brief data.
- At 23:40 local time, the backend creates at most one daily brief for that local date.
- The Daily Brief page can render before MiniMax succeeds, using computed stats and a clear pending/error state.
- The Daily Brief includes descriptive stats, hourly heatmap metrics, past comparison, same-day 5h reports, and a neutral action trajectory.
- The generated report does not contain value judgments or coaching language.
- Redacted mode does not expose generated narrative, screenshots, or raw input text.
- Raw mode presents the generated daily narrative and all same-day 5h report bodies.
- Desktop and mobile screenshots show no text overlap.
