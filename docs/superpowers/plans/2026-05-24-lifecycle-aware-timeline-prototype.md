# Lifecycle-Aware Timeline Prototype Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a prototype core that records lifecycle facts and prevents Windows lock, suspend, graceful stop, and abnormal collector gaps from being counted as normal active window time.

**Architecture:** Keep raw collector facts append-only in SQLite, add typed lifecycle models beside existing window events, then derive lifecycle-aware `TimeEvent` rows at the API boundary. This is intentionally smaller than the full migration/query-v2 design, but it creates stable seams for later analyzer, Notion, and attention-metric work.

**Tech Stack:** Rust 2024, Axum, Tokio, SQLite via rusqlite, chrono/serde, React 19, TypeScript, Vitest.

---

## File Structure

- Modify `collector/src/models.rs`: add `LifecycleType`, `LifecycleEvent`, `TimeEventKind`, optional `TimeEvent.status`, and `TimeEvent.session_id`.
- Modify `collector/src/storage.rs`: add lifecycle table initialization, lifecycle event insert/list helpers, session close helpers, and lifecycle db stats.
- Modify `collector/src/interval.rs`: keep the existing public `build_time_events` wrapper and add `build_time_events_with_lifecycle`.
- Modify `collector/src/api.rs`: expose `/api/lifecycle-events`, include lifecycle rows when building `/api/time-events`, and update health db stats.
- Modify `collector/src/main.rs`: close stale sessions before new session creation and close the record session on normal completion.
- Modify `collector/src/lib.rs`: no new module is needed for this prototype; lifecycle models live in `models.rs`.
- Modify `collector/tests/storage_tests.rs`: cover lifecycle persistence and stale session closure.
- Modify `collector/tests/interval_tests.rs`: cover lock/suspend/session boundaries.
- Modify `collector/tests/api_tests.rs`: cover lifecycle endpoint and lifecycle-aware time events.
- Modify `src/types.ts`: add optional lifecycle-aware `TimeEvent` fields and `DbStats.lifecycleEvents`.
- Modify `src/lib/api.ts`: parse optional lifecycle-aware fields.
- Modify `src/lib/api.test.ts`: verify parser keeps lifecycle metadata.
- Modify `src/lib/statistics.ts`: exclude non-active lifecycle events from active duration/application summaries.
- Modify `src/lib/statistics.test.ts`: verify lock intervals do not inflate active statistics.
- Modify `src/lib/health.ts` and `src/lib/health.test.ts`: parse lifecycle db stats.
- Modify `docs/api/next-query-api.md`: document the prototype response shape.
- Modify `docs/superpowers/plans/2026-05-24-next-iteration-architecture-roadmap.md`: mark this prototype as the completed core slice and refine the next roadmap.

## Task 1: Lifecycle Storage Contract

**Files:**
- Modify: `collector/src/models.rs`
- Modify: `collector/src/storage.rs`
- Modify: `collector/tests/storage_tests.rs`

- [ ] **Step 1: Write failing storage tests**

Add tests that express the target storage API before implementation:

```rust
#[test]
fn persists_lifecycle_events_in_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store.insert_lifecycle_event(
        &session_id,
        ts("2026-05-23T09:05:00Z"),
        LifecycleType::WindowsLock,
        Some("manual_lock"),
        serde_json::json!({"source": "test"}),
    ).unwrap();
    store.insert_lifecycle_event(
        &session_id,
        ts("2026-05-23T09:20:00Z"),
        LifecycleType::WindowsUnlock,
        None,
        serde_json::json!({}),
    ).unwrap();

    let rows = store.list_lifecycle_events(10).unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].session_id, session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::WindowsLock);
    assert_eq!(rows[0].reason.as_deref(), Some("manual_lock"));
    assert_eq!(rows[1].lifecycle_type, LifecycleType::WindowsUnlock);
}

#[test]
fn closes_stale_sessions_as_abnormal_stop_with_collector_gap() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let stale_session_id = store.create_session("0.1.0", "test-config").unwrap();

    let closed = store.close_stale_sessions(
        ts("2026-05-23T10:00:00Z"),
        "abnormal_stop",
    ).unwrap();

    assert_eq!(closed, vec![stale_session_id.clone()]);
    let rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(rows[0].session_id, stale_session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::CollectorGap);
    assert_eq!(rows[0].reason.as_deref(), Some("abnormal_stop"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector persists_lifecycle_events_in_order closes_stale_sessions_as_abnormal_stop_with_collector_gap -- --nocapture
```

Expected: FAIL because `LifecycleType`, `insert_lifecycle_event`, `list_lifecycle_events`, and `close_stale_sessions` do not exist.

- [ ] **Step 3: Implement minimal lifecycle storage**

