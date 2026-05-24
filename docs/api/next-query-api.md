# Next Query API Contract

Date: 2026-05-24

This document specifies the next Time State Recorder query API. It is an architecture contract only; current endpoints remain available until implementation starts.

## Compatibility

Existing endpoints remain v1:

- `GET /api/health`
- `GET /api/window-events`
- `GET /api/lifecycle-events`
- `GET /api/time-events`
- `GET /api/blockers`
- `GET /api/screenshots`
- `GET /api/screenshot-summary`
- `GET /api/input-events`
- `GET /api/input-summary`
- `GET /api/text-segments`

New endpoints live under `/api/v2`.

The API version is intentionally `/api/v2` because the repository already has unversioned MVP endpoints. The v2 layer is the first stable contract for timeline, query, metrics, evidence, and export use cases. If the project later publishes OpenAPI, the generated file should describe both the unversioned compatibility endpoints and `/api/v2`.

## Implemented Prototype Compatibility Additions

The lifecycle-aware prototype keeps v1 endpoints unversioned and adds the minimum fields needed for the current WebUI and future v2 timeline migration.

### GET /api/lifecycle-events

Returns persisted lifecycle facts in chronological order.

Query params:

- `limit`: optional, default `500`, maximum `5000`.

Response:

```json
{
  "events": [
    {
      "rawEventId": 2,
      "sessionId": "9b2f...",
      "eventTs": "2026-05-23T09:05:00Z",
      "lifecycleType": "windows_lock",
      "reason": "manual_lock",
      "activeSessionId": "9b2f...",
      "payload": {}
    }
  ]
}
```

Implemented `lifecycleType` values are `session_start`, `session_stop`, `windows_lock`, `windows_unlock`, `power_suspend`, `power_resume`, `idle_start`, `idle_end`, `capture_unavailable`, `collector_gap`, `session_disconnect`, and `session_reconnect`.

Compatibility payload rule: `payload` is for non-sensitive lifecycle diagnostics only, such as app version, close reason, or stale-session `detectedAt`. It must not contain window titles, raw text, screenshot paths, or external integration identifiers until API v2 redaction policy is enforced.

### GET /api/time-events

The compatibility time-events endpoint now includes lifecycle-aware metadata:

```json
{
  "events": [
    {
      "id": "raw-1",
      "app": "Code.exe",
      "title": "main.rs",
      "kind": "active_window",
      "status": null,
      "sessionId": "9b2f...",
      "startedAt": "2026-05-23T09:00:00Z",
      "endedAt": "2026-05-23T09:05:00Z",
      "durationSeconds": 300
    },
    {
      "id": "lifecycle-2",
      "app": "System",
      "title": "Locked",
      "kind": "lifecycle",
      "status": "windows_lock",
      "sessionId": "9b2f...",
      "startedAt": "2026-05-23T09:05:00Z",
      "endedAt": "2026-05-23T09:20:00Z",
      "durationSeconds": 900
    }
  ]
}
```

Rules:

- Active window intervals do not bridge different `sessionId` values.
- Active window intervals are cut at lifecycle events that make user activity unavailable, including lock, suspend, idle start, capture unavailable, session stop, collector gap, and session disconnect.
- Stale-session `collector_gap` facts are written at the last recorded event timestamp for the stale session, with restart detection time in `payload.detectedAt`; this prevents offline time from being counted as active window time.
- Paired lifecycle intervals are emitted for lock/unlock, suspend/resume, idle start/end, and disconnect/reconnect.
- Frontend active-time summaries treat missing `kind` as legacy `active_window` and exclude `kind = "lifecycle"` from active application totals.

Current limitation: the prototype records lifecycle facts through storage/API methods, stale-session closure, and `serve` shutdown handling. Live Windows message capture for `WM_WTSSESSION_CHANGE`, `WM_POWERBROADCAST`, and `WM_ENDSESSION` remains a next milestone.

## Shared Types

### Time Range

```json
{
  "from": "2026-05-24T00:00:00+08:00",
  "to": "2026-05-25T00:00:00+08:00",
  "timezone": "Asia/Shanghai"
}
```

Rules:

- `from` is inclusive.
- `to` is exclusive.
- Server stores UTC internally but accepts explicit offsets.
- If `timezone` is omitted, the collector default timezone is used.
- Calendar-day queries must use the requested timezone. They must not rely on string-prefix UTC filters such as `LIKE 'YYYY-MM-DD%'`.
- Implementation should parse IANA timezones with `chrono-tz` or equivalent, compute local `[start, end)` boundaries, convert them to UTC, then query indexed UTC timestamps. UI date helpers must use the configured timezone, never `toISOString().slice(0, 10)`.

### Pagination

List endpoints should return cursor pagination when a result can exceed one screen:

```json
{
  "items": [],
  "page": {
    "nextCursor": "cursor_01J...",
    "limit": 100,
    "hasMore": true
  }
}
```

Rules:

- Default `limit` is 100 for detailed evidence lists and 500 for raw/debug lists.
- Maximum `limit` is 5000.
- Cursor order must be stable by `(timestamp, id)`.

### Redaction Policy

```json
{
  "mode": "counts_only",
  "includeWindowTitles": false,
  "includeTextContent": false,
  "includeScreenshots": "metadata_only"
}
```

Allowed values:

- `mode`: `raw`, `redacted`, `counts_only`
- `includeScreenshots`: `none`, `metadata_only`, `thumbnail`

Default is `counts_only` for exports and `redacted` for local WebUI queries.

Evidence content must be served through policy-checked v2 endpoints. API v2 should not return direct `/screenshots/...` paths unless `mode = raw` is permitted by local policy. Redacted responses must omit, hash, or category-map window titles and include the policy that was applied.

### Evidence Reference

```json
{
  "id": "evidence_01J...",
  "kind": "screenshot",
  "timestamp": "2026-05-24T10:31:00+08:00",
  "summary": "VS Code active while editing collector input module",
  "url": "/api/v2/evidence/evidence_01J.../content?redaction=thumbnail",
  "redaction": "thumbnail",
  "policyId": "local_default",
  "redactionApplied": true
}
```

Allowed `kind` values:

- `window_interval`
- `lifecycle_interval`
- `screenshot`
- `input_segment`
- `text_edit`
- `metric_result`
- `external_observation`

## GET /api/v2/timeline

Returns chronological timeline items for the UI.

Query params:

- `from`: ISO timestamp, required.
- `to`: ISO timestamp, required.
- `granularity`: `event`, `minute`, or `hour`; default `event`.
- `limit`: optional page size.
- `cursor`: optional cursor from the previous response.
- `includeEvidence`: boolean; default `true`.
- `redaction`: `raw`, `redacted`, or `counts_only`; default `redacted`.

Example:

```http
GET /api/v2/timeline?from=2026-05-24T09:00:00%2B08:00&to=2026-05-24T12:00:00%2B08:00&granularity=event
```

Response:

```json
{
  "range": {
    "from": "2026-05-24T09:00:00+08:00",
    "to": "2026-05-24T12:00:00+08:00",
    "timezone": "Asia/Shanghai"
  },
  "items": [
    {
      "id": "tl_01J...",
      "startedAt": "2026-05-24T09:04:12+08:00",
      "endedAt": "2026-05-24T09:47:02+08:00",
      "type": "active_window",
      "shape": "TimelineInterval",
      "label": "Code.exe / redacted-title",
      "labelSource": "deterministic",
      "processName": "Code.exe",
      "windowTitle": "[redacted-title]",
      "windowTitleHash": "sha256:...",
      "durationSeconds": 2570,
      "activeSeconds": 2570,
      "confidence": "high",
      "evidence": [
        {
          "id": "shot_01J...",
          "kind": "screenshot",
          "timestamp": "2026-05-24T09:31:00+08:00",
          "summary": "Screenshot thumbnail",
          "url": "/api/v2/evidence/shot_01J.../content?redaction=thumbnail",
          "redaction": "thumbnail",
          "policyId": "local_default",
          "redactionApplied": true
        }
      ],
      "metrics": {
        "inputCorrectionRate": 0.08,
        "contextSwitches": 2
      }
    },
    {
      "id": "tl_01K...",
      "startedAt": "2026-05-24T09:47:02+08:00",
      "endedAt": "2026-05-24T10:02:12+08:00",
      "type": "locked",
      "label": "Windows locked",
      "durationSeconds": 910,
      "activeSeconds": 0,
      "confidence": "high",
      "evidence": []
    }
  ],
  "page": {
    "nextCursor": null,
    "limit": 100,
    "hasMore": false
  },
  "coverage": {
    "activeSeconds": 2570,
    "lockedSeconds": 910,
    "idleSeconds": 0,
    "offlineSeconds": 0,
    "unknownSeconds": 0,
    "suspendedSeconds": 0,
    "blockedSeconds": 0,
    "captureUnavailableSeconds": 0
  }
}
```

