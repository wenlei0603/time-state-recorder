# Activity Insights And Visual Summary PRD

Date: 2026-05-25
Target branch: `codex/timeline-activity-buckets`
Target release: `v1.2.0`

## Product Goal

Time State Recorder should evolve from a window-event recorder into a local-first work insight system. The product should help a person answer:

- What was I actually doing during the day?
- Which work belonged to which project?
- Where did my attention fragment?
- Which activity looked like focused work, communication, administration, learning, or non-work browsing?
- What visual evidence supports those conclusions?

The next feature should introduce 3-minute activity aggregation, human-readable activity categories, and screenshot visual summaries. The goal is not surveillance. The goal is personal review and long-term self-analytics.

## Problem

Current Timeline behavior is too granular for reflection:

- Every foreground window change can become a separate event.
- Browser tab changes appear as many short rows.
- The UI exposes evidence but does not explain what the user was doing.
- Screenshots exist, but thumbnails are not semantically summarized.
- There is no category layer for project work, communication, loafing/non-work, context switching, or focus quality.

Users need a view that compresses raw events into meaningful work states while keeping evidence available for audit.

## Users

Primary user:

- A Windows knowledge worker who wants daily self-review, project-based time understanding, and attention-pattern insight.

Secondary users:

- Future personal analytics agents that can use stable local data for weekly/monthly statistics.
- Future automation features that suggest manual project labels or productivity categories from evidence.

## Product Principles

- Local-first: raw screenshots, OCR, titles, and model summaries stay local unless the user explicitly configures a remote model.
- Evidence-backed: every insight should be traceable to time buckets, app/window metadata, screenshot summaries, or screenshots.
- Human-friendly by default: the main view should show 3-minute activity buckets and focus blocks, not raw event rows.
- Re-aggregatable: preserve raw window events so the user can change bucket size, category rules, or model prompts later.
- Non-judgmental categories: labels like loafing are user-facing only if the user chooses them; default wording should support reflection rather than shame.
- Statistical durability: store structured summaries that can be analyzed across days, weeks, and projects.

## Scope

In scope:

- Backend endpoint for 3-minute activity buckets.
- Category model for project, activity type, attention state, and evidence confidence.
- Frontend Activity Review view optimized for self-insight.
- Browser-tab distinction using normalized window titles.
- 5-minute high-resolution screenshot capture path.
- Visual model analysis interface for screenshot summaries.
- Storage for screenshot summary text and structured visual tags.
- Tests for bucket aggregation, category assignment, browser title normalization, and API parsing.

Out of scope:

- Browser extension for URL capture.
- Automatic billing-grade project attribution.
- Cloud sync.
- Team monitoring.
- Real-time screen streaming.
- Full OCR pipeline unless required by the selected visual model provider.
- Enforcing productivity morality. Categories are descriptive and editable.

## Activity Bucket Model

Default bucket size: `180 seconds`.

The backend should aggregate raw `TimeEvent` intervals into fixed 3-minute buckets. Each bucket represents the best answer to "what was happening in this time slice?"

Required fields:

```text
id
startAt
endAt
bucketSeconds
dominantApp
dominantTitle
normalizedTitle
dominantDurationSeconds
switchCount
projectId
projectName
activityCategory
attentionState
confidence
evidence
visualSummaryId
```

Rules:

- `dominantApp` and `dominantTitle` come from the window/title with the most active seconds in the bucket.
- `switchCount` counts foreground identity changes inside the bucket.
- `attentionState` is derived from switch frequency and dominant duration share.
- `projectId/projectName` can be manual, rule-based, or inferred.
- Raw evidence remains queryable, but the default frontend displays buckets and focus blocks.

## Category System

The category layer should have multiple dimensions instead of one overloaded label.

### Project Dimension

Purpose: answer "which project did this belong to?"

Fields:

```text
projectId
projectName
projectSource
projectConfidence
```

