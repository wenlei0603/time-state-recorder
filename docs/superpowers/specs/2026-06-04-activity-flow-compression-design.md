# Activity Flow Compression Design

## Goal

Compress the Activity buckets and Time flow pages so a human can scan a full day without reading hundreds of raw rows.

The feature must:

- Keep raw events and raw 3-minute activity buckets available for inspection.
- Change the default presentation from raw lists to aggregated, readable views.
- Preserve the current privacy model: raw titles and screenshot details remain hidden unless raw mode and screenshot visibility allow them.
- Make Activity buckets and Time flow usable on desktop and mobile without long unbounded card lists.
- Keep the first implementation frontend-derived. No backend schema change is required to make the pages readable.

## Current State

### Time Flow

`src/lib/flowModel.ts` currently builds the Today flow model by sorting `TimeEvent[]`, converting each event into `FlowEvidence`, and then converting each evidence row into one `FlowBucket`.

This means `src/TodayFlowBoard.tsx` renders one button per event in the Time flow lane. On a normal day, this creates a long page that is technically complete but not readable.

### Activity Buckets

`src/ActivityReview.tsx` receives `ActivityBucket[]`, calculates category and attention summaries with `summarizeActivityBuckets`, and then renders every raw bucket as a selectable button.

Because the backend bucket size is commonly 3 minutes, a full day can produce hundreds of rows. The current UI exposes the data collection unit instead of a human review unit.

## Design Direction

Use an overview-first, details-on-demand model.

Raw data remains the source of truth, but the user-facing default should be:

- hourly overview for Time flow,
- 15-minute or 30-minute overview for Activity buckets,
- merged continuous runs for adjacent similar activity,
- selected-hour or selected-run detail panes,
- raw rows only behind a disclosure control.

The compression should happen in derived frontend view models. This keeps storage, backend APIs, and existing raw evidence behavior stable while fixing the unreadable presentation.

## View Modes

Add one shared density control where both pages need it:

- `Overview`: default. Uses hourly Time flow and 30-minute Activity bucket summaries.
- `15m`: denser Activity bucket summary for closer review.
- `30m`: explicit half-hour Activity bucket summary.
- `Raw`: shows raw source units, but still inside a bounded detail region rather than an unbounded full-page list.

Default mode:

- Time flow: `Overview`.
- Activity buckets: `Overview`, equivalent to 30-minute bands plus merged runs.

The UI may expose this as a segmented control. It should not be a text-heavy explanation block.

## Time Flow Compression

### Derived Types

Add compressed flow types in `src/types.ts` or keep them local in `src/lib/flowModel.ts` if they are only used by `TodayFlowBoard`.

Suggested shape:

```ts
export type FlowHourGroup = {
  id: string;
  hour: number;
  startAt: string;
  endAt: string;
  activeSeconds: number;
  uncertainSeconds: number;
  eventCount: number;
  switchCount: number;
  microSwitchCount: number;
  dominantApp: string;
  dominantTitle: string;
  confidence: FlowConfidence;
  evidence: FlowEvidence[];
  runs: FlowRun[];
};

export type FlowRun = {
  id: string;
  startAt: string;
  endAt: string;
  durationSeconds: number;
  app: string;
  title: string;
  kind?: "active_window" | "lifecycle";
  status?: string;
  confidence: FlowConfidence;
  eventCount: number;
  evidence: FlowEvidence[];
};
```

Extend `TodayFlowModel` with:

- `hourGroups: FlowHourGroup[]`
- `runCount: number`
- `microSwitchCount: number`

Keep the existing `buckets` and `evidence` fields for compatibility and raw detail.

### Grouping Rules

Build `FlowRun[]` from ordered `FlowEvidence[]`.

Merge adjacent evidence rows into the same run when all conditions hold:

- same `app`,
- same privacy-safe `title` after redaction has already been applied,
- same `kind`,
- same `status` for lifecycle events,
- gap between previous `endedAt` and next `startedAt` is less than or equal to 60 seconds.

Split the run when:

- app changes,
- visible title bucket changes,
- lifecycle status changes,
- gap is greater than 60 seconds,
- the run crosses an hour boundary.

