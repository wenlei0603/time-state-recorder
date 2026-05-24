# Next Iteration Architecture Roadmap

> **For agentic workers:** This is an architecture-phase roadmap, not an implementation plan. Do not implement feature code from this document alone. When implementation begins, split each milestone into a Superpowers implementation plan with exact tests, files, code snippets, and commits.

**Goal:** Sequence the next Time State Recorder version so Windows lifecycle handling, Dayflow-style UI, input/IME behavior, Notion integration, and attention metrics can be built without collapsing maintainability.

**Architecture:** Preserve the current Rust collector + SQLite + Axum API + React WebUI stack. Add lifecycle events, analyzer-owned rollups, API v2, layered input events, export jobs, and a metric registry in small, reversible commits.

**Tech Stack:** Rust 2024, Axum, Tokio, SQLite via rusqlite, windows-sys, React 19, TypeScript, Vite, Vitest.

---

## Naming Note

This roadmap follows the user's next-iteration labels. The current repository labels keyboard input as Feature 2 and screenshots as Feature 3. In this roadmap, "Feature 3" refers to the next Input Activity/Chinese IME/product-behavior work.

## Commit Strategy

Use small commits with one architectural layer per commit. Suggested prefixes:

- `docs:` architecture and API contracts.
- `feat(lifecycle):` collector lifecycle events and statistics behavior.
- `feat(query):` API v2 query layer.
- `feat(ui):` WebUI views and component boundaries.
- `feat(input):` layered input/IME model.
- `feat(integrations):` Notion export preview/run.
- `feat(metrics):` attention metric registry and derived metrics.
- `test:` deterministic analyzer/API/UI tests.

Do not mix schema migrations, collector logic, API DTOs, and UI changes in one commit unless a migration task explicitly requires it.

## Cloud Git Workflow

Development happens on remote-tracked feature branches, not directly on `main`.

- Use `codex/<feature-or-milestone>` branch names unless a human requests another naming scheme.
- Keep one branch focused on one milestone or tightly related implementation slice.
- Push the branch to `origin` early with `git push -u origin <branch>`, then push after each meaningful commit group.
- Keep commits small and reviewable: schema/migration, collector, analyzer, API, UI, tests, and docs should usually be separate commits.
- Do not stage unrelated local files. If the worktree is mixed, stage exact paths only.
- Prefer draft PRs while implementation is incomplete.
- Rebase or merge from the remote default branch only at clean checkpoints, never in the middle of unresolved review fixes.
- Every PR description should link the relevant design/implementation plan and list verification commands that actually ran.

## Three-Round Core Review Gate

After each core component is completed, run three review rounds before moving to the next core component. A "core component" means a boundary that other work depends on, such as schema migrations, lifecycle collection, analyzer intervals, API v2 query, layered input, Notion export, or the metric registry.

Round 1: Architecture and Contract Review

- Compare the implementation against the design and implementation plan.
- Focus on module boundaries, data flow, raw-vs-derived separation, API DTO stability, privacy boundaries, and whether any implementation silently changes the architecture.
- Required outcome: all critical and important architecture deviations are fixed or explicitly documented with rationale.

Round 2: Extensibility and Dependency Review

- Focus on future extension points, schema evolution, Windows-specific assumptions, IANA timezone handling, SQLite read/write separation, optional dependencies, build/toolchain requirements, and failure modes.
- Required outcome: no hidden environment assumptions, no one-off abstractions blocking later Notion/wearable/WeChat/lifelog adapters, and all new dependencies are justified.

Round 3: Integration and Operations Review

- Focus on tests, manual Windows checks, launcher behavior, graceful shutdown, migration safety, API compatibility, UI states, redaction enforcement, and rollback/recovery.
- Required outcome: verification evidence is fresh; unresolved risks are documented as follow-up issues before the branch is considered ready.

Review mechanics:

- Use independent subagents for each round when available.
- Give each reviewer the exact base SHA, head SHA, design/plan path, and files changed.
- Reviewers should not inherit implementation context; they should inspect the diff and requirements directly.
- Critical issues block progress. Important issues are fixed before the next component. Minor issues may be batched only if they are not architectural or privacy risks.
- After fixes, rerun the affected review round instead of assuming the fix is correct.