Allowed item `type` values:

- `active_window`
- `idle`
- `locked`
- `suspended`
- `collector_offline`
- `blocked`
- `unknown`
- `external_observation`

For `granularity=event`, `items` are `TimelineInterval` objects. For `granularity=minute` or `hour`, `items` are `TimelineBucket` objects with `bucketStart`, `bucketEnd`, `coverage`, `topApps`, `metrics`, and optional representative evidence. Clients must switch on `shape`.

## GET /api/v2/summary/day

Returns a daily summary for WebUI and Notion preview.

Query params:

- `date`: `YYYY-MM-DD`, required.
- `timezone`: optional.
- `redaction`: optional.

Response:

```json
{
  "date": "2026-05-24",
  "timezone": "Asia/Shanghai",
  "coverage": {
    "availableSeconds": 28800,
    "activeSeconds": 16200,
    "idleSeconds": 2400,
    "lockedSeconds": 3600,
    "suspendedSeconds": 0,
    "collectorOfflineSeconds": 600,
    "unknownSeconds": 0,
    "blockedSeconds": 0,
    "captureUnavailableSeconds": 0
  },
  "topApps": [
    {
      "processName": "Code.exe",
      "activeSeconds": 7200,
      "share": 0.444
    }
  ],
  "input": {
    "physicalKeyEvents": 10420,
    "committedGraphemes": 3150,
    "deletedGraphemes": 290,
    "correctionRate": 0.092,
    "pasteShare": 0.18,
    "compositionCommitCount": null,
    "coverageWarnings": ["composition_source_unavailable"]
  },
  "attention": {
    "focusBlockCount": 7,
    "medianFocusBlockSeconds": 1380,
    "contextSwitchesPerActiveHour": 8.4,
    "recoveryMedianSeconds": 420,
    "status": "available"
  },
  "screenshots": {
    "captured": 132,
    "expected": 168,
    "skippedIdle": 18,
    "skippedLocked": 14,
    "blocked": 4,
    "failed": 0
  }
}
```

Metric fields that cannot be computed from available sources must be `null` or omitted with a warning. They must not be returned as fake zeros.

## GET /api/v2/summary/week

Returns week-level statistics.

Query params:

- `weekStart`: `YYYY-MM-DD`, required.
- `timezone`: optional.

Response:

```json
{
  "weekStart": "2026-05-18",
  "timezone": "Asia/Shanghai",
  "days": [
    {
      "date": "2026-05-18",
      "activeSeconds": 14400,
      "focusBlockCount": 5,
      "contextSwitchesPerActiveHour": 9.1,
      "correctionRate": 0.11
    }
  ],
  "weeklyTotals": {
    "activeSeconds": 81200,
    "lockedSeconds": 14400,
    "collectorOfflineSeconds": 1200
  },
  "patterns": [
    {
      "id": "pattern_morning_focus",
      "label": "Morning focus block",
      "description": "Most sustained focus occurred between 09:00 and 11:00.",
      "confidence": "medium",
      "evidenceIds": ["metric_01J..."]
    }
  ]
}
```

## POST /api/v2/query

Runs a structured local query. This is the stable substrate for WebUI filters, custom analysis, exports, and future chat.

Request:

```json
{
  "range": {
    "from": "2026-05-24T09:00:00+08:00",
    "to": "2026-05-24T18:00:00+08:00",
    "timezone": "Asia/Shanghai"
  },
  "filters": {
    "sourceIds": ["source_windows_collector"],
    "entityIds": ["entity_code_app"],
    "observationKinds": ["window_interval", "text_edit"],
    "processNames": ["Code.exe", "chrome.exe"],
    "intervalTypes": ["active_window"],
    "includeLifecycle": true,
    "textConfidence": ["high", "medium"],
    "editKind": ["insert", "replace"],
    "method": ["uia_diff", "ime_hook"]
  },
  "joinPolicy": {
    "externalObservations": "explicit",
    "timeWindowSeconds": 300,
    "minimumConfidence": "medium"
  },
  "groupBy": ["hour", "processName"],
  "metrics": [
    { "id": "active_seconds" },
    { "id": "context_switch_count" },
    { "id": "committed_graphemes" },
    { "id": "deleted_graphemes" },
    { "id": "correction_rate", "version": 1, "computeMode": "cached" }
  ],
  "evidence": {
    "include": true,
    "limitPerGroup": 3
  },
  "redaction": {
    "mode": "redacted",
    "includeWindowTitles": true,
    "includeTextContent": false,
    "includeScreenshots": "thumbnail"
  }
}
```

