# Toggl-Style UI Toggles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a v1.1.1 frontend prototype with Toggl-style dashboard, timeline, and useful toggles over the existing Time State Recorder data.

**Architecture:** Keep the current React/Vite frontend and unversioned v1 collector endpoints. Add a pure frontend model layer for dashboard, timeline, filters, and input insights; then wire it into focused React components. Keep all privacy and layer state explicit so the UI can later swap to `/api/v2/timeline` without rewriting the interaction model.

**Tech Stack:** React 19, TypeScript, Vite, Vitest, Testing Library, lucide-react, existing Rust collector API.

---

## File Structure

- Create: `src/lib/uiModel.ts`
  - Owns UI-only types and pure derivation helpers.
  - No React imports.
  - Functions: `buildDashboardSummary`, `filterTimelineEvents`, `buildTimelineItems`, `buildHourlyTimelineItems`, `summarizeInputInsights`, `listSegmentApps`, `formatDuration`.
- Create: `src/lib/uiModel.test.ts`
  - TDD coverage for dashboard, timeline filters, hourly grouping, and input insights.
- Create: `src/Dashboard.tsx`
  - Renders summary metrics, focus/context cards, and a recent timeline preview.
- Create: `src/TimelineView.tsx`
  - Renders event/hour timeline with layer and density support.
- Modify: `src/App.tsx`
  - Replace statistics-first default with dashboard-first shell.
  - Add top-level source, privacy, density, granularity, and layer toggles.
  - Keep legacy statistics table accessible enough for debugging through dashboard/timeline detail rather than as the default first screen.
- Modify: `src/InputActivity.tsx`
  - Accept shared `privacyMode`.
  - Add insight strip, app filter, and redacted expanded text behavior.
- Modify: `src/DailyTracking.tsx`
  - Accept shared `sourceMode` where practical while keeping sample/live fallback.
- Modify: `src/App.test.tsx`
  - Update expectations for dashboard-first UI and toggle interactions.
- Modify: `src/styles.css`
  - Add restrained Toggl-inspired visual system: compact segmented controls, timeline rows, metric strip, density modifiers.

## Task 1: UI Model Red Tests

**Files:**

- Create: `src/lib/uiModel.test.ts`

- [ ] **Step 1: Write failing dashboard summary tests**

Add tests with explicit active and lifecycle events:

```ts
import { describe, expect, it } from "vitest";
import {
  buildDashboardSummary,
  buildHourlyTimelineItems,
  filterTimelineEvents,
  summarizeInputInsights
} from "./uiModel";
import type { TextSegment, TimeEvent } from "../types";

const events: TimeEvent[] = [
  {
    id: "a1",
    app: "Code.exe",
    title: "main.ts",
    kind: "active_window",
    startedAt: "2026-05-24T01:00:00.000Z",
    endedAt: "2026-05-24T01:30:00.000Z"
  },
  {
    id: "l1",
    app: "System",
    title: "Locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-05-24T01:30:00.000Z",
    endedAt: "2026-05-24T01:45:00.000Z",
    durationSeconds: 900
  },
  {
    id: "a2",
    app: "Browser.exe",
    title: "docs",
    kind: "active_window",
    startedAt: "2026-05-24T01:45:00.000Z",
    endedAt: "2026-05-24T01:55:00.000Z"
  }
];

describe("buildDashboardSummary", () => {
  it("separates active and lifecycle time and counts focus blocks", () => {
    const summary = buildDashboardSummary(events, []);
    expect(summary.activeSeconds).toBe(2400);
    expect(summary.lifecycleSeconds).toBe(900);
    expect(summary.focusBlockCount).toBe(1);
    expect(summary.contextSwitchCount).toBe(1);
    expect(summary.topApp?.app).toBe("Code.exe");
  });
});
```

- [ ] **Step 2: Write failing timeline and input tests**

Extend `src/lib/uiModel.test.ts`:

```ts
describe("filterTimelineEvents", () => {
  it("hides lifecycle rows when the lifecycle layer is disabled", () => {
    const visible = filterTimelineEvents(events, {
      windows: true,
      lifecycle: false,
      input: true,
      screenshots: true
    });
    expect(visible.map((event) => event.id)).toEqual(["a1", "a2"]);
  });
});

describe("buildHourlyTimelineItems", () => {
  it("aggregates active and lifecycle seconds by local hour", () => {
    const items = buildHourlyTimelineItems(events);
    expect(items).toHaveLength(1);
    expect(items[0]).toMatchObject({
      id: "hour-2026-05-24T01",
      app: "1 hour bucket",
      activeSeconds: 2400,
      lifecycleSeconds: 900
    });
  });
});

describe("summarizeInputInsights", () => {
  it("computes correction ratio and burst count safely", () => {
    const segments: TextSegment[] = [
      {
        id: "s1",
        startedAt: "2026-05-24T01:00:00.000Z",
        endedAt: "2026-05-24T01:04:00.000Z",
        textContent: "nihao",
        keyCount: 30,
        backspaceCount: 6,
        deleteCount: 0,
        foregroundHwnd: 1,
        foregroundPid: 1,
        processName: "Code.exe",
        windowTitle: "main.ts"
      }
    ];
    expect(summarizeInputInsights(segments)).toMatchObject({
      totalKeys: 30,
      correctionCount: 6,
      correctionRatio: 0.2,
      burstCount: 1,
      activeAppCount: 1
    });
  });
});
```