## Milestone 0: Architecture Contracts

Purpose: land traceable design docs before feature code.

Files:

- Create: `docs/superpowers/specs/2026-05-24-next-iteration-architecture-design.md`
- Create: `docs/api/next-query-api.md`
- Create: `docs/superpowers/plans/2026-05-24-next-iteration-architecture-roadmap.md`

Verification:

- `git diff --check`
- Manual placeholder-marker scan and contradictory-scope review.

Commit:

```bash
git add docs/superpowers/specs/2026-05-24-next-iteration-architecture-design.md docs/api/next-query-api.md docs/superpowers/plans/2026-05-24-next-iteration-architecture-roadmap.md
git commit -m "docs: design next iteration architecture and query API"
```

## Milestone 0.5: Schema Migration Foundation

Purpose: make future schema changes explicit, testable, and recoverable before lifecycle/input/external schemas begin changing.

Design files to create or modify:

- Create: `collector/src/migrations/mod.rs`
- Create: `collector/src/migrations/runner.rs`
- Create: `collector/src/migrations/0001_init.sql`
- Modify: `collector/src/storage.rs`
- Create: `collector/tests/migration_tests.rs`
- Create: `collector/tests/fixtures/legacy_v1.sqlite3`

Commit slices:

1. `feat(storage): add schema_migrations table and migration runner`
2. `feat(storage): move initial schema into immutable migration file`
3. `test(storage): migrate legacy fixture database to current schema`
4. `docs(storage): document migration and backup policy`

Design notes:

- Migrations run transactionally and record id, checksum, applied timestamp, and app version.
- Migration files are immutable after merge; changes require a new migration.
- Startup should fail safely if an applied checksum changes.
- Before migrations touching sensitive text or screenshots, the user-facing launcher should recommend backup/export.
- New feature schema tasks should use migrations instead of extending only `CREATE TABLE IF NOT EXISTS`.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
```

---

## Milestone 1: Lifecycle Foundation

Purpose: make Windows lock, shutdown, suspend, resume, and collector gaps observable.

Design files to create or modify:

- Create: `collector/src/lifecycle.rs`
- Create: `collector/src/events.rs`
- Modify: `collector/src/lib.rs`
- Modify: `collector/src/models.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/api.rs`
- Modify: `collector/tests/storage_tests.rs`
- Modify: `collector/tests/api_tests.rs`
- Create: `collector/tests/lifecycle_tests.rs`

Commit slices:

1. `feat(lifecycle): add lifecycle event models and storage`
2. `feat(lifecycle): capture Windows session and power events`
3. `feat(lifecycle): close abnormal sessions on startup`
4. `feat(lifecycle): add soft stop path for launcher and graceful session closure`
5. `test(lifecycle): cover lock suspend resume and restart gaps`

Design notes:

- Add `LifecycleEvent` and `LifecycleType` models before wiring Windows APIs.
- Add storage methods and pure tests before live collector loop changes.
- Use `cfg(windows)` for `WTSRegisterSessionNotification` and `WM_POWERBROADCAST`.
- Keep non-Windows stubs returning `Unavailable` behavior.

Verification commands after milestone:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
```

Manual Windows checks:

- Start collector, lock Windows, unlock Windows, verify lifecycle rows.
- Start collector, stop with launcher, verify graceful `session_stop`.
- Start collector, terminate process, restart, verify previous session closes as `abnormal_stop`.
- Confirm interval construction does not bridge between the old session and the new session.

## Milestone 2: Analyzer-Owned Timeline Intervals

Purpose: move statistics from ad hoc frontend interval inference to deterministic derived intervals.

Design files to create or modify:

- Create: `collector/src/analyzer/mod.rs`
- Create: `collector/src/analyzer/timeline.rs`
- Create: `collector/src/analyzer/screenshots.rs`
- Modify: `collector/src/interval.rs` or replace its responsibilities with analyzer modules.
- Modify: `collector/src/storage.rs`
- Create: `collector/tests/analyzer_timeline_tests.rs`