Response:

```json
{
  "queryId": "query_01J...",
  "generatedAt": "2026-05-24T18:01:00+08:00",
  "rows": [
    {
      "group": {
        "hour": "2026-05-24T09:00:00+08:00",
        "processName": "Code.exe"
      },
      "metrics": {
        "active_seconds": 2740,
        "context_switch_count": 3,
        "committed_graphemes": 640,
        "deleted_graphemes": 42,
        "correction_rate": 0.066
      },
      "evidence": [
        {
          "id": "shot_01J...",
          "kind": "screenshot",
          "timestamp": "2026-05-24T09:31:00+08:00",
          "summary": "Screenshot thumbnail",
          "url": "/api/v2/evidence/shot_01J.../content?redaction=thumbnail",
          "redaction": "thumbnail",
          "policyId": "local_default",
          "redactionApplied": true
        }
      ]
    }
  ],
  "warnings": [],
  "metricStatus": {
    "composition_to_commit_ratio": "unavailable"
  }
}
```

Validation rules:

- Reject ranges longer than 31 days unless `allowLargeRange=true`.
- Reject unknown metric ids with HTTP 400 and a list of allowed metrics.
- Reject `raw` redaction mode unless local user config allows raw queries.
- Reject queries that request text content from blocked or password-classified windows.
- External observations may be joined only through analyzer/query layers using explicit `joinPolicy` rules.
- Return warnings when coverage is incomplete because of lock/offline/unknown intervals.

## GET /api/v2/metrics

Returns the metric catalog exposed by the query layer.

Response:

```json
{
  "metrics": [
    {
      "id": "correction_rate",
      "version": 1,
      "description": "Deleted graphemes divided by inserted graphemes.",
      "unit": "ratio",
      "defaultGranularity": "hour",
      "requiredSources": ["text_edit_events"],
      "privacyLevel": "derived",
      "formula": "deleted_graphemes / max(inserted_graphemes, 1)",
      "sourceSchemaVersions": {
        "text_edit_events": 1
      },
      "cachePolicy": "cacheable",
      "invalidatedAt": null
    }
  ]
}
```

## GET /api/v2/sources

Returns local and imported source metadata.

Response:

```json
{
  "sources": [
    {
      "id": "source_windows_collector",
      "kind": "windows_collector",
      "displayName": "Windows Collector",
      "privacyLevel": "normal",
      "lastImportedAt": null
    },
    {
      "id": "source_notion_principle",
      "kind": "notion_principle",
      "displayName": "Notion Principle",
      "privacyLevel": "sensitive",
      "lastImportedAt": "2026-05-24T08:00:00+08:00"
    }
  ]
}
```

## GET /api/v2/openapi.json

Returns the machine-readable API contract. Initial implementation can be hand-authored, but it should be generated from route/DTO definitions before the API is treated as stable.

## GET /api/v2/evidence/:id

Returns one evidence item and its provenance.

Response:

```json
{
  "id": "shot_01J...",
  "kind": "screenshot",
  "timestamp": "2026-05-24T09:31:00+08:00",
  "payload": {
    "filePath": "2026-05-24/09-31.jpg",
    "width": 640,
    "height": 360,
    "processName": "Code.exe",
    "windowTitleHash": "sha256:..."
  },
  "provenance": {
    "sourceTable": "screenshot_thumbnails",
    "sourceId": 128,
    "captureSessionId": "session_01J...",
    "configHash": "sha256:..."
  },
  "redaction": {
    "mode": "redacted",
    "textContentIncluded": false,
    "policyId": "local_default",
    "redactionApplied": true
  }
}
```

## GET /api/v2/evidence/:id/content

Returns binary or text evidence content after applying the requested redaction policy.

Query params:

- `redaction`: `metadata_only`, `thumbnail`, `raw`, or `counts_only`.

Rules:

- `raw` content requires local raw-query permission.
- `text_edit` evidence must enforce `text_capture` blocker, password-field, and field-classification policy again at read time.
- Blocked evidence returns HTTP 403 with the standard error shape.

