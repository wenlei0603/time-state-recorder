# Academic Mac Green UI Design

## Goal

Redesign the Time State Recorder frontend so it feels like a modern academic research workspace on macOS: quiet, precise, readable, and trustworthy. The UI should keep the current product workflows intact while removing the current "AI dashboard demo" tone.

## Current Issues

- The page reads as a prototype dashboard because almost every surface is a bordered card with similar weight.
- The palette is too uniformly pale green, with unrelated blue and yellow accents competing with the requested green theme.
- Labels are frequently uppercase and heavy, which makes the interface feel generated rather than editorial.
- The insight area uses visible "AI" framing too prominently; it should read as field notes, daily review, and evidence.
- Backend JSON data is already parsed by `src/lib/*`, but the UI presentation can still collapse rich structured fields into generic cards.

## Design Direction

Use an "Academic Mac Workspace" style.

- **Primary color:** deep pine green `#1f4d3a` for active controls, important icons, and selected states.
- **Secondary color:** sage green `#a8bdae` for dividers, quiet badges, and inactive affordances.
- **Neutrals:** paper background `#f6f8f3`, white surface `#ffffff`, ink `#17211b`, muted ink `#66736a`.
- **Status color policy:** keep status colors restrained. Use green for healthy/active, amber only for warning, red only for errors, and remove blue from the main product language.
- **Typography:** system macOS stack, lighter label treatment, sentence case labels, strong values only where they carry data.
- **Surfaces:** flat paper sections, hairline dividers, restrained shadow, 6-8px radius. Avoid nested card stacks.

## Layout

### App Shell

The first screen remains the usable app, not a landing page. The shell should look like a macOS productivity app:

- Compact top toolbar with title, source/privacy/density controls, date query, and refresh.
- The product title becomes smaller and calmer. "Time State Recorder" is still visible, but not oversized.
- Navigation becomes a thin segmented tab bar with understated active state.
- Filters sit in one compact command row instead of a bordered card-like band.

### Today View

Today should become the primary research desk:

- Rename visual framing from "AI Insight" toward "Field Notes" or "Review Notes".
- The top notes panel should show:
  - 5-minute window note status.
  - 5-hour report status.
  - next run / collector state.
- The flow board should feel like a study timeline:
  - Activity buckets as compact timeline tiles.
  - Evidence drawer as a side reading pane.
  - Redacted mode should look intentional, not like missing data.

### Dashboard / Monitor

The dashboard should read as an operational appendix:

- Metrics use table-like rows and compact summary strips.
- Collector Monitor keeps health, database, and image retention, but the visual hierarchy is quieter.
- Image Retention should show:
  - local retention period.
  - active and expired file counts.
  - active image storage size.
  - Google Drive reminder only when `pendingGoogleDriveUpload` is true.

## Backend JSON Presentation Contract

The redesign must preserve correct rendering of backend JSON. Components should not infer data shape directly from raw responses; parsing remains in `src/lib/*` and typed models remain in `src/types.ts`.

### `CollectorHealth`

Fields:

- `status`
- `startedAt`
- `uptimeSeconds`
- `version`
- `windowCollector`
- `inputCollector`
- `screenshotCollector`
- `dbStats`

Presentation:

- Render collector status as a small toolbar health item.
- Render subsystem statuses in a compact health table.
- Render `dbStats.imageRetention` as a dedicated storage row group.
- If `imageRetention` is absent from older backend responses, render the parser fallback: 30 days, zero files, no reminder.

### `ImageRetentionStats`

Fields:

- `retentionDays`
- `activeFiles`
- `expiredFiles`
- `activeBytes`
- `expiredBytes`
- `pendingGoogleDriveUpload`
- `googleDriveMessage`

Presentation:

- Format bytes as B, KB, MB, or GB.
- Show "30 days local" as policy metadata.
- Show the Google Drive reminder only when `pendingGoogleDriveUpload` is true.
- Do not imply automatic upload. The message is a user reminder.

### `AnalysisStatus`

Fields:

- `visual`
- `report`
- `latestObservation`
- `latestWindowSummary`
- `latestReport`

Presentation:

- Render worker state as "Window notes", "Report", and "Next run".
- Avoid making "AI" the visible organizing principle.
- Show errors in a clear warning row; do not bury them in decorative cards.

### `VisualWindowSummary`

Fields:

- `summaryText`
- `taskIntent`
- `trajectory`
- `visibleApps`
- `visibleTextHints`
- `riskFlags`
- `confidence`
- `rawSummaryJson`

Presentation:

- `summaryText` is the main field note.
- `taskIntent` appears as the inferred work intent.
- `trajectory` renders as a short sequence of minute-mark observations.
- `visibleApps` and `projectHints` render as quiet chips.
- `rawSummaryJson` is not shown by default. If exposed, it must be in a collapsible monospace "Raw JSON" detail block.

### `InsightReport`

Fields:

- `periodStart`
- `periodEnd`
- `reportKind`
- `summaryText`
- `categoryMix`
- `projectHints`
- `evidenceCount`

Presentation:

- Render as a research note, not an AI card.
- `categoryMix` becomes a compact distribution row or list.
- `evidenceCount` is evidence metadata.

### Activity, Screenshot, and Input JSON

- `ActivityBucket` drives timeline tiles and should preserve local-time display.
- `ScreenshotSummary` and `ScreenshotMeta` stay behind privacy and screenshot layer gates.
- `InputSummary` and `TextSegment` stay behind privacy gates.
- Arrays should render as chips, ordered lists, or capped lists with overflow count; do not dump comma-separated raw arrays into paragraphs.
- Optional fields render as absent or muted fallback text, not as `undefined`, `null`, or broken blank rows.

## Asset Strategy

Image2-generated assets are allowed, but should be subtle and functional.

Recommended generated asset if implementation needs texture:

- A low-contrast sage paper texture for the page background.
- No text, no logos, no people, no devices.
- It should be usable at low opacity behind white app surfaces.
- Target size: 2200x1400 PNG or WebP.
- Store under `src/assets/` or `public/assets/` only if it improves visual polish after screenshot review.

Do not use a large hero illustration, stock-like background, or decorative blobs. This is an app interface, so generated assets should support material quality, not dominate the screen.

## Implementation Boundaries

- Do not change backend endpoints for this design pass.
- Do not weaken existing privacy gates.
- Do not fetch raw screenshot or input rows in redacted mode.
- Do not change `src/lib/*` parser behavior except where a missing backend field needs a tested fallback.
- Prefer CSS variable/token changes and scoped component markup adjustments over broad rewrites.
- Keep controls familiar: segmented controls, icon buttons, compact toolbar actions.

## Acceptance Criteria

- The first viewport looks like a polished macOS academic workspace, not a generated dashboard.
- The palette is clearly deep green plus sage green, with neutral paper/ink support.
- "AI" is not the dominant visible framing.
- Backend JSON fields listed above render correctly or fall back intentionally.
- Redacted mode still hides raw screenshots and text rows.
- Playwright screenshots at desktop and mobile widths show no text overlap.
- Verification commands:
  - `npm test`
  - `npm run build`
  - Playwright screenshot review for desktop and mobile
  - `git diff --check`
