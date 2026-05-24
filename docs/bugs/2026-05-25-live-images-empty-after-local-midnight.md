# Live Images Empty After Local Midnight

Date recorded: 2026-05-25

## Symptom

The Daily Tracking view can show `0 screenshots` and no live image rows even while the collector is running and `dbStats.screenshots` is increasing.

## Root Cause

The collector stores screenshot rows and paths by UTC calendar date, for example `2026-05-24/16-38.jpg`. The Toggl-style Web UI used the user's local calendar date for live queries. In Asia/Shanghai, local `2026-05-25 00:38` is still UTC `2026-05-24 16:38`, so the UI queried `date=2026-05-25` while the collector data was under `date=2026-05-24`.

The same UI also hides screenshot images in `Redacted` privacy mode. Images are visible only when `Privacy` is set to `Raw` and the `Screenshots` layer is enabled.

## Evidence

- `GET /api/screenshots?date=2026-05-25` returned an empty screenshots list.
- `GET /api/screenshots?date=2026-05-24` returned live screenshot metadata.
- `GET /screenshots/2026-05-24/16-38.jpg` returned `200 image/jpeg`.

## Fix

Use the collector UTC date as the default query date and expose an explicit Query Date control so users can inspect older collector days directly.
