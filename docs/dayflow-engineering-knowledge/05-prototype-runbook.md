# Prototype Runbook

## Purpose

Run and inspect the current Windows-first Dayflow prototype: a local collector, local API, and Today Flow Board with redacted/raw evidence controls.

## Start The Prototype

From the repository root:

```powershell
npm run collector
```

In a second shell:

```powershell
npm run dev
```

Open the Vite URL, usually `http://127.0.0.1:5173/`.

## Manual Verification

1. Confirm the first screen is `Today Flow Board`.
2. In redacted mode, confirm window titles show `Hidden in redacted mode`.
3. Confirm redacted Today does not request raw text segments or screenshot rows.
4. Switch to `Raw`.
5. Confirm window titles can appear in Today buckets and the evidence drawer.
6. With `Screenshots` enabled, confirm overlapping screenshot thumbnails can appear in the Today evidence drawer.
7. Switch back to `Redacted` and confirm screenshot thumbnails disappear.
8. Open `Daily Tracking` in raw mode to inspect the full screenshot timeline.
9. Open `Input Activity` in raw mode to inspect text segments.
10. Open `Dashboard` and `Timeline` to confirm titles remain redacted until Raw is selected.

## Expected Prototype Boundaries

- The collector still uses polling for foreground windows.
- Query Date is UTC-aligned, not a local natural-day query.
- Backend storage still contains raw titles, text segments, and screenshot files.
- Redaction is currently a frontend evidence-loading and rendering policy.
- Chinese IME committed text capture is not solved.

## Verification Commands

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
npm test -- --run
npm run build
git diff --check
```
