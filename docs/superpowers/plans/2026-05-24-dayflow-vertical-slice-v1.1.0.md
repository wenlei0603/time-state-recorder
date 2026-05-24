# Dayflow Vertical Slice v1.1.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Dayflow-inspired Today Flow Board on top of the `v1.1.0` release while adding narrow Windows collector observability for foreground capture and screenshot skip stability.

**Architecture:** Keep the current Rust/Axum/SQLite collector and React/Vite UI. Add small, backward-compatible health/summary fields in Rust, then build a frontend flow model and a new Today Flow Board view that composes time events, input summary, screenshot summary, and collector health without exposing raw evidence in redacted mode.

**Tech Stack:** Rust, Axum, rusqlite, chrono, React, TypeScript, Vitest, Testing Library, CSS.

---

## File Structure

- Modify `collector/src/models.rs`: add optional health details and screenshot skipped reason counts.
- Modify `collector/src/storage.rs`: add screenshot skip reason aggregation from non-ok screenshot metadata.
- Modify `collector/src/api.rs`: update health defaults, record foreground capture failures as lifecycle facts, record screenshot skip metadata, and expose last skip/error details.
- Modify `collector/tests/storage_tests.rs`: test screenshot skipped reason summary.
- Modify `collector/tests/api_tests.rs`: test health exposes collector stability fields.
- Modify `src/types.ts`: add privacy mode, screenshot skipped reason, health details, and flow board types.
- Modify `src/lib/health.ts`: parse optional subsystem detail fields.
- Modify `src/lib/screenshots.ts`: parse optional screenshot skipped reason counts.
- Create `src/lib/flowModel.ts`: deterministic UI model for summary, buckets, evidence drawer data, and privacy gating.
- Create `src/lib/flowModel.test.ts`: test flow buckets, summary, confidence, and redaction.
- Create `src/TodayFlowBoard.tsx`: main Dayflow-style view.
- Modify `src/App.tsx`: make Today Flow Board the default view and wire source/privacy/layer state.
- Modify `src/App.test.tsx`: cover default board rendering and stale raw response privacy.
- Modify `src/styles.css`: add board, lane, evidence drawer, privacy/layer controls, responsive layout styles.
- Modify `docs/dayflow-engineering-knowledge/00-index.md`: record the merged Dayflow/Windows vertical-slice architecture and residual risks.

## Task 1: Collector Observability

**Files:**
- Modify: `collector/src/models.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/src/api.rs`
- Test: `collector/tests/storage_tests.rs`
- Test: `collector/tests/api_tests.rs`

- [ ] **Step 1: Write a failing storage test for screenshot skipped reasons**

Append this test to `collector/tests/storage_tests.rs` before the local `ts()` helper:

```rust
#[test]
fn screenshot_summary_counts_skipped_reasons() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (minute, status) in [("00", "ok"), ("01", "idle"), ("02", "blocked"), ("03", "capture_unavailable")] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(&format!("2026-05-24T09:{minute}:00Z")),
                    file_path: format!("2026-05-24/09-{minute}.jpg"),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: status.into(),
                },
            )
            .unwrap();
    }

    let summary = store.get_screenshot_summary("2026-05-24").unwrap();

    assert_eq!(summary.total_screenshots, 4);
    assert_eq!(summary.skipped_reasons.len(), 3);
    assert_eq!(summary.skipped_reasons[0].reason, "blocked");
    assert_eq!(summary.skipped_reasons[0].count, 1);
    assert_eq!(summary.skipped_reasons[1].reason, "capture_unavailable");
    assert_eq!(summary.skipped_reasons[2].reason, "idle");
}
```

- [ ] **Step 2: Run the storage test and verify RED**

Run:

```powershell
cargo test -p tsr-collector screenshot_summary_counts_skipped_reasons -- --nocapture
```

Expected: fail with a compile error because `ScreenshotSummary` has no `skipped_reasons` field.

- [ ] **Step 3: Implement screenshot skipped reason summary**

