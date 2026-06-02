# Activity Review Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first PRD-facing Activity Review view that presents 3-minute activity buckets as human-friendly self-insight.

**Architecture:** Reuse the backend `GET /api/activity-buckets` endpoint introduced in the previous slice. Keep App as the live-data/privacy boundary, add a focused API parser, a small activity-summary model, and a controlled React view that can render either sample or live bucket data. Keep raw titles/evidence hidden in redacted mode.

**Tech Stack:** React 19, TypeScript, Vitest, Testing Library, existing Vite WebUI.

---

## File Structure

- Modify `src/types.ts`: add `ActivityBucket`, `BucketEvidence`, `ActivityCategory`, and `AttentionState` types matching backend camelCase JSON.
- Modify `src/lib/api.ts`: add `fetchActivityBuckets(date, bucketSeconds)` and parser validation.
- Create `src/lib/activity.ts`: category labels, attention labels, summary totals, and duration helpers.
- Create `src/lib/activity.test.ts`: tests for summaries and labels.
- Modify `src/lib/api.test.ts`: tests for `/api/activity-buckets` parsing and invalid response handling.
- Create `src/data/activitySample.ts`: sample buckets for offline/default view.
- Create `src/ActivityReview.tsx`: Activity Review view.
- Modify `src/App.tsx`: add Activity Review tab, state, live refresh, sample reset, date query integration, and privacy gating.
- Modify `src/App.test.tsx`: tests for tab rendering, live bucket query date, and redacted title hiding.
- Modify `src/styles.css`: Activity Review layout and responsive styling.

## Task 1: Activity Bucket API Client

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/api.ts`
- Test: `src/lib/api.test.ts`

- [x] **Step 1: Write parser tests**

Add tests in `src/lib/api.test.ts`:

```ts
it("fetches activity buckets for a date", async () => {
  const fetcher = vi.fn().mockResolvedValue(jsonResponse({
    date: "2026-05-24",
    bucketSeconds: 180,
    buckets: [{
      id: "bucket-1",
      startAt: "2026-05-24T10:00:00Z",
      endAt: "2026-05-24T10:03:00Z",
      bucketSeconds: 180,
      dominantApp: "Code",
      dominantTitle: "main.rs",
      normalizedTitle: "main.rs",
      dominantDurationSeconds: 150,
      switchCount: 1,
      projectId: null,
      projectName: null,
      activityCategory: "coding",
      attentionState: "deep_focus",
      confidence: 0.83,
      evidence: [],
      visualSummaryId: null
    }]
  }));

  const result = await fetchActivityBuckets("2026-05-24", 180, fetcher);

  expect(fetcher).toHaveBeenCalledWith("/api/activity-buckets?date=2026-05-24&bucketSeconds=180");
  expect(result.date).toBe("2026-05-24");
  expect(result.buckets[0].dominantApp).toBe("Code");
});