Commit slices:

1. `feat(query): add timeline interval schema and repository`
2. `feat(query): derive active idle locked suspended offline intervals`
3. `feat(query): derive screenshot coverage counts`
4. `test(query): cover lifecycle-aware interval construction`

Design notes:

- Raw events remain the source of truth.
- `timeline_intervals` can be rebuilt. It must not be the only copy of facts.
- Initial analyzer may run on demand; background scheduled jobs can come later.
- Live UI can still request "derived through now" without persisting open-ended intervals.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
```

## Milestone 3: API v2 Query Layer

Purpose: expose stable timeline, summary, evidence, query, and export preview contracts.

Design files to create or modify:

- Create: `collector/src/query/mod.rs`
- Create: `collector/src/query/dto.rs`
- Create: `collector/src/query/repository.rs`
- Create: `collector/src/query/errors.rs`
- Modify: `collector/src/api.rs`
- Create: `docs/api/openapi.v2.json`
- Modify: `collector/tests/api_tests.rs`
- Create: `collector/tests/query_api_tests.rs`

Commit slices:

1. `feat(query): add API v2 DTOs and error shape`
2. `feat(query): expose lifecycle-aware timeline endpoint`
3. `feat(query): expose day and week summary endpoints`
4. `feat(query): add structured query endpoint with metric validation`
5. `docs(api): add OpenAPI contract for v2 endpoints`
6. `test(query): cover v2 timeline summary and query errors`

Design notes:

- v1 endpoints remain unchanged.
- API v2 error shape must be consistent across endpoints.
- Reject unsafe raw export by policy before querying sensitive fields.
- Query DTOs should be independent from SQLite row structs.
- Date-range filters must be timezone-aware and must not use UTC string-prefix matching for local days.
- Large list endpoints should use cursor pagination.
- v2 query handlers should use read-only SQLite connections or a small read pool and run blocking `rusqlite` work in `spawn_blocking`; collector writes must not wait on long report queries.
- Metrics not supported by current sources should return `null`, `unavailable`, or warnings rather than fake zeros.
- Contract tests should cover timezone boundary days, cursor stability with duplicate timestamps, redaction blocking raw screenshots/window titles, unavailable metric fields, and v1 compatibility during UI migration.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
npm test -- --run
```

## Milestone 4: Dayflow-Style WebUI Refactor

Purpose: make the first screen a workday timeline and move raw rows behind drill-down.

Design files to create or modify:

- Create: `src/api/client.ts`
- Create: `src/api/timeline.ts`
- Create: `src/api/metrics.ts`
- Create: `src/api/query.ts`
- Create: `src/components/TimelineLane.tsx`
- Create: `src/components/ActivityCard.tsx`
- Create: `src/components/LifecycleBand.tsx`
- Create: `src/components/MetricStrip.tsx`
- Create: `src/components/EvidenceDrawer.tsx`
- Create: `src/views/TodayView.tsx`
- Create: `src/views/WeeklyReviewView.tsx`
- Create: `src/views/InputBehaviorView.tsx`
- Create: `src/views/QueryExplorerView.tsx`
- Create: `src/views/StandupSummaryView.tsx` if standup scope is included in the implementation slice
- Create: `src/views/SettingsView.tsx` if privacy/settings scope is included in the implementation slice
- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Create: `src/api/timeline.test.ts`
- Create: `src/views/TodayView.test.tsx`
- Create: `src/views/InputBehaviorView.test.tsx`

Commit slices:

1. `feat(ui): add API v2 timeline client and validation`
2. `feat(ui): add timeline components and sample DTOs`
3. `feat(ui): replace default stats screen with Today view`
4. `feat(ui): add Weekly Review view`
5. `feat(ui): add Input Behavior view with redaction states`
6. `feat(ui): add Query Explorer view`
7. `test(ui): cover timeline client and Today/Input Behavior view states`

Design notes:

- Use dense operational UI, not landing-page layout.
- Keep cards for repeated activity items and panels, not nested decorative cards.
- Use lifecycle bands to show locked/offline/suspended time.
- Keep screenshot thumbnails inspectable but privacy-aware.
- Preserve sample fallback during API migration.
- If Standup Summary and Settings do not fit the v2.0 implementation slice, explicitly mark them deferred in the view navigation and keep Query Explorer limited to saved query templates.

Verification commands:

```powershell
npm test -- --run
npm run build
```

Visual QA:

- Desktop 1440px: timeline, summary strip, and evidence drawer have no overlap.
- Mobile 390px: tabs and timeline rows remain readable.
- Empty day, offline collector, and malformed API payload states are visible.

## Milestone 5: Layered Input And Chinese IME

Purpose: move from key-code text reconstruction to activity/key/composition/edit semantics.

Design files to create or modify:

- Modify: `collector/src/input.rs`
- Create: `collector/src/input/activity.rs`
- Create: `collector/src/input/physical.rs`
- Create: `collector/src/input/uia_text.rs`
- Create: `collector/src/input/diff.rs`
- Modify: `collector/src/models.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/query/dto.rs`
- Create: `collector/tests/input_composition_state_tests.rs`
- Create: `collector/tests/input_diff_tests.rs`
- Create: `collector/tests/input_privacy_tests.rs`
- Create: `collector/tests/input_store_tests.rs`
- Create: `collector/tests/query_input_redaction_tests.rs`
- Create: `collector/tests/fake_uia_adapter_tests.rs`

Commit slices:

1. `feat(input): add layered input event schema`
2. `feat(input): split physical key capture from text segment reconstruction`
3. `feat(input): enforce text_capture blocker and text privacy modes`
4. `feat(input): add grapheme-safe text diff model`
5. `feat(input): add UIA committed-text observation behind feature flag`
6. `feat(input): expose input friction metrics through API v2`
7. `test(input): cover Chinese English delete paste and replace cases`

Design notes:

- Store `method` and `confidence` for each text edit.
- Use count/hash-only mode unless raw text capture is enabled.
- UIA diff is the first Chinese IME route; TSF/WH_GETMESSAGE comes after coverage testing.
- Treat Backspace and Delete as edit intent even when deleted text is unknown.
- Do not write raw `textContent`, `inserted_text`, or `deleted_text` when a `text_capture` blocker matches.
- Automated tests must cover password fields, `text_capture` blocker, blocked deleted text, composition cancel, focus change before commit, paste, selection replace, and CJK/emoji grapheme diffs.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
npm test -- --run
```

Manual Windows matrix:

- Notepad: English, Chinese IME, Backspace, Delete, selection replace, paste.
- VS Code: editor input, file title changes, multiline edits.
- Chrome: normal input, password field, incognito/private window.
- WeChat desktop or another IM app: Chinese input and sensitive-window blocker behavior.

## Milestone 6: Notion Principle Export

Purpose: create reviewable Notion-ready payloads without pushing integration into capture loops.

Design files to create or modify:

- Create: `collector/src/integrations/mod.rs`
- Create: `collector/src/integrations/notion.rs`
- Create: `collector/src/integrations/export_jobs.rs`
- Create: `collector/src/integrations/sources.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/api.rs`
- Create: `src/api/notion.ts`
- Create: `src/views/ExportPreviewView.tsx`
- Create: `collector/tests/notion_export_tests.rs`

Commit slices:

1. `feat(integrations): add source entity and assignment schema`
2. `feat(integrations): add export target and job schema`
3. `feat(integrations): add Notion preview payload builder`
4. `feat(integrations): expose export preview endpoint`
5. `feat(integrations): add export run job state machine`
6. `feat(ui): add Notion export preview screen`
7. `test(integrations): cover redaction provenance and human review flags`

Design notes:

- Export preview must be available before any write.
- Default every generated Notion item to `Needs Human Review = true`.
- Preserve Daily Diary, Raw Materials, Tasks, facets, and DIKW boundaries.
- Store provenance for every exported item.
- Import or reference Notion Principle entities locally before query joins; queries should not call Notion live.

Verification commands:

```powershell
cargo test -p tsr-collector -- --nocapture
npm test -- --run
npm run build
```

## Milestone 6.5: External Observation Foundation

Purpose: define the source/entity/observation envelope before the metric registry so future wearable, WeChat, Notion, and lifelog joins can shape the source bundle contract.

Design files to create or modify:

- Create: `collector/src/external/mod.rs`
- Create: `collector/src/external/observations.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/query/dto.rs`
- Create: `docs/api/external-observations.md`
- Create: `collector/tests/external_observations_tests.rs`

Commit slices:

1. `feat(external): add source entity and observation envelope schema`
2. `feat(external): add external observation schema and idempotent import validation`
3. `feat(query): allow explicit external observation filters in structured queries`
4. `docs: document external observation adapter contract`
5. `test(external): cover import validation privacy levels and duplicate imports`

Design notes:

- This milestone creates the joinable envelope and fixture import path, not full wearable or WeChat adapters.
- Each external row stores provenance, privacy level, confidence, schema version, timezone, redaction state, and native/content hashes.
- Joins happen in query/analyzer layers, never in collector hot path.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
```