In `collector/src/models.rs`, add this struct after `ScreenshotSummary` and add the field to `ScreenshotSummary`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSummary {
    pub date: String,
    pub total_screenshots: usize,
    pub hours_covered: usize,
    pub top_apps: Vec<AppScreenshotCount>,
    pub skipped_reasons: Vec<ScreenshotSkippedReasonCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSkippedReasonCount {
    pub reason: String,
    pub count: usize,
}
```

Update the `use crate::models::{...}` list in `collector/src/storage.rs` to include `ScreenshotSkippedReasonCount`.

In `Store::get_screenshot_summary`, after building `top_apps`, add:

```rust
let mut skipped_statement = self.conn.prepare(
    r#"
    SELECT capture_status, COUNT(*) AS count
    FROM screenshot_thumbnails
    WHERE captured_at LIKE ?1
      AND capture_status <> 'ok'
    GROUP BY capture_status
    ORDER BY capture_status ASC
    "#,
)?;
let skipped_reasons = skipped_statement
    .query_map(params![&pattern], |row| {
        Ok(ScreenshotSkippedReasonCount {
            reason: row.get(0)?,
            count: row.get(1)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
```

Then return:

```rust
Ok(ScreenshotSummary {
    date: date.to_string(),
    total_screenshots: total,
    hours_covered,
    top_apps,
    skipped_reasons,
})
```

- [ ] **Step 4: Run the storage test and verify GREEN**

Run:

```powershell
cargo test -p tsr-collector screenshot_summary_counts_skipped_reasons -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Write a failing API health test for stability details**

Append this test to `collector/tests/api_tests.rs` before `fn insert(...)`:

```rust
#[tokio::test]
async fn serves_health_with_collector_stability_details() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["windowCollector"]["mode"], "polling");
    assert_eq!(body["windowCollector"]["lastCaptureStatus"], serde_json::Value::Null);
    assert_eq!(body["screenshotCollector"]["lastSkipReason"], serde_json::Value::Null);

    server.abort();
}
```

- [ ] **Step 6: Run the API test and verify RED**

Run:

```powershell
cargo test -p tsr-collector serves_health_with_collector_stability_details -- --nocapture
```

Expected: fail because subsystem health JSON has no `mode`, `lastCaptureStatus`, or `lastSkipReason`.

- [ ] **Step 7: Implement health details and skip fact recording**

In `collector/src/models.rs`, replace `SubsystemHealth` with:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemHealth {
    pub status: String,
    pub last_event_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub mode: Option<String>,
    pub last_capture_status: Option<String>,
    pub last_skip_reason: Option<String>,
}
```

In `collector/src/api.rs`, update each `SubsystemHealth` initializer:

```rust
window_collector: SubsystemHealth {
    status: "not_started".into(),
    last_event_at: None,
    error_count: 0,
    last_error: None,
    mode: Some("polling".into()),
    last_capture_status: None,
    last_skip_reason: None,
},
input_collector: SubsystemHealth {
    status: "not_started".into(),
    last_event_at: None,
    error_count: 0,
    last_error: None,
    mode: Some("raw_input".into()),
    last_capture_status: None,
    last_skip_reason: None,
},
screenshot_collector: SubsystemHealth {
    status: "not_started".into(),
    last_event_at: None,
    error_count: 0,
    last_error: None,
    mode: Some("interval_thumbnail".into()),
    last_capture_status: None,
    last_skip_reason: None,
},
```

In `spawn_collector_loop`, when `sample_foreground_window()` succeeds, set `last_capture_status` to `Some(snapshot.capture_status.as_str().into())` after a successful insert. When it fails, create a `capture_unavailable` lifecycle event and update health:

```rust
Err(err) => {
    eprintln!("window sample failed: {err:#}");
    if let Ok(mut store) = state.store.lock() {
        let _ = store.insert_lifecycle_event(
            &session_id,
            Utc::now(),
            LifecycleType::CaptureUnavailable,
            Some("window_sample_failed"),
            serde_json::json!({ "error": format!("{err:#}") }),
        );
    }
    if let Ok(mut h) = state.health.lock() {
        h.window_collector.error_count += 1;
        h.window_collector.last_error = Some(format!("{err:#}"));
        h.window_collector.last_capture_status = Some("capture_unavailable".into());
    }
}
```

Add a small helper in `collector/src/api.rs` below `spawn_screenshot_loop`:

```rust
fn record_screenshot_skip(
    state: &AppState,
    session_id: &str,
    reason: &str,
    snapshot: Option<&crate::models::WindowSnapshot>,
) {
    let now = Utc::now();
    if let Ok(mut store) = state.store.lock() {
        let _ = store.insert_screenshot(
            session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: now,
                file_path: String::new(),
                width: 0,
                height: 0,
                process_name: snapshot.map(|s| s.process_name.clone()),
                window_title: snapshot.and_then(|s| s.window_title.clone()),
                capture_status: reason.to_string(),
            },
        );
    }
    if let Ok(mut h) = state.health.lock() {
        h.screenshot_collector.last_skip_reason = Some(reason.to_string());
        h.screenshot_collector.last_error = None;
        h.screenshot_collector.last_event_at = Some(now);
    }
}
```

Use this helper in `spawn_screenshot_loop` before each `continue` for idle, foreground sample error, blocker hit, `capture_thumbnail` returning `None`, directory creation failure, write failure, and metadata write failure. Use reasons `idle`, `capture_unavailable`, `blocked`, `capture_failed`, `write_failed`, and `metadata_write_failed`.

- [ ] **Step 8: Run collector tests and format**

Run:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
```

Expected: all collector tests pass.

## Task 2: Flow Model And Privacy Gate

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/health.ts`
- Modify: `src/lib/screenshots.ts`
- Create: `src/lib/flowModel.ts`
- Test: `src/lib/flowModel.test.ts`
- Test: `src/lib/health.test.ts`

- [ ] **Step 1: Write failing flow model tests**

Create `src/lib/flowModel.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { buildTodayFlowModel } from "./flowModel";
import type { CollectorHealth, InputSummary, ScreenshotSummary, TimeEvent } from "../types";

const events: TimeEvent[] = [
  {
    id: "raw-1",
    app: "Code.exe",
    title: "main.rs",
    kind: "active_window",
    sessionId: "s1",
    startedAt: "2026-05-24T09:00:00Z",
    endedAt: "2026-05-24T09:20:00Z",
    durationSeconds: 1200
  },
  {
    id: "lifecycle-2",
    app: "System",
    title: "Collector gap",
    kind: "lifecycle",
    status: "collector_gap",
    sessionId: "s1",
    startedAt: "2026-05-24T09:20:00Z",
    endedAt: "2026-05-24T09:30:00Z",
    durationSeconds: 600
  },
  {
    id: "raw-3",
    app: "Browser.exe",
    title: "Docs",
    kind: "active_window",
    sessionId: "s2",
    startedAt: "2026-05-24T09:30:00Z",
    endedAt: "2026-05-24T10:00:00Z",
    durationSeconds: 1800
  }
];

const screenshotSummary: ScreenshotSummary = {
  date: "2026-05-24",
  totalScreenshots: 12,
  hoursCovered: 2,
  topApps: [{ processName: "Code.exe", count: 8 }],
  skippedReasons: [{ reason: "idle", count: 3 }]
};

const inputSummary: InputSummary = {
  date: "2026-05-24",
  totalEvents: 50,
  keydownCount: 25,
  keyupCount: 25,
  segmentCount: 3,
  totalChars: 120,
  topApps: [{ processName: "Code.exe", charCount: 80 }]
};

const health: CollectorHealth = {
  status: "ok",
  startedAt: "2026-05-24T08:00:00Z",
  uptimeSeconds: 3600,
  version: "0.1.0",
  windowCollector: { status: "running", errorCount: 0, mode: "polling" },
  inputCollector: { status: "running", errorCount: 0, mode: "raw_input" },
  screenshotCollector: {
    status: "running",
    errorCount: 0,
    mode: "interval_thumbnail",
    lastSkipReason: "idle"
  },
  dbStats: {
    windowEvents: 3,
    lifecycleEvents: 1,
    inputEvents: 50,
    textSegments: 3,
    screenshots: 12,
    blockerHits: 0
  }
};

describe("buildTodayFlowModel", () => {
  it("summarizes active, uncertain, evidence, and input totals", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      health,
      privacyMode: "redacted"
    });

    expect(model.summary.activeSeconds).toBe(3000);
    expect(model.summary.uncertainSeconds).toBe(600);
    expect(model.summary.screenshotCount).toBe(12);
    expect(model.summary.inputChars).toBe(120);
    expect(model.buckets.map((bucket) => bucket.confidence)).toEqual([
      "high",
      "uncertain",
      "high"
    ]);
  });

  it("redacts sensitive evidence when privacy mode is redacted", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      health,
      privacyMode: "redacted"
    });

    expect(model.evidence[0].title).toBe("Hidden in redacted mode");
    expect(model.evidence[0].screenshotVisible).toBe(false);
  });

  it("keeps raw evidence labels when privacy mode is raw", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      health,
      privacyMode: "raw"
    });

    expect(model.evidence[0].title).toBe("main.rs");
    expect(model.evidence[0].screenshotVisible).toBe(true);
  });
});
```

- [ ] **Step 2: Run flow model test and verify RED**

Run:

```powershell
npm test -- src/lib/flowModel.test.ts --run
```

Expected: fail because `src/lib/flowModel.ts` and new type fields do not exist.

- [ ] **Step 3: Add frontend types and parsers**

In `src/types.ts`, add:

```ts
export type PrivacyMode = "redacted" | "raw";
export type FlowConfidence = "high" | "partial" | "uncertain";

