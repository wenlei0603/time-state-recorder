@echo off
setlocal

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\start-user.ps1"
if errorlevel 1 (
    echo.
    echo Startup failed. See logs\launcher.err.log for details.
    echo.
    pause
)

endlocal