## POST /api/v2/export/notion/preview

Builds Notion-ready payloads without writing to Notion.

Request:

```json
{
  "targetId": "notion_default",
  "exportType": "daily_diary",
  "query": {
    "date": "2026-05-24",
    "redaction": "redacted"
  },
  "selectedEvidenceIds": ["shot_01J..."],
  "redaction": {
    "mode": "redacted",
    "includeWindowTitles": false,
    "includeTextContent": false,
    "includeScreenshots": "metadata_only"
  }
}
```

Response:

```json
{
  "previewId": "preview_01J...",
  "expiresAt": "2026-05-24T19:00:00+08:00",
  "redactionPolicyId": "local_default",
  "target": "notion_principle",
  "exportType": "daily_diary",
  "items": [
    {
      "localId": "export_item_01J...",
      "title": "INDEX-20260524 | Daily Diary",
      "kind": "daily_diary",
      "properties": {
        "Index ID": "INDEX-20260524",
        "Index Standard": "INDEXv1",
        "Capture Date": "2026-05-24",
        "Daily Diary": "INDEX-20260524 | Daily Diary",
        "Rawness State": "Raw",
        "Material Type": "Daily Diary",
        "Needs Human Review": true
      },
      "markdown": "# Daily Diary\n\nActive time: 4h 30m\nLocked time: 1h 00m\n\n## Highlights\n- Code.exe dominated the morning focus block.",
      "provenance": {
        "query": {
          "date": "2026-05-24",
          "redaction": "redacted"
        },
        "sourceMetricIds": ["metric_01J..."],
        "sourceEvidenceIds": ["shot_01J..."]
      }
    }
  ],
  "warnings": [
    "Window titles are redacted by policy.",
    "Collector was offline for 10 minutes."
  ]
}
```

Generated Raw Materials are daily raw capture units, not Tasks. Task candidates are drafts only and must not create durable Tasks without review. Raw Material titles must follow `INDEX-YYYYMMDD-NNN | Material Type | Short Title`; durable Task drafts must follow `TASK-YYYY-NNN | Action-oriented title` only after review.

Preview payloads should be persisted in an export preview cache or as `export_jobs(status='preview')`; `POST /api/v2/export/notion/run` must reject expired previews with HTTP 409.

## POST /api/v2/export/notion/run

Creates or updates Notion records after preview confirmation.

Request:

```json
{
  "previewId": "preview_01J...",
  "targetId": "notion_default",
  "mode": "create_draft",
  "confirmedItemIds": ["export_item_01J..."]
}
```

Allowed `mode` values:

- `create_draft`
- `update_existing_draft`
- `write_markdown_only`

Response:

```json
{
  "jobId": "export_job_01J...",
  "status": "queued",
  "itemsAccepted": 1
}
```

## Error Shape

All v2 endpoints return this error shape:

```json
{
  "error": {
    "code": "invalid_metric",
    "message": "Unknown metric id: foo",
    "details": {
      "allowedMetrics": ["active_seconds", "correction_rate"]
    }
  }
}
```

HTTP status mapping:

- 400: invalid request shape, invalid time range, unknown metric.
- 403: raw/redacted policy violation.
- 404: evidence or preview id not found.
- 409: export preview expired or Notion target disabled.
- 500: internal collector error.

## Metrics Catalog V1

Supported initial metric ids:

- `active_seconds`
- `idle_seconds`
- `locked_seconds`
- `suspended_seconds`
- `collector_offline_seconds`
- `unknown_seconds`
- `blocked_seconds`
- `capture_unavailable_seconds`
- `context_switch_count`
- `context_switches_per_active_hour`
- `focus_block_count`
- `median_focus_block_seconds`
- `physical_key_count`
- `committed_graphemes`
- `deleted_graphemes`
- `correction_rate`
- `paste_share`
- `input_burst_count`
- `hesitation_seconds`
- `rework_graphemes`
- `interrupted_composition_count`
- `candidate_select_count`
- `input_method_transition_count`
- `composition_commit_count`
- `composition_to_commit_ratio`
- `screenshot_expected_count`
- `screenshot_captured_count`
- `screenshot_skipped_idle_count`
- `screenshot_skipped_locked_count`
- `screenshot_blocked_count`
- `screenshot_failed_count`

Each metric implementation must declare source tables, formula, version, cache policy, source schema versions, and provenance fields before it is exposed through `/api/v2/query`.