it("rejects invalid activity bucket rows", async () => {
  const fetcher = vi.fn().mockResolvedValue(jsonResponse({
    date: "2026-05-24",
    bucketSeconds: 180,
    buckets: [{ id: "broken" }]
  }));

  await expect(fetchActivityBuckets("2026-05-24", 180, fetcher)).rejects.toThrow(
    /invalid activity bucket/i
  );
});
```

- [x] **Step 2: Run parser tests and verify failure**

Run:

```powershell
npm.cmd test -- src/lib/api.test.ts
```

Expected: tests fail because `fetchActivityBuckets` and activity bucket types do not exist.

- [x] **Step 3: Implement parser**

Add the frontend types in `src/types.ts`. Add `fetchActivityBuckets` to `src/lib/api.ts`; it must construct `/api/activity-buckets?date=${encodeURIComponent(date)}&bucketSeconds=${bucketSeconds}`, validate the top-level `date`, `bucketSeconds`, and every bucket row, and allow nullable `projectId`, `projectName`, and `visualSummaryId`.

- [x] **Step 4: Verify parser tests pass**

Run:

```powershell
npm.cmd test -- src/lib/api.test.ts
```

Expected: parser tests pass.

## Task 2: Activity Summary Model

**Files:**
- Create: `src/lib/activity.ts`
- Test: `src/lib/activity.test.ts`

- [x] **Step 1: Write summary tests**

Create tests that assert `summarizeActivityBuckets` returns total bucket count, total active seconds, top categories, attention counts, and switch total from two buckets.

- [x] **Step 2: Run model tests and verify failure**

Run:

```powershell
npm.cmd test -- src/lib/activity.test.ts
```

Expected: compile failure because `src/lib/activity.ts` does not exist.

- [x] **Step 3: Implement summary helpers**

Implement:

```ts
export function summarizeActivityBuckets(buckets: ActivityBucket[]): ActivityReviewSummary
export function activityCategoryLabel(category: ActivityCategory): string
export function attentionStateLabel(state: AttentionState): string
export function formatBucketMinutes(seconds: number): string
```

Use deterministic maps and avoid inferring moral labels from unknown categories.

- [x] **Step 4: Verify model tests pass**

Run:

```powershell
npm.cmd test -- src/lib/activity.test.ts
```

Expected: model tests pass.

## Task 3: Activity Review View

**Files:**
- Create: `src/data/activitySample.ts`
- Create: `src/ActivityReview.tsx`
- Modify: `src/styles.css`
- Test: `src/App.test.tsx`

- [x] **Step 1: Write UI tests**

Add tests that render the Activity Review tab, show summary cards, hide raw titles in redacted mode, and reveal normalized titles after switching to raw mode.

- [x] **Step 2: Run UI tests and verify failure**

Run:

```powershell
npm.cmd test -- src/App.test.tsx
```

Expected: tests fail because the Activity Review tab/view does not exist.

- [x] **Step 3: Implement component and styles**

Create `ActivityReview.tsx` with summary metrics, category distribution, attention rhythm, bucket list, and evidence drawer. Use existing `panel`, `metric`, and button styles where possible. In redacted mode, show `Hidden in redacted mode` for raw titles/evidence.

- [x] **Step 4: Verify UI tests pass**

Run:

```powershell
npm.cmd test -- src/App.test.tsx
```

Expected: UI tests pass.

## Task 4: App Integration

**Files:**
- Modify: `src/App.tsx`
- Test: `src/App.test.tsx`

- [x] **Step 1: Write live-fetch integration test**

Extend App tests so selecting Query Date calls `/api/activity-buckets?date=2026-05-24&bucketSeconds=180`, and live buckets appear in Activity Review.

- [x] **Step 2: Implement App state integration**

Add `activityBuckets`, `activityStatus`, `activityLoading`, and `activityError` state. Load sample buckets in `loadSample`. In `refreshCollector`, fetch activity buckets alongside time events, input summary, screenshot summary, and health. Guard stale responses using the existing request generation pattern.

- [x] **Step 3: Verify App integration tests pass**

Run:

```powershell
npm.cmd test -- src/App.test.tsx
```

Expected: App integration tests pass.

## Task 5: Verification And Commit

**Files:**
- No new implementation files.

- [x] **Step 1: Run frontend suite**

Run:

```powershell
npm.cmd test
```

Expected: all frontend tests pass.

- [x] **Step 2: Run production build**

Run:

```powershell
npm.cmd run build
```

Expected: TypeScript and Vite build pass.

- [x] **Step 3: Run diff check**

Run:

```powershell
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 4: Commit frontend slice**

Run:

```powershell
git add src/types.ts src/lib/api.ts src/lib/api.test.ts src/lib/activity.ts src/lib/activity.test.ts src/data/activitySample.ts src/ActivityReview.tsx src/App.tsx src/App.test.tsx src/styles.css docs/superpowers/plans/2026-06-03-activity-review-frontend.md
git commit -m "feat: add activity review frontend"
```

Expected: one frontend commit on `codex/activity-insights-prd-v1-2`.