Allowed `projectSource`:

- `manual`: user explicitly assigned.
- `rule`: matched by app/title/category rules.
- `model`: inferred from screenshot/title summary.
- `unknown`: no project match.

Examples:

- `PhD Paper`
- `Time State Recorder`
- `Teaching`
- `Personal Admin`
- `Unknown`

### Activity Category Dimension

Purpose: answer "what type of activity was this?"

Initial categories:

- `project_work`: direct work on a known project.
- `research`: reading, searching, papers, documentation, knowledge gathering.
- `writing`: writing or editing documents.
- `coding`: source code, terminal, IDE, tests.
- `communication`: WeChat, email, chat, messages.
- `meeting`: video calls, conferencing, calendar calls.
- `admin`: file management, forms, system settings, scheduling.
- `learning`: courses, tutorials, lectures, explanatory videos.
- `planning`: task managers, notes, Notion planning, calendars.
- `loafing`: user-defined non-work or distraction activity.
- `personal`: personal errands that are intentional but not project work.
- `idle`: lock, idle, sleep, away, collector gap.
- `unknown`: insufficient evidence.

`loafing` should be rule-driven or user-confirmed by default. The system should avoid confidently calling something loafing based only on app name unless the user has configured that rule.

### Attention State Dimension

Purpose: answer "how fragmented was this time?"

Initial states:

- `deep_focus`: low switching, one dominant app/title, bucket continuity.
- `steady`: mostly one activity with minor switches.
- `light_switching`: several switches but one clear dominant task.
- `fragmented`: many switches, no clear dominant task.
- `away`: idle, lock, suspend, or session gap.
- `unknown`: insufficient evidence.

Default heuristics for a 3-minute bucket:

- `deep_focus`: `switchCount <= 1` and dominant share >= 85%.
- `steady`: `switchCount <= 2` and dominant share >= 65%.
- `light_switching`: `switchCount <= 5` or dominant share >= 45%.
- `fragmented`: otherwise.

These are heuristics, not moral judgments. They should be visible and tunable later.

## Browser Tab Handling

First version should distinguish browser tabs using process name plus normalized window title.

Normalization should:

- Remove browser suffixes like `- Google Chrome`, `- Microsoft Edge`, and localized equivalents.
- Remove noisy page-count suffixes like "and 2 more pages" or Chinese equivalents.
- Trim whitespace and repeated separators.
- Preserve meaningful page title content.

Examples:

```text
"GitHub - Pull Request - Google Chrome" -> "GitHub - Pull Request"
"Time State Recorder 和另外 2 个页面 - 个人 - Microsoft Edge" -> "Time State Recorder"
```

Limitation: without a browser extension, the system cannot reliably capture URL. The PRD intentionally treats URL capture as future work.

## Visual Summary Feature

The next feature should add a visual model analysis interface for screenshots. This is separate from the existing low-resolution screenshot thumbnails.

### Capture Requirements

- Keep existing lightweight screenshots for timeline thumbnails.
- Add high-resolution screenshot capture every 5 minutes.
- Default high-resolution capture interval: `300 seconds`.
- Target resolution should preserve readable text when possible, with configurable downscale.
- Store high-resolution screenshots separately from thumbnail screenshots.
- Apply blocker rules before saving or analyzing high-resolution screenshots.

### Analysis Requirements

For each eligible high-resolution screenshot, the system should create a visual summary record.

Required fields:

```text
id
screenshotId
capturedAt
modelProvider
modelName
promptVersion
summaryText
activityCategory
projectHints
visibleApps
visibleTextHints
riskFlags
confidence
createdAt
error
```

The model should summarize what is visible in a human-useful way:

- concise description of current work context
- likely project or topic
- visible app/site/document type
- whether the screen appears work-related, communication-heavy, idle, or non-work
- evidence that supports the label

The model should not produce sensitive full-text extraction by default. It should summarize without copying private content unless `Raw` mode and explicit local-analysis settings allow it.

