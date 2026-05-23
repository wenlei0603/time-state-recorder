@echo off
if exist "target\release\tsr-collector.exe" (
    echo Starting collector (release)...
    target\release\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else if exist "target\debug\tsr-collector.exe" (
    echo Starting collector (debug)...
    target\debug\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else (
    echo Collector binary not found. Build first:
    echo   cargo build --release -p tsr-collector
    pause
)