- [ ] **Step 3: Run red tests**

Run:

```powershell
npm test -- --run src/lib/uiModel.test.ts
```

Expected: fails because `src/lib/uiModel.ts` does not exist.

## Task 2: UI Model Implementation

**Files:**

- Create: `src/lib/uiModel.ts`
- Modify: `src/lib/uiModel.test.ts`

- [ ] **Step 1: Implement shared types and helpers**

Create `src/lib/uiModel.ts` with these exported types:

```ts
import type { TextSegment, TimeEvent } from "../types";

export type UiSourceMode = "sample" | "live";
export type PrivacyMode = "redacted" | "raw";
export type DensityMode = "comfortable" | "compact";
export type TimelineGranularity = "event" | "hour";
export type LayerKey = "windows" | "lifecycle" | "input" | "screenshots";

export type LayerVisibility = Record<LayerKey, boolean>;
```

Add pure functions that:

- Treat missing `event.kind` as `active_window`.
- Use explicit `durationSeconds` when present, otherwise compute timestamps.
- Exclude lifecycle events from active seconds.
- Count a focus block when an active event is at least 1500 seconds.
- Count context switches only across active window app transitions.
- Count input bursts when `keyCount >= 20` or segment duration is at least 120 seconds.

- [ ] **Step 2: Run model tests green**

Run:

```powershell
npm test -- --run src/lib/uiModel.test.ts
```

Expected: all `uiModel` tests pass.

- [ ] **Step 3: Commit model layer**

Run:

```powershell
git add src/lib/uiModel.ts src/lib/uiModel.test.ts
git commit -m "feat(ui): add timeline and insight model helpers"
```

## Task 3: Dashboard And App Shell Tests

**Files:**

- Modify: `src/App.test.tsx`
- Create: `src/Dashboard.tsx`
- Create: `src/TimelineView.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Update App tests before implementation**

Replace the old statistics-first assertion with dashboard-first expectations:

```ts
it("renders dashboard as the default view", async () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: /dashboard/i })).toBeInTheDocument();
  expect(screen.getByText(/active time/i)).toBeInTheDocument();
});

it("exposes Toggl-style source and privacy toggles", () => {
  render(<App />);
  expect(screen.getByRole("button", { name: /^sample$/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /^live$/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /^redacted$/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /^raw$/i })).toBeInTheDocument();
});

