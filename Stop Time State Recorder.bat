@echo off
setlocal

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\stop-user.ps1"
if errorlevel 1 (
    echo.
    echo Stop failed. See logs\launcher.err.log for details.
    echo.
    pause
)

endlocal
