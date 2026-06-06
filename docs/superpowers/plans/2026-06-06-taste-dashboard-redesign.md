# Taste Dashboard Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Time State Recorder frontend into a working, restrained recorder workspace with clearer information hierarchy, better state visibility, and verified tab flows.

**Architecture:** Keep the existing React/Vite app and API contracts. `App.tsx` continues to own collector source state and top-level navigation; `DailyTracking.tsx`, `InputActivity.tsx`, and `CollectorMonitor.tsx` remain focused view components. Styling remains a single CSS file, but the visual system is rebuilt around shared tokens, compact cards, responsive grids, and explicit states.

**Tech Stack:** React 19, Vite, TypeScript, Vitest, Testing Library, lucide-react already in the project.

---

## Installed Skills And Design Read

Installed from `leonxlnx/taste-skill`:

- `brandkit`
- `brutalist-skill`
- `gpt-tasteskill`
- `image-to-code-skill`
- `imagegen-frontend-mobile`
- `imagegen-frontend-web`
- `minimalist-skill`
- `output-skill`
- `redesign-skill`
- `soft-skill`
- `stitch-skill`
- `taste-skill`
- `taste-skill-v1`

Reading this as: local productivity recorder dashboard for one operator, with a quiet work-focused language, leaning toward an editorial/minimal utility surface rather than a marketing page.

The installed `design-taste-frontend` skill explicitly says it is not for dashboards or data tables, so this implementation follows:

- `redesign-existing-projects`: scan, diagnose, then apply targeted upgrades without framework migration.
- `minimalist-ui`: restrained typography, flat surfaces, low-shadow cards, concise states, and no gradients or decorative AI-purple treatment.
- Local memory: overview-first UI, raw evidence behind explicit tabs, and `npm test -- --run`, `npm run build`, `git diff --check` as the frontend verification loop.

## Current UI Audit

- Header reads like an MVP artifact (`MVP / Features 1 & 3`) instead of a product workspace.
- Tabs use feature-oriented labels (`Statistics`, `Daily Tracking`, `Input Activity`) instead of task-oriented labels.
- The overview shows seven equal metric cards before context; this weakens hierarchy.
- Collector source state is implicit in buttons and errors rather than presented as a concise status strip.
- The CSS repeats responsive rules and relies on a green-tinted card style across everything.
- Tables and timelines work, but spacing, focus, and hover states are not unified.
- The project has good component boundaries; no framework migration is needed.

## Files

- Modify: `src/App.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/DailyTracking.tsx`
- Modify: `src/InputActivity.tsx`
- Replace: `src/styles.css`
- Verify: `npm test -- --run`
- Verify: `npm run build`
- Verify: `git diff --check`

## Task 1: Add Failing UI Contract Tests

**Files:**

- Modify: `src/App.test.tsx`

- [ ] **Step 1: Add tests for the redesigned workspace contract**

Add `fireEvent` to the Testing Library import and add this test:

