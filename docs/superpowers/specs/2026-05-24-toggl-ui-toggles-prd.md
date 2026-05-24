# Toggl-Style UI Toggles PRD

Date: 2026-05-24
Target branch: `codex/0524-toggl-ui-toggles`
Target release: `v1.1.1` unless API v2 work is added later

## Product Goal

Time State Recorder v1.1.0 has lifecycle-aware data, screenshots, and input segments, but the Web UI still behaves like a technical demo. The next frontend slice should turn it into a daily review workspace inspired by Toggl Track's timer, calendar, timeline, reports, and filtering patterns while staying local-first and privacy-aware.

The goal is not to copy Toggl's team billing product. The goal is to learn its interaction grammar: segmented view switches, timeline blocks, fast filters, compact reports, and clear state toggles.

## Problem

Current UI problems:

- The first screen is statistics-heavy and table-heavy, which makes the product feel like a raw data browser instead of a workday review tool.
- Data source controls are inconsistent across pages. `Sample` and `Live Data` exist, but they are page-local buttons rather than a product-level mode.
- Lifecycle events from v1.1.0 are technically available but not visually obvious. Lock, suspend, collector gaps, and offline time should be visible as timeline states.
- Input Activity exposes raw segments but does not help a Chinese user reason about typing, correction, IME friction, or human-system interaction.
- Privacy controls are not prominent enough for raw text and screenshot-adjacent workflows.

## Users

Primary user:

- A Chinese knowledge worker using Windows who wants to understand work habits, focus blocks, interruptions, typing/editing behavior, and app context over a day.

Secondary user:

- Future personal analytics agents that need stable frontend state concepts before API v2, Notion, wearable, WeChat, and lifelog sources are added.

## Product Principles

- Local-first: assume sensitive data remains local unless an explicit export/integration action exists.
- Review-first: the default screen should answer "what happened today?" faster than it exposes raw rows.
- Toggle-rich but bounded: toggles must change useful analysis dimensions, not create decorative UI.
- Evidence-aware: every metric should be traceable to timeline, input, or screenshot evidence.
- Extensible: frontend state should map cleanly to future `/api/v2/timeline`, `/api/v2/summary`, and `/api/v2/query` contracts.

## Scope

In scope for this branch:

- New product-level app shell with Toggl-style view navigation.
- A daily dashboard that summarizes active time, lifecycle time, focus blocks, top app, input activity, and correction behavior.
- A timeline view derived from current `TimeEvent[]`, including lifecycle rows.
- Global or shared controls for data source, privacy mode, density, granularity, and layer visibility.
- Input Activity improvements focused on correction ratio, input burst review, raw-text privacy toggle, and app filtering.
- Frontend-only derived metrics using existing v1 endpoints and sample data.
- Focused Vitest coverage for pure derivation helpers and UI interaction states.

Out of scope for this branch:

- API v2 implementation.
- Persistent user settings.
- Notion sync implementation.
- Real Toggl integration.
- Manual time entry persistence.
- Live Windows WTS/power event capture beyond the v1.1.0 compatibility data already available.
- New backend schema migrations.

## Toggl Lessons To Apply

Toggl Track's public product surface emphasizes a small set of high-frequency time review concepts: calendar/timeline review, offline/manual tracking modes, automated desktop capture, projects/tags, reporting, and filters. TSR should adapt those into personal analytics concepts:

- Calendar/timeline becomes daily review by hour.
- Project/tag becomes app, status, source, and future Notion Principle category filters.
- Offline/manual tracking becomes sample/live and lifecycle visibility.
- Reporting becomes compact dashboard cards and explainable derived metrics.
- Team billing is intentionally ignored.

## UX Requirements

### App Shell

- Default view should be `Dashboard`, not raw `Statistics`.
- Top navigation should expose `Dashboard`, `Timeline`, `Daily Tracking`, and `Input Activity`.
- The header should show local collector state, current source mode, and a refresh action.
- Data source should be controlled consistently as `Sample` or `Live`, not by separate page-specific button labels.

### Shared Toggles

The UI must include these controls:

- Source toggle: `Sample` / `Live`.
- Privacy toggle: `Redacted` / `Raw`.
- Density toggle: `Comfortable` / `Compact`.
- Granularity toggle for timeline: `Event` / `Hour`.
- Layer toggles: `Windows`, `Lifecycle`, `Input`, `Screenshots`.