---

## Milestone 7: Attention Metric Registry

Purpose: support attention-management engineering metrics and future multimodal joins.

Design files to create or modify:

- Create: `collector/src/metrics/mod.rs`
- Create: `collector/src/metrics/registry.rs`
- Create: `collector/src/metrics/focus.rs`
- Create: `collector/src/metrics/input_friction.rs`
- Create: `collector/src/metrics/context_switch.rs`
- Create: `collector/src/metrics/provenance.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/query/dto.rs`
- Create: `collector/tests/metric_registry_tests.rs`

Commit slices:

1. `feat(metrics): add metric result schema and registry`
2. `feat(metrics): add focus block and context switch metrics`
3. `feat(metrics): add input friction metrics`
4. `feat(metrics): expose metric catalog through query API`
5. `test(metrics): cover provenance and metric versioning`

Design notes:

- Every metric has an id, version, formula, required source tables, and provenance.
- Do not compute opaque scores without evidence links.
- Metrics can be rebuilt from raw data and the external observation envelope. Full external-source metric composition begins after concrete adapters are added.

Verification commands:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
```

## Milestone 8: External Source Adapters

Purpose: add concrete wearable, WeChat, and lifelog-image importers on top of the external observation foundation without changing collector raw tables.

Design files to create or modify:

- Create: `collector/src/external/wearables.rs`
- Create: `collector/src/external/wechat.rs`
- Create: `collector/src/external/lifelog_images.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/query/dto.rs`
- Create: `collector/tests/external_adapter_tests.rs`

Commit slices:

1. `feat(external): add wearable sample importer`
2. `feat(external): add WeChat batch importer`
3. `feat(external): add lifelog image asset importer`
4. `feat(query): add adapter-specific query fixtures`
5. `test(external): cover adapter import privacy and provenance`

Design notes:

- External source adapters normalize into the Milestone 6.5 observation envelope.
- Each adapter has fixture-based tests and privacy docs before it contributes to metrics.

## Cross-Cutting Verification

Run before any implementation branch is considered complete:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
npm test -- --run
npm run build
git diff --check
```

Manual checks:

- Collector still starts through `Start Time State Recorder.bat`.
- `/api/health` remains compatible with current WebUI monitor.
- Old sample-data fallback still works.
- API v1 endpoint behavior is unchanged unless an explicit compatibility note says otherwise.
- API v2 returns lifecycle-aware summaries when lifecycle data exists and safe fallbacks when it does not.

## Stop Conditions

If Windows lifecycle or UIA/IME behavior fails repeatedly because of environment/toolchain issues, stop local patching after five failed attempts and broaden investigation:

- confirm Windows version and permissions
- verify event notifications with a minimal standalone probe
- confirm target app UIA support outside the collector
- isolate compiler/toolchain issues from API/model logic
- document the blocker and continue with non-dependent API or UI work