it("opens the timeline view from the tab bar", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: /timeline/i }));
  expect(screen.getByRole("heading", { name: /timeline/i })).toBeInTheDocument();
});
```

Also import `userEvent` from `@testing-library/user-event`. If the dependency is absent, prefer using `fireEvent` from Testing Library instead of adding a package.

- [ ] **Step 2: Run red App tests**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: fails because dashboard, timeline tab, and toggles do not exist yet.

## Task 4: Dashboard And Timeline Implementation

**Files:**

- Create: `src/Dashboard.tsx`
- Create: `src/TimelineView.tsx`
- Modify: `src/App.tsx`
- Modify: `src/data/feature1Sample.ts`
- Modify: `src/styles.css`

- [ ] **Step 1: Add sample lifecycle evidence**

Extend `feature1SampleEvents` with at least one lifecycle interval so timeline states are visible in sample mode:

```ts
{
  id: "feature1-lifecycle-1",
  app: "System",
  title: "Windows locked",
  kind: "lifecycle",
  status: "windows_lock",
  startedAt: "2026-05-23T10:27:00.000Z",
  endedAt: "2026-05-23T10:39:00.000Z",
  durationSeconds: 720
}
```

- [ ] **Step 2: Implement `Dashboard.tsx`**

`Dashboard` props:

```ts
type DashboardProps = {
  events: TimeEvent[];
  segments: TextSegment[];
  layers: LayerVisibility;
  densityMode: DensityMode;
  privacyMode: PrivacyMode;
};
```

It should render:

- Heading `Dashboard`.
- Metric cards for `Active Time`, `Lifecycle`, `Focus Blocks`, `Context Switches`, and `Correction Ratio`.
- Recent timeline preview from `buildTimelineItems(events).slice(-5)`.

- [ ] **Step 3: Implement `TimelineView.tsx`**

`TimelineView` props:

```ts
type TimelineViewProps = {
  events: TimeEvent[];
  layers: LayerVisibility;
  densityMode: DensityMode;
  granularity: TimelineGranularity;
};
```

It should render:

- Heading `Timeline`.
- Event rows when `granularity === "event"`.
- Hour bucket rows when `granularity === "hour"`.
- Empty state when all relevant layers are hidden.

- [ ] **Step 4: Modify `App.tsx`**

Add state:

```ts
const [viewMode, setViewMode] = useState<ViewMode>("dashboard");
const [sourceMode, setSourceMode] = useState<UiSourceMode>("sample");
const [privacyMode, setPrivacyMode] = useState<PrivacyMode>("redacted");
const [densityMode, setDensityMode] = useState<DensityMode>("comfortable");
const [granularity, setGranularity] = useState<TimelineGranularity>("event");
const [layers, setLayers] = useState<LayerVisibility>({
  windows: true,
  lifecycle: true,
  input: true,
  screenshots: true
});
```

Add segmented controls in the header, add `Dashboard` and `TimelineView`, and pass `privacyMode` to `InputActivity`.

- [ ] **Step 5: Run App tests green**

Run:

```powershell
npm test -- --run src/App.test.tsx src/lib/uiModel.test.ts
```

Expected: all selected tests pass.

- [ ] **Step 6: Commit dashboard and timeline**

Run:

```powershell
git add src/App.tsx src/App.test.tsx src/Dashboard.tsx src/TimelineView.tsx src/data/feature1Sample.ts src/styles.css
git commit -m "feat(ui): add toggl-style dashboard and timeline"
```

## Task 5: Input Activity Privacy And Insight Tests

**Files:**

- Modify: `src/InputActivity.tsx`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Write failing redaction UI test**

Add an App-level test that opens Input Activity, expands a sample segment, and verifies raw sample text is hidden in redacted mode:

```ts
it("keeps input segment text hidden while privacy mode is redacted", async () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: /input activity/i }));
  fireEvent.click(screen.getAllByRole("button", { expanded: false })[0]);
  expect(screen.getByText(/Raw text hidden/i)).toBeInTheDocument();
});
```

Use robust queries based on the final accessible row labels if needed.

- [ ] **Step 2: Run red test**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: fails because `InputActivity` still always renders raw expanded segment text.

## Task 6: Input Activity Implementation

**Files:**

- Modify: `src/InputActivity.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Add props and insight summary**

Change `InputActivity` to:

```ts
type InputActivityProps = {
  privacyMode?: PrivacyMode;
};

export function InputActivity({ privacyMode = "redacted" }: InputActivityProps) {
```

Use `summarizeInputInsights(segments)` and render insight metrics above the app bars.

- [ ] **Step 2: Add app filter**

Use `listSegmentApps(segments)` to render a select or segmented filter with `All apps` and per-app options. Filter `segments` before rendering table rows.

- [ ] **Step 3: Respect privacy mode**

When a segment is expanded:

- If `privacyMode === "raw"`, render `seg.textContent`.
- If `privacyMode === "redacted"`, render `Raw text hidden in redacted mode` and keep counts visible.

- [ ] **Step 4: Run tests green**

Run:

```powershell
npm test -- --run src/App.test.tsx src/lib/uiModel.test.ts
```

Expected: all selected tests pass.

- [ ] **Step 5: Commit input activity improvements**

Run:

```powershell
git add src/InputActivity.tsx src/App.tsx src/App.test.tsx src/styles.css
git commit -m "feat(ui): add input activity privacy and insights"
```

## Task 7: Full Verification And Push

**Files:**

- All modified frontend/docs files.

- [ ] **Step 1: Run frontend tests**

Run:

```powershell
npm test -- --run
```

Expected: Vitest passes.

- [ ] **Step 2: Run frontend build**

Run:

```powershell
npm run build
```

Expected: TypeScript and Vite build pass.

- [ ] **Step 3: Check formatting-sensitive diff**

Run:

```powershell
git diff --check
git status --short --branch --untracked-files=all
```

Expected: no whitespace errors; only unrelated `.claude` and `.playwright-mcp` untracked files may remain.

- [ ] **Step 4: Push branch**

Run:

```powershell
git push
```

Expected: branch `codex/0524-toggl-ui-toggles` is updated on `origin`.

## Self-Review Notes

- The plan covers every PRD in-scope item without adding backend work.
- API v2, Notion sync, persistent settings, and manual time entry remain out of scope.
- The first implementation task is pure model TDD, so UI behavior is grounded in tested derivation.
- Privacy is explicit at the component boundary and can later map to API v2 redaction policy.