Rules:

- `Redacted` is the default privacy mode.
- Raw input text must not be visible unless `Raw` is selected.
- Layer toggles should immediately change visible dashboard/timeline sections.
- `Input` and `Screenshots` layers can use existing endpoints independently; failures should degrade only those panels.

### Dashboard

Dashboard should show:

- Active time total from active window events.
- Lifecycle/non-active time from lifecycle events.
- Top app by active duration.
- Focus blocks: active intervals at least 25 minutes long.
- Context switches: number of app changes in visible active window events.
- Input correction ratio: `(backspace + delete) / keyCount` from input segments.
- Recent timeline preview, showing the latest important window/lifecycle items.

Dashboard should prioritize scanability over dense tables.

### Timeline

Timeline should:

- Render chronological items grouped by hour.
- Use block/pill rows with duration, app/status label, and title summary.
- Distinguish `active_window` and `lifecycle` visually.
- Support layer visibility, granularity, and density toggles.
- In `Hour` granularity, aggregate visible events into hourly buckets while preserving active/lifecycle split.
- Avoid large layout shifts when toggles change.

### Input Activity

Input Activity should:

- Keep the existing segment table but add an insight strip above it.
- Show total keys, total characters, correction ratio, segments, and active app count.
- Add an app filter generated from available segments.
- Respect privacy mode:
  - `Redacted`: show counts and segment metadata, hide raw text content.
  - `Raw`: allow expanded text content.
- Add a "burst" concept: a segment is a burst when it has at least 20 keys or at least 120 seconds of duration.

Chinese/IME-oriented product direction:

- This branch should not pretend to fully identify Chinese IME composition.
- It should expose correction-heavy and burst-heavy segments as the first measurable signals.
- Future work can add IME composition event modeling once the collector records sufficient data.

## Data Requirements

Use existing frontend types:

- `TimeEvent` from `/api/time-events`.
- `ScreenshotSummary` and `ScreenshotMeta` from screenshot endpoints.
- `InputSummary` and `TextSegment` from input endpoints.

Add frontend-only types:

- `UiSourceMode = "sample" | "live"`.
- `PrivacyMode = "redacted" | "raw"`.
- `DensityMode = "comfortable" | "compact"`.
- `TimelineGranularity = "event" | "hour"`.
- `LayerKey = "windows" | "lifecycle" | "input" | "screenshots"`.
- `DashboardSummary`.
- `TimelineItem`.
- `InputInsightSummary`.

These types should be placed in a focused frontend model/helper module, not scattered across components.

## API Requirements

No new backend endpoint is required for this branch.

The frontend should still be shaped so later API v2 replacement is straightforward:

- Centralize fetch and source loading in one hook or helper.
- Keep derived frontend types separate from raw API DTOs.
- Keep privacy mode state explicit even though v1 endpoints cannot enforce backend redaction.

## Error And Loading Requirements

- If collector loading fails, the app should remain usable with sample data and show a clear local error.
- Screenshot or input fetch failure should not break timeline/dashboard.
- Live refresh should not clear existing data until replacement data is successfully parsed.
- Empty states should explain the missing layer without implying data loss.

## Testing Requirements

Unit tests:

- Dashboard summary excludes lifecycle rows from active time.
- Lifecycle duration is reported separately.
- Focus block count uses the 25-minute threshold.
- Context switch count only tracks active window app transitions.
- Timeline layer filtering hides and shows lifecycle/window rows correctly.
- Hour granularity aggregates visible events by local hour.
- Input insight correction ratio handles zero-key segments safely.
- Privacy mode hides expanded raw segment text in redacted mode.

UI tests:

- App default view renders dashboard.
- Timeline tab can be selected.
- Source/privacy/density/layer toggles update labels or visible rows.
- Input Activity redacted mode does not show raw segment content after expansion.

## Success Criteria

- `npm test -- --run` passes.
- `npm run build` passes.
- Existing collector Rust tests are not required unless backend files are changed.
- The branch contains a PRD commit, an implementation plan commit, and development commits with focused scope.
- The UI presents a believable daily review prototype, not just restyled existing tables.

## Release Decision

If only this frontend scope lands, release as `v1.1.1`.

If API v2 endpoints are implemented in the same branch, promote the release target to `v1.2.0` and require backend review.
