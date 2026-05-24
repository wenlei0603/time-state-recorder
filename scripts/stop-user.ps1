param()

$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LogDir = Join-Path $Root "logs"
$StateDir = Join-Path $Root ".launcher"
$CollectorPidFile = Join-Path $StateDir "collector.pid"
$WebPidFile = Join-Path $StateDir "webui.pid"
$LauncherLog = Join-Path $LogDir "launcher.out.log"
$LauncherErr = Join-Path $LogDir "launcher.err.log"

New-Item -ItemType Directory -Force -Path $LogDir, $StateDir | Out-Null

function Write-LauncherLog {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message
    Add-Content -LiteralPath $LauncherLog -Value $line
    Write-Host $Message
}

function Write-LauncherError {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message
    Add-Content -LiteralPath $LauncherErr -Value $line
    Write-Error $Message
}

function Stop-PidFileProcess {
    param(
        [string]$Path,
        [string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return
    }

    $pidText = (Get-Content -LiteralPath $Path -Raw).Trim()
    if ($pidText -match "^\d+$") {
        $proc = Get-Process -Id ([int]$pidText) -ErrorAction SilentlyContinue
        if ($proc) {
            Stop-Process -Id $proc.Id -Force
            Write-LauncherLog "Stopped $Label process $($proc.Id)."
        }
    }

    Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
}

try {
    Write-LauncherLog "Stopping Time State Recorder..."
    Stop-PidFileProcess -Path $WebPidFile -Label "WebUI"
    Stop-PidFileProcess -Path $CollectorPidFile -Label "collector"

    $ownedProcesses = Get-CimInstance Win32_Process | Where-Object {
        $_.CommandLine -and
        $_.CommandLine -like "*$Root*" -and
        (
            $_.CommandLine -like "*tsr-collector*" -or
            $_.CommandLine -like "*vite*" -or
            $_.CommandLine -like "*web-server.mjs*"
        )
    }

    foreach ($proc in $ownedProcesses) {
        Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
        Write-LauncherLog "Stopped project process $($proc.ProcessId)."
    }

    Write-LauncherLog "Time State Recorder is stopped."
    exit 0
} catch {
    Write-LauncherError $_.Exception.Message
    exit 1
}