```tsx
it("surfaces the recorder workspace and task-oriented tabs", () => {
  render(<App />);

  expect(
    screen.getByRole("heading", { name: /today overview/i })
  ).toBeInTheDocument();
  expect(screen.getByText(/sample workspace/i)).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /screenshots/i }));
  expect(
    screen.getByRole("heading", { name: /screenshot timeline/i })
  ).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /input/i }));
  expect(
    screen.getByRole("heading", { name: /keyboard activity/i })
  ).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `npm test -- --run src/App.test.tsx`

Expected: FAIL because the current UI still renders `Time State Recorder`, `Daily Tracking`, and `Input Activity`.

## Task 2: Refactor Top-Level App Information Architecture

**Files:**

- Modify: `src/App.tsx`

- [ ] **Step 1: Implement the redesigned overview shell**

Change `ViewMode` to `overview | screenshots | input`. Replace the header with:

- Eyebrow: `Local recorder`
- H1: `Today overview`
- Supporting text: `A compact readout of active time, app focus, screenshots, keyboard input, and collector health.`
- Status strip showing `Sample workspace`, `Collector online`, `Loading collector`, or `Collector offline`
- Actions: `Sample` and `Refresh`
- Tabs: `Overview`, `Screenshots`, `Input`

- [ ] **Step 2: Reduce metric hierarchy**

Use five overview metrics:

- Events
- Active time
- Mean
- Median
- Max

Keep `Application time`, `Collector Monitor`, and `Event rows` on the overview page.

- [ ] **Step 3: Run the focused test and verify GREEN**

Run: `npm test -- --run src/App.test.tsx`

Expected: PASS for the new UI contract and all existing App tests.

## Task 3: Rename Child Views Without Changing API Contracts

**Files:**

- Modify: `src/DailyTracking.tsx`
- Modify: `src/InputActivity.tsx`

- [ ] **Step 1: Rename view headings**

In `DailyTracking.tsx`, change:

- `Daily Tracking` to `Screenshot timeline`
- date line to `Visual evidence for {date}`
- live action label to `Load live`

In `InputActivity.tsx`, change:

- `Input Activity` to `Keyboard activity`
- subtitle to `Raw Input segments grouped by application`
- live action label to `Load live`

- [ ] **Step 2: Run the focused test**

Run: `npm test -- --run src/App.test.tsx`

Expected: PASS.

## Task 4: Rebuild CSS Around A Quiet Utility System

**Files:**

- Replace: `src/styles.css`

- [ ] **Step 1: Define shared tokens**

Use CSS custom properties for canvas, panel, text, muted text, border, accent, warning, danger, and focus.

- [ ] **Step 2: Style the new shell**

Implement:

- `skipLink`
- `shell`
- `topbar`
- `titleBlock`
- `pageLead`
- `sourceCard`
- `sourceStatus`
- `sourceCopy`
- `tabBar`
- `statsGrid`
- `metric`
- `workspace`
- `panel`
- `tableWrap`

- [ ] **Step 3: Keep existing view classes working**

Restyle existing classes used by `DailyTracking`, `InputActivity`, and `CollectorMonitor`:

- `dailyTracking`
- `dailyHeader`
- `summaryBar`
- `timeline`
- `timelineRow`
- `timelineExpand`
- `inputActivity`
- `segmentRow`
- `segmentText`
- `collectorMonitor`
- `healthGrid`
- `subsystemRow`
- `errorBanner`

- [ ] **Step 4: Run full frontend checks**

Run: `npm test -- --run`

Expected: all tests pass.

Run: `npm run build`

Expected: TypeScript and Vite build pass.

Run: `git diff --check`

Expected: no whitespace errors.

## Task 5: Browser Smoke Test

**Files:**

- Verify rendered app only

- [ ] **Step 1: Start Vite**

Run: `npm run dev -- --port 5178 --strictPort`

Expected: Vite serves `http://127.0.0.1:5178/`.

- [ ] **Step 2: Inspect the app in a browser**

Check:

- Header and source strip fit at desktop width.
- Overview metrics do not wrap badly.
- `Screenshots` and `Input` tabs switch views.
- Mobile width stacks header actions and metric cards cleanly.

- [ ] **Step 3: Stop the dev server**

Stop the Vite process after inspection.

## Task 6: Commit And PR

**Files:**

- Stage only files changed by this plan.

- [ ] **Step 1: Commit plan**

Run:

```powershell
git add docs/superpowers/plans/2026-06-06-taste-dashboard-redesign.md
git commit -m "docs: plan taste dashboard redesign"
```

- [ ] **Step 2: Commit implementation**

Run:

```powershell
git add src/App.test.tsx src/App.tsx src/DailyTracking.tsx src/InputActivity.tsx src/styles.css
git commit -m "refactor: redesign recorder dashboard"
```

- [ ] **Step 3: Push and open draft PR to master**

Run:

```powershell
git push -u origin codex/taste-dashboard-redesign
gh pr create --draft --base master --head codex/taste-dashboard-redesign --title "[codex] redesign recorder dashboard" --body-file <body-file>
```

The PR body must include changed scope, validation commands, and that the main checkout had unrelated dirty changes isolated away in this worktree.
