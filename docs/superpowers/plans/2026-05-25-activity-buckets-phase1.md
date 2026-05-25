# Activity Buckets Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first backend slice for 3-minute activity buckets: deterministic aggregation, browser-title normalization, basic category and attention fields, and a local API endpoint.

**Architecture:** Keep raw window/lifecycle collection unchanged. Add a pure derivation module that converts existing `TimeEvent` rows into stable `ActivityBucket` DTOs, then expose it through an Axum route. Do not add database tables or visual-model code in this slice.

**Tech Stack:** Rust, Axum, chrono, serde, existing SQLite `Store`, existing collector integration tests.

---

## File Structure

- Create `collector/src/activity.rs`: pure activity bucket domain and derivation functions.
- Modify `collector/src/models.rs`: add `ActivityBucket`, `ActivityCategory`, `AttentionState`, `BucketEvidence`, `BucketQuery`-compatible response structs.
- Modify `collector/src/lib.rs`: export `activity`.
- Modify `collector/src/api.rs`: add `GET /api/activity-buckets` route that reuses existing `TimeEvent` construction.
- Create `collector/tests/activity_tests.rs`: unit tests for bucketization, title normalization, category heuristics, and attention state.
- Modify `collector/tests/api_tests.rs`: API integration test for `/api/activity-buckets?date=...&bucketSeconds=180`.

## Task 1: Activity Bucket Pure Functions

**Files:**
- Create: `collector/src/activity.rs`
- Modify: `collector/src/models.rs`
- Modify: `collector/src/lib.rs`
- Test: `collector/tests/activity_tests.rs`

- [x] **Step 1: Write failing tests**

Add tests that assert:

```rust
#[test]
fn buckets_choose_dominant_activity_and_count_switches()
```

with three `TimeEvent` rows inside one 3-minute bucket. Expected:

- bucket start `2026-05-23T09:00:00Z`
- bucket end `2026-05-23T09:03:00Z`
- dominant app `Code`
- dominant title `main.rs`
- dominant duration `120`
- switch count `2`
- attention state `light_switching`

Add tests that assert:

```rust
#[test]
fn normalizes_browser_titles()
```

Expected examples:

- `"GitHub - Pull Request - Google Chrome"` becomes `"GitHub - Pull Request"`
- `"Time State Recorder 和另外 2 个页面 - 个人 - Microsoft Edge"` becomes `"Time State Recorder"`

Add tests that assert:

```rust
#[test]
fn does_not_infer_loafing_without_explicit_rule()
```

Expected: a YouTube-like browser title defaults to `unknown`, not `loafing`.

- [x] **Step 2: Verify tests fail**

Run:

```powershell
cargo test -p tsr-collector --test activity_tests -- --nocapture
```

Expected: compile failure because `tsr_collector::activity` and `ActivityBucket` types do not exist.

- [x] **Step 3: Implement minimal bucketizer**

Implement:

```rust
pub fn build_activity_buckets(events: &[TimeEvent], query: ActivityBucketQuery) -> Vec<ActivityBucket>
pub fn normalize_browser_title(process_name: &str, title: &str) -> String
```

Rules:

- Only use intervals with finite positive `duration_seconds`.
- Split intervals across fixed UTC bucket boundaries.
- Bucket start/end are aligned to `bucket_seconds`.
- Dominant activity is the evidence row with max seconds in that bucket.
- Switch count counts distinct active-window evidence transitions inside the bucket.
- Lifecycle rows map to `idle` category and `away` attention.
- Initial category heuristics:
  - lifecycle -> `idle`
  - process/title containing Code, Cursor, cargo, rust, tsr -> `coding`
  - process/title containing Word, docx, writing -> `writing`
  - process/title containing Weixin, WeChat, mail, Outlook -> `communication`
  - browser process -> `research` unless no meaningful title, then `unknown`
  - otherwise `unknown`
- Do not infer `loafing` in this slice.
- Attention:
  - lifecycle dominant -> `away`
  - switchCount <= 1 and dominantShare >= 0.85 -> `deep_focus`
  - switchCount <= 2 and dominantShare >= 0.75 -> `steady`
  - switchCount <= 5 or dominantShare >= 0.45 -> `light_switching`
  - otherwise -> `fragmented`

- [x] **Step 4: Verify tests pass**

Run:

```powershell
cargo test -p tsr-collector --test activity_tests -- --nocapture
```

Expected: all activity tests pass.

## Task 2: API Route

**Files:**
- Modify: `collector/src/api.rs`
- Test: `collector/tests/api_tests.rs`

- [x] **Step 1: Write failing API test**

Add:

```rust
#[tokio::test]
async fn serves_activity_buckets_for_date()
```

Setup two window focus rows:

- `2026-05-23T09:00:00Z`, Code, `main.rs`
- `2026-05-23T09:02:00Z`, Browser, `GitHub - Pull Request - Google Chrome`
- `2026-05-23T09:04:00Z`, Code, `lib.rs`

Request:

```text
/api/activity-buckets?date=2026-05-23&bucketSeconds=180
```

Expected:

- HTTP 200
- JSON has `buckets`
- first bucket has `bucketSeconds = 180`
- first bucket `dominantApp = "Code"`
- first bucket `normalizedTitle = "main.rs"`
- second bucket exists

- [x] **Step 2: Verify API test fails**

Run:

```powershell
cargo test -p tsr-collector --test api_tests serves_activity_buckets_for_date -- --nocapture
```

Expected: HTTP 404 or compile failure because route does not exist.

- [x] **Step 3: Implement route**

Add route:

```text
GET /api/activity-buckets
```

Query params:

```text
date=YYYY-MM-DD
bucketSeconds=180
limit=10000
```

Validation:

- `bucketSeconds` defaults to 180.
- `bucketSeconds` must be between 60 and 3600.
- `date` defaults to current UTC date if missing.
- Invalid `date` returns HTTP 400.

Implementation:

- Load window/lifecycle events like `/api/time-events`.
- Build `TimeEvent` rows.
- Filter relevant events by UTC date overlap.
- Call `build_activity_buckets`.
- Return `{ "date": "...", "bucketSeconds": 180, "buckets": [...] }`.

- [x] **Step 4: Verify API test passes**

Run:

```powershell
cargo test -p tsr-collector --test api_tests serves_activity_buckets_for_date -- --nocapture
```

Expected: test passes.

## Task 3: Full Backend Verification

**Files:**
- No new files.

- [x] **Step 1: Run collector tests**

Run:

```powershell
cargo test -p tsr-collector -- --nocapture
```

Expected: all collector tests pass.

- [x] **Step 2: Run formatting/checks**

Run:

```powershell
cargo fmt --check
git diff --check
```

Expected: both pass.

- [x] **Step 3: Commit backend slice**

Run:

```powershell
git add collector/src/activity.rs collector/src/models.rs collector/src/lib.rs collector/src/api.rs collector/tests/activity_tests.rs collector/tests/api_tests.rs docs/superpowers/plans/2026-05-25-activity-buckets-phase1.md
git commit -m "feat: add activity bucket API"
```

Expected: commit succeeds on `codex/timeline-activity-buckets`.

## Self-Review

- This plan implements only the first PRD slice: backend bucket API.
- It intentionally excludes frontend Activity Review, high-resolution screenshots, and visual model analysis.
- It keeps raw collection unchanged.
- It uses TDD for both pure derivation and API route behavior.
