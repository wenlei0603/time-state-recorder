$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$artifactPath = Join-Path $repoRoot 'reports/notion-daily-archive-smoke.json'

$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
    $cargoExe = $cargoCommand.Source
} else {
    $cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (-not (Test-Path -LiteralPath $cargoExe)) {
        throw "Cargo was not found on PATH or at $cargoExe"
    }
}

& $cargoExe run -p tsr-collector -- notion-daily-archive-smoke --artifact $artifactPath
