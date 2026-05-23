# Windows Collector Runbook

## Purpose

Run the first real Time State Recorder collector: a local Rust CLI that samples the Windows foreground window, stores `window_focus` events in SQLite, and serves JSON to the WebUI.

## Prerequisites

- Rust stable toolchain with `x86_64-pc-windows-gnullvm`.
- LLVM MinGW UCRT tools available on `PATH`, especially `x86_64-w64-mingw32-clang`.
- Node.js/npm for the WebUI.

Install commands used for this workspace:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnullvm --profile minimal
winget install --id MartinStorsjo.LLVM-MinGW.UCRT --silent --accept-package-agreements --accept-source-agreements
```

Restart the shell after installing LLVM MinGW, or add its `bin` directory to `PATH` for the current session.

## Commands

Sample the current foreground window:

```powershell
cargo run -p tsr-collector -- sample-once
```

Record foreground-window changes for a short window:

```powershell
cargo run -p tsr-collector -- record --seconds 30 --poll-ms 1000 --db data/local.sqlite3
```

Run the collector API:

```powershell
cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317 --poll-ms 1000
```

Run the WebUI in a second shell:

```powershell
npm run dev
```

The WebUI proxies `/api/*` to `http://127.0.0.1:4317`.

## API

- `GET /api/health` returns collector status.
- `GET /api/window-events?limit=500` returns the latest raw `window_focus` rows joined with window metadata, ordered chronologically.
- `GET /api/time-events?limit=500` returns the latest interval-shaped events consumed by the WebUI, ordered chronologically.

The collector does not enable permissive CORS. Browser UI access should go through the Vite `/api/*` proxy or a future same-origin app shell.

## Verification

```powershell
cargo test -p tsr-collector
npm test
npm run build
```

Manual checks:

- `sample-once` returns a JSON object with `processName`, `windowTitle`, and `captureStatus`.
- `record` creates `data/local.sqlite3`.
- `serve` responds on `/api/time-events`.
- WebUI status changes to `Connected` and shows real application rows.

## Limits

- This version uses polling, not `SetWinEventHook`.
- `--poll-ms` must be at least `100` to avoid a tight polling loop.
- It records window titles; future privacy controls should redact or disable titles by app/window policy.
- The final open interval has no `endedAt` until another focus event is recorded.