Add model types and storage helpers:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleType {
    SessionStart,
    SessionStop,
    WindowsLock,
    WindowsUnlock,
    PowerSuspend,
    PowerResume,
    IdleStart,
    IdleEnd,
    CaptureUnavailable,
    CollectorGap,
    SessionDisconnect,
    SessionReconnect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    pub raw_event_id: i64,
    pub session_id: String,
    pub event_ts: DateTime<Utc>,
    pub lifecycle_type: LifecycleType,
    pub reason: Option<String>,
    pub active_session_id: Option<String>,
    pub payload: serde_json::Value,
}
```

`storage.rs` must create `lifecycle_events`, tolerate an existing `capture_sessions` table without `ended_reason`, and add:

```rust
pub fn insert_lifecycle_event(
    &mut self,
    session_id: &str,
    event_ts: DateTime<Utc>,
    lifecycle_type: LifecycleType,
    reason: Option<&str>,
    payload: serde_json::Value,
) -> Result<i64>

pub fn list_lifecycle_events(&self, limit: usize) -> Result<Vec<LifecycleEvent>>

pub fn close_session(
    &mut self,
    session_id: &str,
    ended_at: DateTime<Utc>,
    reason: &str,
) -> Result<()>

pub fn close_stale_sessions(
    &mut self,
    ended_at: DateTime<Utc>,
    reason: &str,
) -> Result<Vec<String>>
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test storage_tests -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/models.rs collector/src/storage.rs collector/tests/storage_tests.rs
git commit -m "feat(lifecycle): add lifecycle event storage"
```

## Task 2: Lifecycle-Aware Time Intervals

**Files:**
- Modify: `collector/src/interval.rs`
- Modify: `collector/tests/interval_tests.rs`
- Modify: `src/types.ts`
- Modify: `src/lib/statistics.ts`
- Modify: `src/lib/statistics.test.ts`

- [ ] **Step 1: Write failing interval and statistics tests**

Add Rust tests:

```rust
#[test]
fn cuts_active_window_interval_at_lock_and_adds_locked_interval() {
    let window_events = vec![
        event(1, "session-1", "2026-05-23T09:00:00Z", "Code", "main.rs"),
        event(2, "session-1", "2026-05-23T09:25:00Z", "Browser", "Docs"),
    ];
    let lifecycle_events = vec![
        lifecycle(10, "session-1", "2026-05-23T09:05:00Z", LifecycleType::WindowsLock),
        lifecycle(11, "session-1", "2026-05-23T09:20:00Z", LifecycleType::WindowsUnlock),
    ];

    let intervals = build_time_events_with_lifecycle(&window_events, &lifecycle_events);

    assert_eq!(intervals[0].id, "raw-1");
    assert_eq!(intervals[0].duration_seconds, Some(300));
    assert_eq!(intervals[1].id, "lifecycle-10");
    assert_eq!(intervals[1].kind, TimeEventKind::Lifecycle);
    assert_eq!(intervals[1].status.as_deref(), Some("windows_lock"));
    assert_eq!(intervals[1].duration_seconds, Some(900));
}

#[test]
fn does_not_bridge_window_intervals_between_sessions() {
    let window_events = vec![
        event(1, "session-1", "2026-05-23T09:00:00Z", "Code", "main.rs"),
        event(2, "session-2", "2026-05-23T10:00:00Z", "Code", "README.md"),
    ];

    let intervals = build_time_events_with_lifecycle(&window_events, &[]);

    assert_eq!(intervals[0].duration_seconds, None);
    assert_eq!(intervals[1].duration_seconds, None);
}
```

Add TypeScript test:

```ts
it("excludes lifecycle intervals from active duration and application summaries", () => {
  const locked: TimeEvent = {
    id: "lifecycle-10",
    app: "System",
    title: "Locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-05-23T09:05:00.000Z",
    endedAt: "2026-05-23T09:20:00.000Z",
    durationSeconds: 900
  };

  expect(summarizeDurations([...events, locked]).total).toBe(1800);
  expect(summarizeByApplication([...events, locked]).map((row) => row.app)).not.toContain("System");
});
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test interval_tests -- --nocapture
npm test -- --run src/lib/statistics.test.ts
```

Expected: FAIL because lifecycle-aware builders and `TimeEvent.kind` do not exist.

- [ ] **Step 3: Implement interval builder and frontend active filters**

`build_time_events` remains backward compatible:

```rust
pub fn build_time_events(events: &[StoredWindowEvent]) -> Vec<TimeEvent> {
    build_time_events_with_lifecycle(events, &[])
}
```

`build_time_events_with_lifecycle` must:

- end active window intervals at the next same-session window event
- never bridge different `session_id` values
- cut active intervals at same-session `windows_lock`, `power_suspend`, `idle_start`, `capture_unavailable`, `session_stop`, or `collector_gap`
- emit lifecycle intervals for lock/unlock, suspend/resume, and idle start/end pairs
- sort final rows by `started_at`, then `id`

`src/lib/statistics.ts` must treat `event.kind === "lifecycle"` as non-active for aggregate summaries.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test interval_tests -- --nocapture
npm test -- --run src/lib/statistics.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/interval.rs collector/tests/interval_tests.rs src/types.ts src/lib/statistics.ts src/lib/statistics.test.ts
git commit -m "feat(lifecycle): derive lifecycle-aware time events"
```

