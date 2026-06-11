# Structured Diary Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Goal

Add a human-readable structured diary dashboard to the daily brief flow. The dashboard should summarize the day at the work-overview level: collaborators, places, work types, user roles, role heterogeneity, team count, team distribution, and duration distribution. It must avoid raw 5-minute detail and use MiniMax-M3 daily brief generation when configured.

## File Structure

- `collector/src/models.rs`: add serializable dashboard DTOs shared by API and tests.
- `collector/src/diary_dashboard.rs`: parse model-provided `diaryDashboard` from `rawSummaryJson` and build a conservative fallback from existing stats/hourly metrics/reports.
- `collector/src/insights.rs`: update the MiniMax daily brief prompt/schema expectation to request `diaryDashboard`.
- `collector/src/api.rs`: include `diaryDashboard` in `DailyBriefResponse`.
- `collector/tests/insight_tests.rs`: verify the MiniMax-M3 daily brief prompt asks for the dashboard dimensions.
- `collector/tests/api_tests.rs`: verify `/api/daily-brief` returns dashboard data from model JSON or fallback.
- `src/types.ts`, `src/lib/dailyBrief.ts`: parse the new response field.
- `src/DailyBriefPanel.tsx` plus a small component: render the dashboard while honoring raw/redacted privacy.
- `src/lib/dailyBrief.test.ts`, `src/App.test.tsx`: verify frontend parsing and rendering.
- `src/styles.css`: add compact dashboard styling inside the existing Daily Brief section.

## Tasks

- [x] Write failing Rust tests for dashboard prompt and `/api/daily-brief` response.
- [x] Write failing TypeScript tests for API parsing and Daily Brief panel rendering.
- [x] Add Rust dashboard DTOs and fallback builder.
- [x] Wire `diaryDashboard` into the backend response and MiniMax daily brief prompt.
- [x] Add frontend types/parser/component rendering.
- [x] Run targeted Rust and TypeScript tests.
- [x] Run full build/test verification as time allows.
- [ ] Commit, push `codex/structured-diary-dashboard`, and open a PR.