export type ScreenshotSkippedReasonCount = {
  reason: string;
  count: number;
};
```

Add `skippedReasons: ScreenshotSkippedReasonCount[];` to `ScreenshotSummary`.

Add optional fields to `SubsystemHealth`:

```ts
mode?: string;
lastCaptureStatus?: string;
lastSkipReason?: string;
```

Add these flow types:

```ts
export type FlowBucket = {
  id: string;
  app: string;
  title: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds: number;
  kind: "active_window" | "lifecycle";
  status?: string;
  confidence: FlowConfidence;
  share: number;
};

export type FlowEvidence = {
  id: string;
  app: string;
  title: string;
  timeRange: string;
  confidence: FlowConfidence;
  screenshotVisible: boolean;
  notes: string[];
};

export type TodayFlowModel = {
  summary: {
    activeSeconds: number;
    uncertainSeconds: number;
    screenshotCount: number;
    screenshotSkippedCount: number;
    inputChars: number;
  };
  buckets: FlowBucket[];
  evidence: FlowEvidence[];
};
```

In `src/lib/screenshots.ts`, parse `skippedReasons` with a fallback empty array.

In `src/lib/health.ts`, read `mode`, `lastCaptureStatus`, and `lastSkipReason` inside `readSubsystem`.

- [ ] **Step 4: Implement `src/lib/flowModel.ts`**

Create `src/lib/flowModel.ts`:

```ts
import type {
  CollectorHealth,
  FlowBucket,
  FlowConfidence,
  FlowEvidence,
  InputSummary,
  PrivacyMode,
  ScreenshotSummary,
  TimeEvent,
  TodayFlowModel
} from "../types";
import { toDurationSeconds } from "./statistics";