Micro switch rule:

- A run shorter than 30 seconds is a micro switch unless it is the only run in the hour.
- In `Overview`, micro switches are counted and folded into the hour group.
- In selected-hour details, show at most the first 5 micro switches, followed by a count such as `+12 micro switches`.
- In `Raw`, show all original rows inside the raw disclosure.

### Hour Groups

Build 24 possible local-hour groups, but render only hours that have data by default. If visual balance needs a heatmap, render all 24 hour cells compactly and make empty hours quiet.

Each hour group should contain:

- hour label,
- active duration,
- uncertain duration,
- dominant app,
- dominant title or redacted title,
- event count,
- switch count,
- micro switch count,
- confidence,
- top 3 runs by duration or chronological order.

Switch count is the number of app/title/run changes inside the hour. It should not count every evidence row if multiple rows were merged into a single run.

### Time Flow UI

Replace the unbounded `model.buckets.map(...)` lane in `src/TodayFlowBoard.tsx`.

Default layout:

- top metrics remain,
- a compact 24-hour heat strip or vertical hour list appears in the Time flow panel,
- each hour row has fixed height and stable columns,
- selecting an hour opens a detail pane in the existing evidence drawer area,
- the detail pane shows top runs and evidence, not every raw event.

Selected-hour detail should show:

- selected hour time range,
- active duration,
- dominant app/title,
- run list capped to the most important or chronological 8 runs,
- screenshot evidence already filtered by the selected hour or selected run,
- `Show raw events` disclosure for the original evidence rows.

## Activity Bucket Compression

### Derived Types

Add derived activity types in `src/lib/activity.ts` or a new `src/lib/activityCompression.ts`.

Suggested shape:

```ts
export type ActivityBand = {
  id: string;
  startAt: string;
  endAt: string;
  seconds: number;
  bucketCount: number;
  activeSeconds: number;
  dominantApp: string;
  dominantTitle: string;
  activityCategory: ActivityCategory;
  attentionState: AttentionState;
  confidence: number;
  switchCount: number;
  visualSummaryCount: number;
  buckets: ActivityBucket[];
};

export type ActivityRun = {
  id: string;
  startAt: string;
  endAt: string;
  durationSeconds: number;
  bucketCount: number;
  dominantApp: string;
  dominantTitle: string;
  activityCategory: ActivityCategory;
  attentionState: AttentionState;
  confidence: number;
  switchCount: number;
  visualSummaryIds: number[];
  buckets: ActivityBucket[];
};
```

### Banding Rules

Create fixed time bands from `ActivityBucket[]`.

Supported band sizes:

- 15 minutes,
- 30 minutes,
- 60 minutes for overview metrics if needed.

For each band:

- include buckets whose `startAt` falls inside the band,
- `seconds` is the sum of `bucketSeconds`,
- `activeSeconds` is the sum of `dominantDurationSeconds`,
- `dominantApp` is the app with the largest total dominant duration,
- `activityCategory` is the category with the largest bucket-second share,
- `attentionState` is the attention state with the largest bucket-second share,
- `confidence` is the bucket-second-weighted mean confidence,
- `switchCount` is the sum of switch counts.

### Run Rules

Build `ActivityRun[]` from sorted buckets.

Merge adjacent buckets when all conditions hold:

- same `activityCategory`,
- same `attentionState` or compatible attention state,
- same `projectId` when both buckets have a project id,
- same `dominantApp` or same normalized title bucket,
- time gap is less than or equal to one bucket length.

Compatible attention states:

- `deep_focus` can merge with `steady`,
- `light_switching` can merge with `fragmented`,
- `away` only merges with `away`,
- `unknown` only merges with `unknown`.

Split a run when:

- category changes,
- attention state becomes incompatible,
- app/title context changes sharply,
- gap is larger than one bucket length,
- duration would exceed 90 minutes.

The 90-minute cap prevents one long run from swallowing an entire day and keeps the UI scannable.

### Activity UI

Replace the default raw bucket list in `src/ActivityReview.tsx`.

Default layout:

- summary metrics stay near the top,
- category mix and attention rhythm remain compact,
- add an hourly or half-hour heatmap for activity intensity,
- show an `Activity runs` list instead of every raw bucket,
- selecting a run opens the existing evidence area,
- raw buckets live behind `Show raw buckets`.

Each run row should show:

- time range,
- duration,
- category,
- attention state,
- dominant app,
- confidence,
- switch count,
- visual summary count if available.

The selected-run evidence pane should show:

- run-level summary metrics,
- representative buckets capped to 8,
- visual summaries inside the selected run,
- raw bucket disclosure with all source buckets.

## Privacy Behavior

Compression must not weaken privacy boundaries.

Rules:

- In redacted mode, all titles use the existing redacted label.
- Derived dominant title must be computed after privacy redaction if it is rendered.
- Raw title text appears only when `privacyMode === "raw"`.
- Screenshot thumbnails still require screenshot visibility and raw mode.
- Raw disclosures respect the same privacy guards as current evidence rows.

## JSON Rendering

The compressed UI must render structured backend JSON fields instead of dumping JSON text.

Rules:

- Arrays render as capped token lists or structured rows.
- Long strings render as bounded paragraphs with a disclosure for full text.
- Unknown JSON fields are ignored in compact views and may appear only in debug/raw sections.
- No user-facing component should display escaped JSON such as ```json { ... }``` as normal summary text.

This requirement applies to the selected-hour and selected-run detail panes when they include visual summaries or report fragments.

## Implementation Plan

1. Add compression helpers.
   - Add flow run and hour grouping helpers in `src/lib/flowModel.ts`.
   - Add activity band and run helpers in `src/lib/activity.ts` or `src/lib/activityCompression.ts`.

2. Update types.
   - Extend `TodayFlowModel` with hour groups and compression counts.
   - Add activity compression types if shared across components.

3. Update Time flow rendering.
   - Replace raw `model.buckets.map(...)` default lane in `src/TodayFlowBoard.tsx`.
   - Add selected hour state.
   - Keep raw event disclosure.

4. Update Activity buckets rendering.
   - Replace raw bucket button list in `src/ActivityReview.tsx`.
   - Add density control and selected run state.
   - Keep raw bucket disclosure.

5. Update styling.
   - Use fixed row heights, compact typography, and stable columns.
   - Avoid nested cards.
   - Ensure mobile wraps into readable rows without overlapping text.

6. Add tests.
   - Unit-test flow run merging, hour splitting, micro switch counting, and redacted title handling.
   - Unit-test activity banding, run merging, confidence weighting, and 90-minute split behavior.
   - Component-test default render does not expose hundreds of raw rows.

## Acceptance Criteria

- Time flow default view renders no more than 24 primary time units for a full day.
- Activity buckets default view does not render every raw 3-minute bucket as a primary list row.
- Raw events and raw buckets remain inspectable through explicit disclosure controls.
- Redacted mode never leaks raw titles through dominant-title aggregation.
- A day with hundreds of events remains readable at desktop width without unbounded card columns.
- A 390px mobile viewport has no overlapping text in Time flow or Activity buckets.
- `Overview`, `15m`, `30m`, and `Raw` modes produce predictable list sizes.
- Visual summaries and backend JSON fields render as structured text, not raw JSON dumps.
- Existing source data and backend API contracts remain unchanged.

## Non-Goals

- Do not delete raw events or raw activity buckets.
- Do not change backend retention policy.
- Do not add new backend endpoints solely for compression.
- Do not make value judgments about productivity or quality of work.
- Do not hide uncertainty. Low-confidence or uncertain evidence should remain visible at the group level.

## Verification

Run:

- `npm test -- --run`
- `npm run build`

Manual checks:

- Load a day with many raw events.
- Confirm Time flow shows hour groups by default.
- Confirm Activity buckets shows runs or bands by default.
- Switch to raw mode and confirm original detail is still reachable.
- Switch to redacted mode and confirm raw titles do not appear in any group, run, detail, or disclosure.
- Check desktop and 390px mobile screenshots for text overflow and overlap.
