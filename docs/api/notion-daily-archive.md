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
