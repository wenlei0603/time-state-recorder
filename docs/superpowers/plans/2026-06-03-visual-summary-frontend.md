# Visual Summary Frontend Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface backend visual summaries in Activity Review without exposing summary text in redacted mode.

**Architecture:** Add a typed frontend parser for `GET /api/visual-summaries`, keep App as the live-data boundary, and pass visual summaries into `ActivityReview`. Match summaries to a selected 3-minute bucket by `capturedAt` falling inside the bucket interval.

**Tech Stack:** React 19, TypeScript, Vitest, Testing Library, existing Vite WebUI.

---

## Tasks

- [x] Add `VisualSummary` frontend type and parser tests.
- [x] Implement `fetchVisualSummaries(date)` in `src/lib/screenshots.ts`.
- [x] Add sample visual summaries.
- [x] Integrate visual summaries into App refresh/sample state.
- [x] Render visual summary availability in Activity Review, with summary text hidden unless privacy mode is `raw`.
- [x] Run `npm.cmd test`, `npm.cmd run build`, and `git diff --check`.
- [x] Commit as `feat: surface visual summaries in activity review`.
