param(
    [string]$Version = "",
    [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

if (-not $Version) {
    $packageJson = Get-Content -LiteralPath (Join-Path $Root "package.json") -Raw | ConvertFrom-Json
    $Version = [string]$packageJson.version
}

$releaseRoot = Join-Path $Root "output\release\time-state-recorder-v$Version"
$zipPath = Join-Path $Root "output\release\time-state-recorder-v$Version-windows-x64.zip"
$collectorExe = Join-Path $Root "target\x86_64-pc-windows-gnullvm\release\tsr-collector.exe"
$distIndex = Join-Path $Root "dist\index.html"

if (-not (Test-Path -LiteralPath $collectorExe -PathType Leaf)) {
    throw "Missing release collector executable: $collectorExe"
}
if (-not (Test-Path -LiteralPath $distIndex -PathType Leaf)) {
    throw "Missing WebUI build: $distIndex"
}

if (Test-Path -LiteralPath $releaseRoot) {
    Remove-Item -LiteralPath $releaseRoot -Recurse -Force
}
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}

New-Item -ItemType Directory -Force -Path `
    (Join-Path $releaseRoot "bin"), `
    (Join-Path $releaseRoot "scripts"), `
    (Join-Path $releaseRoot "dist") | Out-Null

Copy-Item -LiteralPath $collectorExe -Destination (Join-Path $releaseRoot "bin\tsr-collector.exe")
Copy-Item -LiteralPath (Join-Path $Root "Start Time State Recorder.bat") -Destination $releaseRoot
Copy-Item -LiteralPath (Join-Path $Root "Stop Time State Recorder.bat") -Destination $releaseRoot
Copy-Item -LiteralPath (Join-Path $Root "README.md") -Destination $releaseRoot
Copy-Item -LiteralPath (Join-Path $Root "collector\blocker_config.json") -Destination (Join-Path $releaseRoot "blocker_config.json")
Copy-Item -LiteralPath (Join-Path $Root "scripts\start-user.ps1") -Destination (Join-Path $releaseRoot "scripts\start-user.ps1")
Copy-Item -LiteralPath (Join-Path $Root "scripts\stop-user.ps1") -Destination (Join-Path $releaseRoot "scripts\stop-user.ps1")
Copy-Item -LiteralPath (Join-Path $Root "scripts\web-server.mjs") -Destination (Join-Path $releaseRoot "scripts\web-server.mjs")
Copy-Item -Path (Join-Path $Root "dist\*") -Destination (Join-Path $releaseRoot "dist") -Recurse -Force

@"
Time State Recorder v$Version

Quick start:
1. Extract this zip.
2. Double-click Start Time State Recorder.bat.
3. The app opens at http://127.0.0.1:5173.
4. Double-click Stop Time State Recorder.bat to stop it.

Runtime requirement:
- Node.js must be available on PATH for the local WebUI server.
- No Visual Studio or Rust toolchain is needed for this packaged build.
"@ | Set-Content -LiteralPath (Join-Path $releaseRoot "RELEASE_README.txt") -Encoding ASCII

Compress-Archive -LiteralPath $releaseRoot -DestinationPath $zipPath -CompressionLevel Optimal

Get-Item -LiteralPath $zipPath