## Task 3: API Prototype And Session Closure

**Files:**
- Modify: `collector/src/api.rs`
- Modify: `collector/src/main.rs`
- Modify: `collector/tests/api_tests.rs`
- Modify: `src/lib/api.ts`
- Modify: `src/lib/api.test.ts`
- Modify: `src/lib/health.ts`
- Modify: `src/lib/health.test.ts`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing API/parser tests**

Add API tests for:

```rust
#[tokio::test]
async fn serves_lifecycle_events_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    store.insert_lifecycle_event(
        &session_id,
        ts("2026-05-23T09:05:00Z"),
        LifecycleType::WindowsLock,
        Some("manual_lock"),
        serde_json::json!({}),
    ).unwrap();

    let app = api::router(store, None);
    let body = get_json(app, "/api/lifecycle-events").await;

    assert_eq!(body["events"][0]["lifecycleType"], "windows_lock");
    assert_eq!(body["events"][0]["reason"], "manual_lock");
}
```

and `/api/time-events` including a lifecycle lock row with `kind: "lifecycle"`.

Add parser tests that verify `fetchTimeEvents` preserves `kind`, `status`, and `sessionId`.

- [ ] **Step 2: Run tests to verify RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests -- --nocapture
npm test -- --run src/lib/api.test.ts src/lib/health.test.ts
```

Expected: FAIL because the endpoint and parser fields do not exist.

- [ ] **Step 3: Implement API and closure behavior**

Add:

- `LifecycleEventsResponse { events: Vec<LifecycleEvent> }`
- `GET /api/lifecycle-events?limit=...`
- `/api/time-events` calls `build_time_events_with_lifecycle`
- `DbStats.lifecycle_events`
- `api::serve` closes stale sessions before creating a new one
- `main::Record` closes its session as `completed` after `record_for`

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests -- --nocapture
npm test -- --run src/lib/api.test.ts src/lib/health.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add collector/src/api.rs collector/src/main.rs collector/tests/api_tests.rs src/types.ts src/lib/api.ts src/lib/api.test.ts src/lib/health.ts src/lib/health.test.ts
git commit -m "feat(lifecycle): expose lifecycle-aware API prototype"
```

## Task 4: Roadmap And API Documentation

**Files:**
- Modify: `docs/api/next-query-api.md`
- Modify: `docs/superpowers/plans/2026-05-24-next-iteration-architecture-roadmap.md`

- [ ] **Step 1: Update docs**

Document:

- prototype endpoint `/api/lifecycle-events`
- extra `/api/time-events` fields `kind`, `status`, `sessionId`
- current limitation: live Windows message capture is still a next milestone; prototype supports persisted lifecycle facts, stale-session closure, and analyzer semantics
- next roadmap: migration foundation, real Windows lifecycle collector, API v2 query layer, Dayflow UI, Chinese IME input model, Notion Principle export, metric registry

- [ ] **Step 2: Run doc checks**

Run:

```powershell
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 3: Commit**

```powershell
git add docs/api/next-query-api.md docs/superpowers/plans/2026-05-24-next-iteration-architecture-roadmap.md docs/superpowers/plans/2026-05-24-lifecycle-aware-timeline-prototype.md
git commit -m "docs: update roadmap for lifecycle prototype"
```

## Task 5: Verification And Review Gate

**Files:**
- No planned edits unless reviewers find issues.

- [ ] **Step 1: Run full verification**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --all
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector -- --nocapture
npm test -- --run
npm run build
git diff --check
```

Expected: all pass.

- [ ] **Step 2: Run three reviewer-agent rounds**

Dispatch independent reviewer agents with base SHA, head SHA, plan path, design path, and changed files.

Round 1: Architecture and Contract Review.

Round 2: Extensibility and Dependency Review.

Round 3: Integration and Operations Review.

- [ ] **Step 3: Fix critical/important review findings**

If a reviewer finds critical or important issues, add failing tests where behavior changes, implement fixes, rerun the affected review round, and rerun full verification.

- [ ] **Step 4: Push branch**

Run:

```powershell
git push origin codex/0524feature
```

Expected: remote branch updated.
