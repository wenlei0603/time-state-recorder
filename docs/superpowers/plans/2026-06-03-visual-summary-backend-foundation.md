# Visual Summary Backend Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the backend storage and API foundation for screenshot visual summaries.

**Architecture:** Store visual summaries as derived records linked to existing screenshot metadata. Expose query and manual analyze endpoints now, using a deterministic `local_stub` analyzer so the API contract is testable before a real model provider is configured.

**Tech Stack:** Rust, Axum, SQLite/rusqlite, serde, existing collector tests.

---

## File Structure

- Modify `collector/src/models.rs`: add `VisualSummary` and activity category DB helpers.
- Modify `collector/src/storage.rs`: add `visual_summaries` table and CRUD/query methods.
- Modify `collector/src/api.rs`: add `GET /api/visual-summaries?date=YYYY-MM-DD` and `POST /api/screenshots/{id}/analyze`.
- Modify `collector/tests/storage_tests.rs`: storage tests for insert/list.
- Modify `collector/tests/api_tests.rs`: API tests for list and analyze.

## Task 1: Visual Summary Storage

- [x] **Step 1: Write failing storage test**

Add a test that inserts a screenshot, inserts a visual summary linked to it, then lists summaries by date and asserts model provider, category, apps, and summary text.

- [x] **Step 2: Run test to verify failure**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test storage_tests visual_summaries -- --nocapture
```

Expected: compile failure because `VisualSummary` and store methods do not exist.

- [x] **Step 3: Implement model and storage**

Add `VisualSummary`, `ActivityCategory::as_str`, `ActivityCategory::from_db`, a `visual_summaries` table, `insert_visual_summary`, `list_visual_summaries_by_date`, and `get_screenshot`.

- [x] **Step 4: Verify storage test passes**

Run the same storage test command. Expected: pass.

## Task 2: Visual Summary API

- [x] **Step 1: Write failing API tests**

Add one test for `GET /api/visual-summaries?date=...` and one test for `POST /api/screenshots/{id}/analyze`.

- [x] **Step 2: Run API tests to verify failure**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector --test api_tests visual -- --nocapture
```

Expected: route failures or compile errors.

- [x] **Step 3: Implement API routes**

Add routes:

```text
GET /api/visual-summaries
POST /api/screenshots/{id}/analyze
```

The analyze endpoint should create a deterministic local stub summary from screenshot metadata and return the inserted summary. Missing screenshot ids return 404.

- [x] **Step 4: Verify API tests pass**

Run the same API test command. Expected: pass.

## Task 3: Full Verification And Commit

- [x] **Step 1: Run collector tests**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p tsr-collector -- --nocapture
```

- [x] **Step 2: Run format and diff checks**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --all -- --check
git diff --check
```

- [ ] **Step 3: Commit backend slice**

```powershell
git add collector/src/models.rs collector/src/storage.rs collector/src/api.rs collector/tests/storage_tests.rs collector/tests/api_tests.rs docs/superpowers/plans/2026-06-03-visual-summary-backend-foundation.md
git commit -m "feat: add visual summary backend foundation"
```