### API Requirements

Backend endpoints:

```text
GET /api/activity-buckets?date=YYYY-MM-DD&bucketSeconds=180
GET /api/visual-summaries?date=YYYY-MM-DD
POST /api/screenshots/{id}/analyze
POST /api/visual-summaries/analyze-range
```

`POST /api/screenshots/{id}/analyze` should support manual re-analysis.

`POST /api/visual-summaries/analyze-range` should support batch analysis for a date range, with rate limits and progress reporting.

Model provider abstraction:

```text
provider = local | openai-compatible | custom-http
model
baseUrl
apiKeyEnv
timeoutSeconds
maxImagePixels
```

The first implementation can support a local or OpenAI-compatible HTTP endpoint behind a provider interface. The UI should not hard-code a vendor.

## Frontend UX

Add or reshape the main view into `Activity Review`.

Default mode:

```text
Activity Review
Granularity: 3 min
Privacy: Redacted
Evidence: Summary first, screenshots opt-in
```

### Top Insight Panel

Show a concise daily summary:

```text
Today: 4h 20m project work, 52m communication, 31m fragmented time.
Longest focus block: 48m on Time State Recorder.
Most fragmented period: 14:00-15:00, mainly Chrome / WeChat / Notion.
```

### Category Cards

Show time totals by category:

```text
Project Work
Research
Writing
Communication
Planning
Loafing / Non-work
Idle / Away
Unknown
```

Each card should show total duration, trend indicator if historical data exists, and top evidence.

### Focus Blocks

Merge consecutive compatible 3-minute buckets into human-readable blocks:

```text
09:12-10:03  Time State Recorder / coding / deep focus
10:03-10:18  Browser research / research / steady
10:18-10:27  WeChat / communication / fragmented
```

Each block can expand into:

- bucket list
- app/title evidence
- screenshot thumbnails
- visual model summary
- manual project/category correction controls

### Switching Insight

Show attention fragmentation explicitly:

```text
Switching score: Moderate
High-switch buckets: 7
Most interrupted project: PhD Paper
```

The UI should treat this as descriptive, not accusatory.

### Visual Summary Timeline

Every 5-minute high-resolution screenshot summary should appear as evidence markers:

```text
10:05  Visual summary: editing a Rust API file and reviewing tests.
10:10  Visual summary: browser page about Windows raw input API.
```

In `Redacted` mode, show summary text and hide image.
In `Raw` mode, allow image expansion.

## Manual Correction

The system should allow later correction of:

- project
- activity category
- attention state if the user disagrees
- visual summary quality

Corrections should become training/rule data for future categorization.

First version may store corrections locally only.

## Long-Term Analytics

This feature should enable later statistics such as:

- project time by day/week/month
- deep focus hours by project
- loafing/non-work time trend
- communication load trend
- context switch rate by hour
- most fragmented projects
- browser-topic distribution
- visual summary topic clustering
- relationship between screenshots, input bursts, and focus blocks

The data model should preserve enough structure for these analyses without requiring reprocessing raw images every time.

## Privacy And Safety

Privacy defaults:

- `Redacted` UI mode by default.
- Screenshot images hidden unless `Raw` mode.
- Visual summaries allowed in redacted mode only if they avoid sensitive verbatim extraction.
- Blocker rules apply before high-resolution capture and before model analysis.
- Remote model providers require explicit configuration.

Risk controls:

- Add model-analysis disabled state by default if no provider is configured.
- Log model errors without blocking core recording.
- Store prompt version to support future re-analysis.
- Provide a clear way to delete visual summaries and high-resolution screenshots for a date.

## Technical Architecture

The implementation should follow a pipeline architecture with explicit module boundaries:

