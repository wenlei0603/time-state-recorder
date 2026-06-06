# Notion Daily Archive API

## Endpoint

`GET /api/notion/daily-archive?date=YYYY-MM-DD&tzOffsetMinutes=-480`

The endpoint is read-only. It is intended for local Notion Principles OS archival jobs and reuses the same local-date behavior as the frontend daily brief route.

## Response

- `date`: selected local date.
- `generatedAt`: response generation time.
- `archiveTitle`: human title for the archive payload.
- `dailyDiaryTitle`: target INDEXv1 daily diary title.
- `source`: app and endpoint metadata for provenance.
- `status`: daily brief status, or `missing` when the generated daily brief does not exist yet.
- `archiveMarkdown`: human-readable diary text.
- `brief`: generated daily brief when available.
- `fiveHourReports`: same-day 5-hour reports.
- `descriptiveStats`: activity, input, screenshot, app, category, and report counts.
- `hourlyMetrics`: hourly workflow metrics and linked 5-hour report IDs.
- `comparison`: comparison against recent baseline days.

## Notion Principles OS Use

1. Ensure the target Daily Diary exists: `npm run notion:os -- ensure-diary 2026-06-05`.
2. Fetch the archive endpoint from the running TSR collector.
3. Append a diary section with marker `TSR Daily Archive | 2026-06-05`.
4. Verify the diary marker with `npm run notion:os -- verify-diary 2026-06-05` and block readback.

The Notion write job should be idempotent and must not create durable Tasks from report text automatically.

## Repo-Local Smoke Command

Run this before wiring or changing the Notion Principles OS archive job:

```powershell
npm run smoke:notion-daily-archive
```

The command starts an in-memory collector API with sample daily brief data, calls `/api/notion/daily-archive?date=2026-05-24&tzOffsetMinutes=0`, asserts the required markdown sections and same-day 5-hour report filtering, then writes `reports/notion-daily-archive-smoke.json`.

The JSON artifact is an exact sample endpoint response. Other agents can use it as the local contract fixture when building the Principles OS diary append and verification flow.
