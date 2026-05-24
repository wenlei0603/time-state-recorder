param(
    [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

function Assert-FileContains {
    param(
        [string]$Path,
        [string]$Pattern
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing expected file: $Path"
    }

    $content = Get-Content -LiteralPath $Path -Raw
    if ($content -notmatch $Pattern) {
        throw "Expected $Path to contain pattern: $Pattern"
    }
}

$startBat = Join-Path $Root "Start Time State Recorder.bat"
$stopBat = Join-Path $Root "Stop Time State Recorder.bat"
$startScript = Join-Path $Root "scripts\start-user.ps1"
$stopScript = Join-Path $Root "scripts\stop-user.ps1"
$webServer = Join-Path $Root "scripts\web-server.mjs"

Assert-FileContains $startBat "start-user\.ps1"
Assert-FileContains $stopBat "stop-user\.ps1"
Assert-FileContains $startScript "ApiPort = 4317"
Assert-FileContains $startScript "WebPort = 5173"
Assert-FileContains $startScript "Invoke-LocalHttp"
Assert-FileContains $startScript "Start-Process"
Assert-FileContains $startScript "web-server\.mjs"
Assert-FileContains $startScript "npm run build"
Assert-FileContains $startScript "bin\\tsr-collector\.exe"
Assert-FileContains $startScript "Get-BlockerConfigPath"
Assert-FileContains $stopScript "tsr-collector"
Assert-FileContains $stopScript "vite"
Assert-FileContains $stopScript "web-server\.mjs"
Assert-FileContains $stopScript "/api/shutdown"
Assert-FileContains $stopScript "PostAsync"
Assert-FileContains $stopScript "WaitForExit"
Assert-FileContains $webServer "createServer"
Assert-FileContains $webServer "/api"
Assert-FileContains $webServer "/screenshots"

Write-Host "Launcher scripts are present and contain the expected startup contract."
