# Time State Recorder Product Pitch

## One-Sentence Pitch

Time State Recorder is a local-first Windows workday memory layer that turns desktop activity, screenshots, input signals, and AI-assisted summaries into a reviewable daily work log.

中文一句话：

Time State Recorder 是一个本地优先的 Windows 工作日记忆层，把桌面活动、截图证据、输入行为和 AI 辅助总结整理成可回看的每日工作流。

## 30-Second Pitch

Knowledge work is fragmented across editors, browsers, chats, documents, meetings, and personal knowledge systems. At the end of the day, people often remember being busy but cannot reconstruct what actually happened.

Time State Recorder records the local evidence of a workday: active windows, lifecycle events, input activity, screenshots, visual summaries, 5-hour reports, and a Daily Brief. It keeps capture and storage local, exposes reviewable evidence in a WebUI, and produces a Notion-ready archive payload for a personal Daily Diary.

The product gives an individual knowledge worker a private operating log: enough context to restore attention, write a truthful daily record, and understand work patterns without manually starting timers or relying on memory.

## Chinese 30-Second Pitch

知识工作的一天通常被编辑器、浏览器、聊天、文档、会议和 Notion 页面切碎。到晚上时，人往往只知道自己很忙，却很难准确还原今天到底推进了什么、什么时候切换了上下文、哪些工作有证据。

Time State Recorder 做的是一个本地优先的工作日记忆层：它在 Windows 本机记录窗口活动、生命周期事件、输入行为和截图证据，再生成 5 小时报告、Daily Brief，以及可归档到 Notion Daily Diary 的结构化摘要。

它不是员工监控，也不是简单计时器，而是给个人知识工作者使用的本地操作日志，帮助用户恢复上下文、复盘一天，并把真实工作轨迹写进长期知识系统。

## Problem

Personal work tracking usually fails for one of three reasons:

- Manual timers are too easy to forget.
- Calendar and task tools record intentions, not actual work.
- Screenshots and activity logs are too raw to review without interpretation.

The missing layer is an evidence-backed personal memory system that reconstructs what happened without requiring constant manual bookkeeping.

## Product Thesis

The best daily review is not a productivity score. It is a trustworthy reconstruction of the work path:

- What was active?
- What projects appeared?
- When did context switch?
- Which evidence supports the summary?
- What can be archived into a personal knowledge system?

Time State Recorder captures local facts first, derives summaries second, and leaves final interpretation to the user.

## Target User

Primary user:

- Individual knowledge workers who use Windows heavily.
- Researchers, developers, analysts, writers, and operators who need to restore context from a messy workday.
- Users who already keep a Daily Diary, Notion workspace, research log, or personal operating system.

Not the primary user:

- Managers monitoring employees.
- Teams needing billing-grade time sheets.
- Users who only need a simple timer.

## What The Product Does Today

Time State Recorder currently provides:

- Local Windows collector for foreground windows, lifecycle events, input events, and screenshots.
- SQLite-backed activity history and local screenshot file storage.
- React WebUI with Today Flow, timeline, activity review, input activity, screenshot evidence, and health status.
- Optional visual analysis and structured labels.
- 5-hour work trajectory reports.
- Daily Brief with descriptive stats, hourly metrics, baseline comparison, and action trajectory.
- Read-only Notion Daily Archive API for downstream Principles OS diary writing.
- Repo-local smoke artifact for agents wiring the Notion archive workflow.
- Windows release packaging for local use.

## Differentiation

Compared with a time tracker:

- It records actual desktop evidence rather than asking the user to label every block manually.

Compared with a screenshot recorder:

- It adds time intervals, input activity, lifecycle events, summaries, and review flows.

Compared with an AI diary app:

- It keeps raw capture local and treats generated summaries as reviewable interpretations, not authoritative truth.

Compared with a Notion template:

- It provides local evidence and a daily archive payload that Notion can consume.

## Privacy Positioning

The product should be described as local-first, not cloud-only:

- Capture and storage are local.
- The WebUI reads from a local API.
- Screenshots and text segments may contain private information.
- Optional model-backed analysis can send selected evidence or summaries to a configured provider.
- Notion writes happen outside this repo through separate Principles OS tooling.

Suggested wording:

> Local-first by default, explicit when external analysis or archive tools are used.

Avoid claiming:

- "No data ever leaves the machine."
- "Fully private in every mode."
- "Automatically safe for sensitive environments."

## Demo Script

### 1. Start With The Problem

"At the end of a research or coding day, I often need to write a diary or progress note. But the day is scattered across windows, screenshots, chats, and documents. I do not want to reconstruct it from memory."

### 2. Show The Running App

Open:

```text
http://127.0.0.1:5173/
```

Show:

- Today Flow Board.
- Raw/redacted mode.
- Evidence drawer.
- Collector health.

### 3. Show Local Evidence

Point out:

- Window activity and lifecycle rows.
- Screenshot evidence.
- Input activity summaries.
- Daily Tracking coverage.

### 4. Show Summaries

Show:

- 5-hour report.
- Daily Brief.
- Project hints and uncertainty.
- How raw evidence remains available behind the summary.

### 5. Show Notion Archive Contract

Run:

```powershell
npm run smoke:notion-daily-archive
```

Open:

```text
reports/notion-daily-archive-smoke.json
```

Explain:

"Other agents can consume this JSON and write a verified section into my Notion Daily Diary. Time State Recorder does not need a Notion token in the capture loop."

## Investor / Product Framing

### Category

Personal work memory infrastructure.

### Core Insight

The future of personal productivity is not better task lists. It is a local evidence layer that lets agents and humans reconstruct what actually happened.

### Why Now

- Work is increasingly fragmented across many tools.
- AI agents need structured personal context but should not scrape random UI state.
- Local-first capture is becoming more important as privacy concerns rise.
- Personal knowledge systems need verified event evidence, not just generated summaries.

### Wedge

Start with Windows desktop workday reconstruction:

- Capture signals.
- Build a reviewable time flow.
- Generate daily summaries.
- Archive into Notion Daily Diary.

### Expansion Path

- Better Chinese/IME text capture.
- Query API v2 for agents.
- Local OCR and semantic labels.
- Weekly review and project-level memory.
- Integrations with Notion, calendar, wearables, WeChat, and lifelog images.
- Chat over the personal work journal.

## Product Principles

- Evidence before summary.
- Local capture before external integration.
- Reviewable interpretation, not automatic judgment.
- Privacy boundaries must be visible.
- Notion and other systems consume exports; they do not belong in the capture hot path.
- The UI should help the user reconstruct a day, not shame them with productivity scores.

## Current Gaps

- No installed Windows service or tray app yet.
- No full IME/composed Chinese or Japanese text capture.
- Optional model-backed analysis needs explicit provider configuration and privacy review.
- Daily archive writes are handled by external Principles OS tooling, not this repo.
- The product is still Windows-first and local single-user.

## Closing Line

Time State Recorder gives a knowledge worker something they usually do not have: an evidence-backed memory of the day that can be reviewed locally and archived into a long-term personal operating system.
