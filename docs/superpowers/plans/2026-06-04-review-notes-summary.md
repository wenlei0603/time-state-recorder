# Review Notes Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the raw-looking insight summary cards with structured Review Notes that render JSON-like backend text correctly and use an Academia-style serif presentation.

**Architecture:** Add a small presentation helper in `src/lib/insightPresentation.ts` so JSON extraction and report sectioning are testable outside React. Update `src/InsightFeedback.tsx` to consume that helper and render Review Notes with privacy-safe states. Refresh the existing `aiInsight*` CSS classes into a quieter macOS academic notes layout.

**Tech Stack:** React 19, TypeScript, Vitest, Testing Library, lucide-react, Vite.

---

### Task 1: Test Insight Summary Normalization

**Files:**
- Create: `src/lib/insightPresentation.test.ts`
- Create: `src/lib/insightPresentation.ts`

- [ ] **Step 1: Write failing tests**

Create tests that prove:

```ts
import { describe, expect, it } from "vitest";
import {
  buildReportNote,
  buildWindowNote,
  stripJsonSummarySyntax,
} from "./insightPresentation";
import type { InsightReport, VisualWindowSummary } from "../types";

it("extracts a fenced JSON summary instead of returning raw object syntax", () => {
  const note = buildWindowNote({
    ...windowSummary(),
    summaryText: "```json\n{\"summaryText\":\"Focused Overpayment analysis.\",\"taskIntent\":\"Prepare regression table\",\"continuity\":\"Continues formal analysis\"}\n```",
  });

  expect(note.thesis).toBe("Focused Overpayment analysis.");
  expect(note.intent).toBe("Prepare regression table");
  expect(note.continuity).toBe("Continues formal analysis");
  expect(note.rawAvailable).toBe(true);
  expect(note.thesis).not.toMatch(/[{}]|```|summaryText/);
});

it("turns a long five-hour report into readable sections", () => {
  const note = buildReportNote({
    ...report(),
    summaryText:
      "5小时工作轨迹可分四个阶段。① 教学协调阶段(06:20-07:00)：处理课程材料。② Stata实证阶段(07:15-07:35)：推进do-file。③ Codex工程阶段(07:35-08:10)：整理worktree。整体呈现科研-工程并行。",
  });

  expect(note.mainThread).toContain("5小时工作轨迹");
  expect(note.phases).toHaveLength(3);
  expect(note.phases[0].label).toContain("教学协调阶段");
  expect(note.fullText).toContain("整体呈现");
});

it("removes JSON wrapper syntax without hiding plain prose", () => {
  expect(stripJsonSummarySyntax("Plain summary.")).toBe("Plain summary.");
  expect(stripJsonSummarySyntax("{\"summaryText\":\"Plain JSON summary.\"}")).toBe(
    "Plain JSON summary.",
  );
});
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run: `npm test -- src/lib/insightPresentation.test.ts`

Expected: fails because `src/lib/insightPresentation.ts` does not exist.

- [ ] **Step 3: Implement presentation helper**

Create `src/lib/insightPresentation.ts` with exported helpers:

- `stripJsonSummarySyntax(value: string): string`
- `buildWindowNote(summary: VisualWindowSummary): WindowReviewNote`
- `buildReportNote(report: InsightReport): ReportReviewNote`

The helper should parse fenced JSON, plain JSON strings, and raw strings. It should extract known fields and never return raw JSON syntax as the default thesis.

- [ ] **Step 4: Run focused tests and confirm GREEN**

Run: `npm test -- src/lib/insightPresentation.test.ts`

Expected: all tests pass.

### Task 2: Render Review Notes

**Files:**
- Modify: `src/InsightFeedback.tsx`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Write failing render expectations**

Update the existing "shows AI insight status..." app test so it expects:

- region label contains `review notes`
- heading text is `Review Notes`
- redacted mode hides note text but shows evidence metadata
- raw mode shows the normalized thesis and phase/timeline labels
- raw mode does not show fenced JSON or object syntax

- [ ] **Step 2: Run the app test and confirm RED**

Run: `npm test -- src/App.test.tsx -t "shows"`

Expected: fails because the component still renders `AI insight` and raw paragraph layout.

- [ ] **Step 3: Update `InsightFeedback.tsx`**

Replace raw paragraph rendering with:

- section label: `Review Notes`
- window note title: `Window note`
- report title: `Session report`
- `buildWindowNote` and `buildReportNote` outputs
- three compact trajectory cells for the 1/3/5 minute cadence
- structured report rows for phases, project hints, category mix, and evidence
- `<details>` disclosures for full report / raw JSON only in raw mode

- [ ] **Step 4: Run the focused app test and confirm GREEN**

Run: `npm test -- src/App.test.tsx -t "shows"`

Expected: the updated expectations pass.

### Task 3: Apply Academic Mac Styling

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Restyle existing insight classes**

Update the `aiInsight*` CSS section to:

- use a serif stack for Review Notes headings and body: `ui-serif, "New York", "Iowan Old Style", "Palatino Linotype", Palatino, Georgia, serif`
- keep controls and metadata in the existing sans-serif stack
- use deep pine and sage tokens, remove blue from insight icons/badges
- remove all-bold paragraph styling
- cap note height and use collapsible details for long content
- stack cleanly below 620px

- [ ] **Step 2: Run visual-oriented build checks**

Run:

- `npm test -- src/lib/insightPresentation.test.ts src/App.test.tsx -t "shows|insightPresentation"`
- `npm run build`
- `git diff --check`

Expected: tests, TypeScript build, and whitespace check pass.

### Task 4: Screenshot Verification

**Files:**
- No code edits unless screenshot review shows layout defects.

- [ ] **Step 1: Verify services and capture desktop/mobile screenshots**

Use the running app at `http://127.0.0.1:5173/`. Capture desktop and mobile screenshots after switching to raw mode if live data is available.

- [ ] **Step 2: Fix layout defects if present**

If screenshots show overlap, unreadable text, raw JSON leakage, or one-note color treatment, patch CSS/markup and rerun Task 3 checks.

- [ ] **Step 3: Commit**

Commit only the implementation files and this plan:

```bash
git add docs/superpowers/plans/2026-06-04-review-notes-summary.md src/lib/insightPresentation.ts src/lib/insightPresentation.test.ts src/InsightFeedback.tsx src/App.test.tsx src/styles.css
git commit -m "feat: redesign review notes summary"
```