```text
Windows Signals
  -> Capture Layer
  -> Raw Evidence Store
  -> Derivation Engine
  -> Insight Engine
  -> Local API
  -> Activity Review UI

High-resolution Screenshots
  -> Privacy/Blocker Gate
  -> Visual Analysis Queue
  -> Visual Model Provider
  -> Visual Summary Store
  -> Insight Engine
```

### Architecture Principles

- Preserve raw evidence. Window events, lifecycle events, screenshots, and input segments are the source of truth.
- Keep aggregation re-computable. Activity buckets, focus blocks, categories, and daily insights are derived views.
- Keep model analysis optional and asynchronous. A missing or slow visual model must not stop recording.
- Separate database rows from API DTOs. API responses should stay stable even if schema changes.
- Put privacy gates at capture, analysis, and response boundaries.
- Prefer pure derivation functions with deterministic tests.
- Introduce schema migrations before adding new persistent tables.

### Backend Module Boundaries

Recommended module layout:

```text
collector/src/
  capture/
    window.rs
    input.rs
    screenshot_thumb.rs
    screenshot_hires.rs
  domain/
    models.rs
    activity_bucket.rs
    category.rs
    attention.rs
    visual_summary.rs
  derive/
    interval.rs
    bucketizer.rs
    focus_blocks.rs
    browser_title.rs
  insight/
    category_rules.rs
    day_summary.rs
    long_term_metrics.rs
  visual/
    provider.rs
    openai_compatible.rs
    local.rs
    jobs.rs
  storage/
    schema.rs
    migrations.rs
    repositories.rs
  api/
    routes.rs
    dto.rs
```

The first implementation does not need to move every existing file immediately. It should add new work in this direction and avoid making `api.rs` or `storage.rs` responsible for more behavior.

### Layer Responsibilities

Capture layer:

- Reads OS and screen signals.
- Emits normalized domain structs.
- Does not classify project, activity, or loafing.

Raw evidence store:

- Persists immutable facts and file paths.
- Owns SQLite access and file location conventions.
- Does not return frontend-specific summaries directly.

Derivation engine:

- Converts raw intervals into `ActivityBucket`.
- Normalizes browser titles.
- Builds focus blocks and switching metrics.
- Should be mostly pure Rust functions.

Insight engine:

- Applies project/category rules.
- Computes attention state.
- Produces day-level summaries and long-term metrics.
- Consumes visual summaries as evidence, not as the only truth.

Visual analysis pipeline:

- Stores high-resolution screenshots only after blocker checks.
- Creates analysis jobs.
- Calls a provider through a trait/interface.
- Persists model output, errors, prompt version, and provider metadata.

API layer:

- Exposes DTOs designed for frontend and future compatibility.
- Applies redaction and privacy policy before returning sensitive fields.
- Supports stable query parameters such as `date`, `bucketSeconds`, and `privacyMode`.

Frontend layer:

- Keeps API clients separate from UI components.
- Keeps frontend derivation helpers pure when possible.
- Treats `Activity Review` as the primary user-facing workflow and raw event views as evidence drill-down.

### Data Lifecycle

1. Capture raw facts: window focus, lifecycle, input, thumbnails, and high-resolution screenshots.
2. Store facts in SQLite and image files with session/date metadata.
3. Derive time intervals and 3-minute activity buckets.
4. Enrich buckets with category, project, attention state, and visual summary references.
5. Present focus blocks, category totals, switching insight, and evidence in the Web UI.
6. Store user corrections as rules or overrides.
7. Recompute future summaries from raw facts plus corrections.

### Storage Design

New persistent tables should be additive and migration-backed:

```text
schema_migrations
screenshot_hires
visual_summary_jobs
visual_summaries
activity_category_rules
activity_overrides
project_rules
```

Recommended policy:

- Do not store model output inside screenshot rows.
- Do not store binary screenshots in SQLite.
- Store prompt version and provider metadata with each visual summary.
- Store user corrections separately from derived buckets so buckets can be rebuilt.
- Add indexes for date/time lookups and foreign keys for screenshot-summary relationships.