type BuildTodayFlowModelInput = {
  events: TimeEvent[];
  screenshotSummary?: ScreenshotSummary;
  inputSummary?: InputSummary;
  health?: CollectorHealth;
  privacyMode: PrivacyMode;
};

const UNCERTAIN_STATUSES = new Set([
  "collector_gap",
  "capture_unavailable",
  "session_stop",
  "power_suspend"
]);

export function buildTodayFlowModel({
  events,
  screenshotSummary,
  inputSummary,
  health,
  privacyMode
}: BuildTodayFlowModelInput): TodayFlowModel {
  const durations = events.map((event) => Math.max(0, toDurationSeconds(event)));
  const totalDuration = Math.max(1, durations.reduce((sum, value) => sum + value, 0));
  const buckets = events.map((event, index): FlowBucket => {
    const durationSeconds = durations[index];
    const confidence = confidenceForEvent(event);
    return {
      id: event.id,
      app: event.app,
      title: event.title,
      startedAt: event.startedAt,
      endedAt: event.endedAt,
      durationSeconds,
      kind: event.kind ?? "active_window",
      status: event.status,
      confidence,
      share: durationSeconds / totalDuration
    };
  });

  const activeSeconds = buckets
    .filter((bucket) => bucket.kind === "active_window")
    .reduce((sum, bucket) => sum + bucket.durationSeconds, 0);
  const uncertainSeconds = buckets
    .filter((bucket) => bucket.confidence === "uncertain")
    .reduce((sum, bucket) => sum + bucket.durationSeconds, 0);
  const screenshotSkippedCount =
    screenshotSummary?.skippedReasons.reduce((sum, row) => sum + row.count, 0) ?? 0;

  return {
    summary: {
      activeSeconds,
      uncertainSeconds,
      screenshotCount: screenshotSummary?.totalScreenshots ?? 0,
      screenshotSkippedCount,
      inputChars: inputSummary?.totalChars ?? 0
    },
    buckets,
    evidence: buckets.map((bucket) =>
      toEvidence(bucket, privacyMode, screenshotSummary, inputSummary, health)
    )
  };
}

