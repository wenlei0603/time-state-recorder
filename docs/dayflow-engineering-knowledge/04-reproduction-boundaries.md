# Reproduction Boundaries

## Current Dayflow-Like Coverage

The current repository already reproduces several Dayflow-style primitives:

- continuous local recording;
- active app/window timeline;
- screenshot-based visual evidence;
- keyboard/input activity summary;
- daily flow board, dashboard, and timeline review;
- evidence drawer over derived time-flow buckets;
- sample data fallback when collector is unavailable;
- redacted/raw UI switch for sensitive evidence.

The current implementation is best treated as a Dayflow-inspired local recorder prototype, not a complete Dayflow reproduction.

## Implemented Architecture Baseline

```text
Windows collector
  -> SQLite raw/specialized tables
  -> local API v1
  -> React review UI
```

This is enough for the first reproduction baseline:

- collect real local activity;
- persist raw facts;
- query daily activity;
- view evidence and summaries locally.

## Not Yet Implemented Dayflow Layers

```text
Policy-aware evidence layer
  -> API v2 timeline/query contract
  -> semantic daily summary
  -> richer evidence drawer
  -> query/chat explorer
  -> weekly review
  -> cleanup/export workflow
```

Missing or only documented:

- `/api/v2/timeline`
- `/api/v2/summary/day`
- `/api/v2/query`
- evidence drawer with raw-source drilldown
- WeeklyReviewView
- QueryExplorerView
- metric registry
- Notion/export path
- cleanup controls
- backend redaction policy service

`docs/api/next-query-api.md` is an architecture contract, not implemented code.

## Important Contract Boundaries

### API v1 Is Real; API v2 Is Planned

Real routes are in `collector/src/api.rs`:

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
- `POST /api/shutdown`

Current v1 caveats:

- `/api/window-events` returns a raw array, not `{ events: [...] }`.
- `/api/time-events`, `/api/input-events`, and `/api/text-segments` return wrapped objects.
- Date filters use string-prefix UTC date matching in storage.
- The frontend computes today's date in local browser time.
- This can drift for Asia/Shanghai natural-day queries.

### Frontend Redaction Is Not Full Backend Privacy

The WebUI avoids fetching/rendering raw text and screenshot rows in redacted mode, but the backend still stores:

- plaintext `text_segments.text_content`;
- JPEG screenshots on disk;
- raw window titles where available.

For real Dayflow reproduction with privacy guarantees, backend policy enforcement must move closer to storage/API/evidence retrieval.

### Input Capture Is Not Text-Edit Capture

Current implementation captures physical keyboard-derived characters and local buffer edits.

It is not yet:

- IME committed text capture;
- UI Automation diff;
- TSF-based text service capture;
- paste capture;
- replacement capture;
- grapheme-aware cursor editing;
- text redaction pipeline.

This is the largest gap for Chinese-first Dayflow reproduction.

## Reproduction Risks

| Risk | Current Evidence | Why It Matters |
| --- | --- | --- |
| Polling misses rapid focus changes | `GetForegroundWindow` polling loop | Dayflow-style timeline may undercount short app switches |
| Blocker audit privacy surface | `blocker_hits.actual_value` stores matched values | Blocked skip metadata is redacted, but audit rows can still contain sensitive titles |
| Timezone drift | v1 date queries use `LIKE 'YYYY-MM-DD%'` | Local daily review can mismatch UTC-stored timestamps |
| Text privacy exposure | plaintext text segments | Raw local DB contains sensitive typed content |
| Screenshot privacy exposure | JPEG files on disk | Raw evidence exists outside SQLite policy boundary |
| Text blocker not enforced | `text_capture` config exists but input path does not call blocker | Cannot claim text privacy blocker coverage |
| IME gap | Raw Input plus `ToUnicodeEx` | Chinese committed text may be wrong or absent |
| Lifecycle gap | no Windows session/power message capture yet | Lock/suspend/offline time can be incomplete |
| Health staleness | screenshot success does not clear all last skip details | Collector monitor may show old skip context after later success |
| API drift | README, v2 docs, real route handlers, frontend clients are separate | Future reproduction can break unless contracts are centralized |
| Toolchain ambiguity | README says LLVM-MinGW MSVCRT; runbook says UCRT | New machines may fail to build consistently |
| WAL backup risk | SQLite WAL enabled | Copying only `local.sqlite3` can miss recent data |

## Bottom-Up Aggregation Notes

The architecture can be understood from bottom to top:

1. OS adapters produce low-level facts.
2. Domain models normalize facts into JSON/SQLite shapes.
3. Store persists raw and specialized records.
4. Derivation layer turns facts into review intervals and summaries.
5. API exposes local contracts.
6. Frontend clients validate contracts.
7. UI model computes Dayflow-like review metrics.
8. Views render daily review, timeline, evidence, input, and health.

This bottom-up chain is the reproduction skeleton. Any future Dayflow clone work should preserve this separation:

- collectors should not own UI semantics;
- storage should preserve auditable raw facts;
- query/derivation should be reproducible;
- UI should consume explicit API contracts;
- privacy policy should be enforced before evidence is exposed.

## Next Knowledge Gaps To Explore

These are not required for the current acceptance round, but should be the next decomposition targets:

1. Compare current UI with Dayflow interaction flows screen by screen.
2. Design API v2 timeline/query schema from implemented v1 data.
3. Decompose input capture upgrade path for Chinese IME and text edits.
4. Define backend privacy policy and evidence redaction boundaries.
5. Trace test coverage against architecture invariants.
6. Build a reproduction roadmap from MVP primitives to Dayflow workflows.