### Visual Model Provider Contract

The model provider should be represented by an interface rather than direct API calls in route handlers.

Required behavior:

```text
analyze_screenshot(input) -> VisualSummaryResult
```

Provider implementations:

- `NoopProvider`: default when no model is configured.
- `OpenAICompatibleProvider`: HTTP endpoint with model and API key environment variable.
- `LocalProvider`: local model endpoint such as Ollama or another OpenAI-compatible local server.

Route handlers should enqueue work or trigger a bounded job, not own provider-specific code.

### Error Handling And Observability

- Each collector and analysis worker should publish health state.
- Visual analysis failures should be stored per job and visible in API responses.
- Long-running batch analysis should be cancellable or resumable.
- API errors should distinguish invalid query, provider disabled, model timeout, and storage failure.
- Existing recording must continue if visual analysis fails.

### Versioning And Compatibility

- Keep existing v1 endpoints stable.
- Add new endpoints without replacing `/api/time-events` immediately.
- Consider `/api/v2/activity-buckets` only when the DTO shape is stable enough for external consumers.
- Frontend should consume the new activity endpoints through isolated client modules.

## Backend Requirements

New or changed backend modules:

- Activity bucket aggregation from existing `TimeEvent`.
- Title normalization for browser tabs.
- Category rule engine.
- High-resolution screenshot capture loop.
- Visual summary storage.
- Visual model provider abstraction.

The raw collector should continue recording foreground changes at the current poll interval. Aggregation should be a query/storage layer, not a destructive change to collection.

## Testing Requirements

Rust tests:

- 3-minute bucket aggregation chooses dominant activity by duration.
- Bucket switch counts are correct.
- Lifecycle/idle events produce `away` buckets.
- Browser title normalization removes Chrome/Edge suffixes.
- Category rules assign configured project and category.
- Loafing category is not inferred without rule/model evidence.
- Visual summary insert/list works.
- Blocker rules prevent high-resolution capture analysis.

Frontend tests:

- Activity Review defaults to 3-minute view.
- Category cards render project work, communication, loafing/non-work, idle, and unknown.
- Focus blocks merge adjacent compatible buckets.
- Fragmented buckets show switching insight.
- Redacted mode hides images but shows visual summary text.
- Raw mode can expand evidence thumbnails.

Integration tests:

- `/api/activity-buckets?bucketSeconds=180` returns stable JSON.
- `/api/visual-summaries?date=...` returns summaries linked to screenshots.
- Missing model provider returns a clear disabled/error state.

## Success Criteria

- A user can open the app and understand the day without reading raw event rows.
- Default Timeline/Activity Review uses 3-minute aggregation.
- Browser tab titles are distinguishable enough for topic review.
- Categories include project-based work, communication, loafing/non-work, idle, and unknown.
- Switching frequency is visible as a first-class insight.
- High-resolution screenshots are captured every 5 minutes when enabled.
- Visual summaries can be generated and displayed as evidence.
- Raw images remain hidden in redacted mode.
- The data structure supports future weekly/monthly statistics.

## Open Decisions

- Whether high-resolution screenshot capture should be enabled by default or require explicit opt-in.
- Which visual model provider should be supported first.
- Whether project/category corrections need a UI in the first implementation slice.
- Whether visual summaries should be generated automatically or queued for manual batch analysis.
- How aggressive the first loafing/non-work rules should be.

## Recommended First Implementation Slice

Implement in this order:

1. Backend `/api/activity-buckets` with 3-minute aggregation, switch count, normalized browser titles, and basic category fields.
2. Frontend Activity Review with category cards, focus blocks, and switching insight.
3. High-resolution 5-minute screenshot capture storage behind a config flag.
4. Visual summary storage and provider interface.
5. Manual "Analyze screenshot" action for one screenshot.
6. Batch analysis only after single-screenshot analysis is stable.