function confidenceForEvent(event: TimeEvent): FlowConfidence {
  if ((event.kind ?? "active_window") === "lifecycle") {
    return event.status && UNCERTAIN_STATUSES.has(event.status) ? "uncertain" : "partial";
  }
  if (event.durationSeconds === undefined || event.durationSeconds <= 0) {
    return "partial";
  }
  return "high";
}

function toEvidence(
  bucket: FlowBucket,
  privacyMode: PrivacyMode,
  screenshotSummary: ScreenshotSummary | undefined,
  inputSummary: InputSummary | undefined,
  health: CollectorHealth | undefined
): FlowEvidence {
  const notes = [
    `${formatSeconds(bucket.durationSeconds)} captured`,
    `${screenshotSummary?.totalScreenshots ?? 0} screenshots today`,
    `${inputSummary?.totalChars ?? 0} input chars today`
  ];

  if (health?.windowCollector.mode) {
    notes.push(`window collector: ${health.windowCollector.mode}`);
  }
  if (health?.screenshotCollector.lastSkipReason) {
    notes.push(`last screenshot skip: ${health.screenshotCollector.lastSkipReason}`);
  }

  return {
    id: bucket.id,
    app: bucket.kind === "lifecycle" ? "System" : bucket.app,
    title: privacyMode === "raw" ? bucket.title : "Hidden in redacted mode",
    timeRange: `${formatTime(bucket.startedAt)} - ${bucket.endedAt ? formatTime(bucket.endedAt) : "open"}`,
    confidence: bucket.confidence,
    screenshotVisible: privacyMode === "raw",
    notes
  };
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatSeconds(value: number): string {
  const minutes = Math.floor(value / 60);
  const seconds = Math.round(value % 60);
  if (minutes <= 0) return `${seconds}s`;
  if (seconds === 0) return `${minutes}m`;
  return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}
```

- [ ] **Step 5: Run frontend unit tests for model/parsers**

Run:

```powershell
npm test -- src/lib/flowModel.test.ts src/lib/health.test.ts src/lib/api.test.ts --run
```

Expected: pass.

## Task 3: Today Flow Board UI

**Files:**
- Create: `src/TodayFlowBoard.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Write failing App tests for default board and privacy**

Append these tests to `src/App.test.tsx`:

```ts
it("uses Today Flow Board as the default first screen", async () => {
  render(<App />);

  expect(await screen.findByText("Today Flow Board")).toBeInTheDocument();
  expect(screen.getByText("Time flow")).toBeInTheDocument();
  expect(screen.getByText("Evidence drawer")).toBeInTheDocument();
});

it("does not render raw titles in redacted mode on the flow board", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      statusText: "OK",
      json: async () => ({
        events: [
          {
            id: "raw-sensitive",
            app: "SecretApp.exe",
            title: "Sensitive planning document",
            kind: "active_window",
            sessionId: "s1",
            startedAt: "2026-05-24T09:00:00Z",
            endedAt: "2026-05-24T09:10:00Z",
            durationSeconds: 600
          }
        ]
      })
    })
  );

  render(<App />);

  expect(await screen.findByText("Hidden in redacted mode")).toBeInTheDocument();
  expect(screen.queryByText("Sensitive planning document")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run App tests and verify RED**

Run:

```powershell
npm test -- src/App.test.tsx --run
```

Expected: fail because `TodayFlowBoard` is not implemented and `App` still defaults to Statistics.

- [ ] **Step 3: Implement `TodayFlowBoard`**

Create `src/TodayFlowBoard.tsx`:

```tsx
import { Activity, AlertTriangle, Camera, Keyboard, Shield, Timer } from "lucide-react";
import { useMemo, useState } from "react";
import { buildTodayFlowModel } from "./lib/flowModel";
import type {
  CollectorHealth,
  InputSummary,
  PrivacyMode,
  ScreenshotSummary,
  TimeEvent
} from "./types";

type TodayFlowBoardProps = {
  events: TimeEvent[];
  screenshotSummary?: ScreenshotSummary;
  inputSummary?: InputSummary;
  health?: CollectorHealth;
  privacyMode: PrivacyMode;
  sourceLabel: string;
};

export function TodayFlowBoard({
  events,
  screenshotSummary,
  inputSummary,
  health,
  privacyMode,
  sourceLabel
}: TodayFlowBoardProps) {
  const model = useMemo(
    () =>
      buildTodayFlowModel({
        events,
        screenshotSummary,
        inputSummary,
        health,
        privacyMode
      }),
    [events, screenshotSummary, inputSummary, health, privacyMode]
  );
  const [selectedId, setSelectedId] = useState<string | null>(model.buckets[0]?.id ?? null);
  const selected = model.evidence.find((item) => item.id === selectedId) ?? model.evidence[0];

  return (
    <section className="flowBoard" aria-label="Today Flow Board">
      <div className="flowBoardHeader">
        <div>
          <p className="eyebrow">Dayflow vertical slice / {sourceLabel}</p>
          <h2>Today Flow Board</h2>
        </div>
        <div className="privacyBadge">
          <Shield aria-hidden="true" size={16} />
          <span>{privacyMode === "raw" ? "Raw evidence enabled" : "Redacted evidence"}</span>
        </div>
      </div>

      <div className="flowBoardGrid">
        <aside className="flowSummary" aria-label="Today summary">
          <FlowMetric icon={<Timer aria-hidden="true" size={16} />} label="Active" value={formatSeconds(model.summary.activeSeconds)} />
          <FlowMetric icon={<AlertTriangle aria-hidden="true" size={16} />} label="Uncertain" value={formatSeconds(model.summary.uncertainSeconds)} />
          <FlowMetric icon={<Camera aria-hidden="true" size={16} />} label="Evidence" value={`${model.summary.screenshotCount} shots`} />
          <FlowMetric icon={<Keyboard aria-hidden="true" size={16} />} label="Input" value={`${model.summary.inputChars} chars`} />
        </aside>

        <div className="flowLanePanel">
          <div className="panelHeader">
            <Activity aria-hidden="true" size={20} />
            <h2>Time flow</h2>
          </div>
          <div className="flowLane" role="list" aria-label="Daily time flow">
            {model.buckets.map((bucket) => (
              <button
                type="button"
                key={bucket.id}
                className={`flowBucket ${bucket.confidence} ${selectedId === bucket.id ? "selected" : ""}`}
                style={{ "--bucket-share": bucket.share } as React.CSSProperties}
                onClick={() => setSelectedId(bucket.id)}
              >
                <span className="flowBucketTime">{formatTime(bucket.startedAt)}</span>
                <span className="flowBucketMain">{bucket.kind === "lifecycle" ? bucket.status ?? "lifecycle" : bucket.app}</span>
                <span className="flowBucketDuration">{formatSeconds(bucket.durationSeconds)}</span>
              </button>
            ))}
          </div>
          {model.summary.screenshotSkippedCount > 0 && (
            <p className="flowHint">
              {model.summary.screenshotSkippedCount} screenshot attempts were skipped or blocked today.
            </p>
          )}
        </div>

        <aside className="evidenceDrawer" aria-label="Evidence drawer">
          <div className="panelHeader">
            <Camera aria-hidden="true" size={20} />
            <h2>Evidence drawer</h2>
          </div>
          {selected ? (
            <>
              <div className="evidencePrimary">
                <span className={`confidencePill ${selected.confidence}`}>{selected.confidence}</span>
                <strong>{selected.app}</strong>
                <span>{selected.timeRange}</span>
              </div>
              <div className="evidencePreview">
                {selected.screenshotVisible ? "Screenshot evidence available in raw mode" : "Screenshot preview hidden in redacted mode"}
              </div>
              <dl className="evidenceFacts">
                <div>
                  <dt>Title</dt>
                  <dd>{selected.title}</dd>
                </div>
                {selected.notes.map((note) => (
                  <div key={note}>
                    <dt>Fact</dt>
                    <dd>{note}</dd>
                  </div>
                ))}
              </dl>
            </>
          ) : (
            <p className="flowHint">No time events are available yet.</p>
          )}
        </aside>
      </div>
    </section>
  );
}

function FlowMetric({
  icon,
  label,
  value
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <article className="flowMetric">
      <span>{icon}{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function formatSeconds(value: number): string {
  const rounded = Math.round(value);
  const minutes = Math.floor(rounded / 60);
  const seconds = rounded % 60;
  if (minutes === 0) return `${seconds}s`;
  if (seconds === 0) return `${minutes}m`;
  return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
```

- [ ] **Step 4: Wire App source/privacy state**

Modify `src/App.tsx`:

- Import `Shield`, `TodayFlowBoard`, `feature2SampleSummary`, `feature3SampleSummary`, `fetchCollectorHealth`, `fetchInputSummary`, `fetchScreenshotSummary`, and the new types.
- Change `ViewMode` to include `"today"` and default state to `"today"`.
- Add `privacyMode` state defaulting to `"redacted"`.
- Add `inputSummary`, `screenshotSummary`, and `health` state initialized from sample data or undefined.
- Add a topbar segmented button to toggle redacted/raw.
- In `refreshCollector`, fetch `fetchTimeEvents()`, `fetchInputSummary(today)`, `fetchScreenshotSummary(today)`, and `fetchCollectorHealth()` with `Promise.allSettled`, keeping failed optional summaries undefined.
- Render `<TodayFlowBoard ... />` when `viewMode === "today"`.

Do not pass text segments or screenshot rows into `TodayFlowBoard`; it only receives summaries and health.

- [ ] **Step 5: Add CSS for the board**

Append scoped styles to `src/styles.css`:

```css
.flowBoard {
  display: grid;
  gap: 16px;
}

.flowBoardHeader,
.flowBoardGrid,
.privacyBadge,
.flowMetric span,
.flowBucket,
.evidencePrimary,
.evidenceFacts div {
  display: flex;
}

.flowBoardHeader {
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.flowBoardHeader h2 {
  font-size: 1.45rem;
}

.privacyBadge {
  align-items: center;
  gap: 8px;
  border: 1px solid #d9dfd8;
  border-radius: 999px;
  background: #ffffff;
  color: #5f6d65;
  font-size: 0.82rem;
  font-weight: 700;
  padding: 8px 12px;
}

.flowBoardGrid {
  align-items: stretch;
  gap: 16px;
}

.flowSummary {
  display: grid;
  width: 210px;
  align-content: start;
  gap: 10px;
}

.flowMetric,
.flowLanePanel,
.evidenceDrawer {
  border: 1px solid #d9dfd8;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 12px 28px rgba(23, 33, 27, 0.06);
}

.flowMetric {
  min-height: 86px;
  padding: 14px;
}

.flowMetric span {
  align-items: center;
  gap: 6px;
  color: #5f6d65;
  font-size: 0.76rem;
  font-weight: 800;
  text-transform: uppercase;
}

.flowMetric strong {
  display: block;
  margin-top: 10px;
  font-size: 1.2rem;
}

.flowLanePanel {
  flex: 1;
  min-width: 0;
  padding: 18px;
}

.flowLane {
  display: grid;
  gap: 8px;
}

.flowBucket {
  width: 100%;
  min-height: 44px;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-color: #d9dfd8;
  background: linear-gradient(90deg, rgba(53, 107, 99, 0.16), rgba(53, 107, 99, 0.04));
  padding: 0 12px;
}

.flowBucket.partial {
  background: #fef3cf;
}

.flowBucket.uncertain {
  background: #f5dfdd;
}

.flowBucket.selected {
  border-color: #356b63;
  box-shadow: 0 0 0 2px rgba(53, 107, 99, 0.15);
}

.flowBucketTime,
.flowBucketDuration {
  color: #5f6d65;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.78rem;
}

.flowBucketMain {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-align: left;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.flowHint {
  margin: 12px 0 0;
  color: #5f6d65;
  font-size: 0.84rem;
}

.evidenceDrawer {
  width: 300px;
  padding: 18px;
}

.evidencePrimary {
  align-items: flex-start;
  flex-direction: column;
  gap: 5px;
  margin-bottom: 12px;
}

.confidencePill {
  border-radius: 999px;
  background: #d9ecdf;
  color: #1f6b3b;
  font-size: 0.72rem;
  font-weight: 800;
  padding: 3px 8px;
  text-transform: uppercase;
}

.confidencePill.partial {
  background: #fef3cf;
  color: #8a6d10;
}

.confidencePill.uncertain {
  background: #f5dfdd;
  color: #8a312e;
}

.evidencePreview {
  display: flex;
  min-height: 96px;
  align-items: center;
  justify-content: center;
  border: 1px dashed #c6d2c9;
  border-radius: 8px;
  color: #5f6d65;
  font-size: 0.82rem;
  text-align: center;
  padding: 12px;
}

.evidenceFacts {
  display: grid;
  gap: 8px;
  margin: 14px 0 0;
}

.evidenceFacts div {
  justify-content: space-between;
  gap: 12px;
  border-top: 1px solid #e2e6e1;
  padding-top: 8px;
}

.evidenceFacts dt {
  color: #5f6d65;
  font-size: 0.72rem;
  font-weight: 800;
  text-transform: uppercase;
}

.evidenceFacts dd {
  margin: 0;
  max-width: 170px;
  color: #29352e;
  font-size: 0.82rem;
  text-align: right;
}

@media (max-width: 980px) {
  .flowBoardGrid {
    display: grid;
  }

  .flowSummary,
  .evidenceDrawer {
    width: auto;
  }
}
```

- [ ] **Step 6: Run App tests and frontend build**

Run:

```powershell
npm test -- src/App.test.tsx src/lib/flowModel.test.ts --run
npm run build
```

Expected: all tests pass and Vite build succeeds.

## Task 4: Engineering Knowledge And Final Verification

**Files:**
- Modify: `docs/dayflow-engineering-knowledge/00-index.md`
- Modify: `docs/dayflow-engineering-knowledge/02-information-flow.md`
- Modify: `docs/dayflow-engineering-knowledge/04-reproduction-boundaries.md`

- [ ] **Step 1: Update Obsidian knowledge base**

Add a section to `docs/dayflow-engineering-knowledge/00-index.md`:

```markdown
## 2026-05-24 v1.1.0 Vertical Slice

- 基线：`time-state-recorder` release `v1.1.0` / commit `fd2b25b`。
- 目标：把 Dayflow 的“时间流 + 证据抽屉”复现为 Windows-first Today Flow Board。
- 下层：继续使用 Rust/Axum/SQLite collector，增强 foreground polling 和 screenshot skip 的可观测性。
- 上层：React WebUI 默认进入 Today Flow Board，组合 time events、input summary、screenshot summary、collector health。
- 隐私：redacted 默认隐藏窗口标题细节、截图预览和 raw text；raw 模式才展示敏感证据。
- 残余风险：polling 仍可能漏掉极短窗口切换；完整 Windows event hook、LLM 日报和 Notion 导出留到后续版本。
```

Add matching data-flow notes to `02-information-flow.md` and reproduction boundary notes to `04-reproduction-boundaries.md`.

- [ ] **Step 2: Run full verification**

Run:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
npm test -- --run
npm run build
git status --short
```

Expected: all tests/builds pass. `git status --short` should show only intentional modified/new files plus existing unrelated untracked `.claude/`, `.playwright-mcp/`, `.superpowers/`, and `docs/dayflow-engineering-knowledge/` if they remain untracked.

- [ ] **Step 3: Commit implementation**

If verification passes, stage only the files touched by this plan:

```powershell
git add -- collector/src/models.rs collector/src/storage.rs collector/src/api.rs collector/tests/storage_tests.rs collector/tests/api_tests.rs src/types.ts src/lib/health.ts src/lib/screenshots.ts src/lib/flowModel.ts src/lib/flowModel.test.ts src/TodayFlowBoard.tsx src/App.tsx src/App.test.tsx src/styles.css docs/dayflow-engineering-knowledge/00-index.md docs/dayflow-engineering-knowledge/02-information-flow.md docs/dayflow-engineering-knowledge/04-reproduction-boundaries.md
git commit -m "feat: add dayflow today flow board"
```

Expected: one implementation commit on top of the design commit.

## Self-Review

- Spec coverage: collector observability, API/health details, Today Flow Board, privacy gate, tests, and Obsidian engineering knowledge are covered.
- Placeholder scan: clean; no incomplete markers or unspecified test steps remain.
- Type consistency: Rust field names use snake_case and serialize to camelCase; TypeScript parser fields use camelCase.
- Scope control: full Windows hook collector, API v2, LLM summaries, Notion export, and external data joins remain out of scope.
