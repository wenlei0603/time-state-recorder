param(
    [int]$ApiPort = 4317,
    [int]$WebPort = 5173,
    [switch]$NoBrowser
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Net.Http

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LogDir = Join-Path $Root "logs"
$StateDir = Join-Path $Root ".launcher"
$CollectorPidFile = Join-Path $StateDir "collector.pid"
$WebPidFile = Join-Path $StateDir "webui.pid"
$LauncherLog = Join-Path $LogDir "launcher.out.log"
$LauncherErr = Join-Path $LogDir "launcher.err.log"
$CollectorOut = Join-Path $LogDir "collector.out.log"
$CollectorErr = Join-Path $LogDir "collector.err.log"
$WebOut = Join-Path $LogDir "webui.out.log"
$WebErr = Join-Path $LogDir "webui.err.log"
$WebServerScript = Join-Path $Root "scripts\web-server.mjs"
$ApiUrl = "http://127.0.0.1:$ApiPort/api/health"
$WebUrl = "http://127.0.0.1:$WebPort/"

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

function Invoke-LocalHttp {
    param(
        [string]$Url,
        [int]$TimeoutSec = 5
    )

    $client = [System.Net.Http.HttpClient]::new()
    try {
        $client.Timeout = [TimeSpan]::FromSeconds($TimeoutSec)
        $response = $client.GetAsync($Url).GetAwaiter().GetResult()
        return $response.IsSuccessStatusCode
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

function Wait-LocalHttp {
    param(
        [string]$Url,
        [int]$TimeoutSec
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if (Invoke-LocalHttp -Url $Url -TimeoutSec 5) {
            return $true
        }
        Start-Sleep -Seconds 2
    }
    return $false
}

function Get-ProcessFromPidFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }

    $pidText = (Get-Content -LiteralPath $Path -Raw).Trim()
    if ($pidText -notmatch "^\d+$") {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
        return $null
    }

    $proc = Get-Process -Id ([int]$pidText) -ErrorAction SilentlyContinue
    if ($null -eq $proc) {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    }
    return $proc
}

function Add-LocalToolPaths {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path -LiteralPath $cargoBin -PathType Container) {
        $env:PATH = "$cargoBin;$env:PATH"
    }

    if (-not (Get-Command "x86_64-w64-mingw32-clang.exe" -ErrorAction SilentlyContinue)) {
        $wingetRoot = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe"
        if (Test-Path -LiteralPath $wingetRoot -PathType Container) {
            $mingwBin = Get-ChildItem $wingetRoot -Recurse -Filter "x86_64-w64-mingw32-clang.exe" -ErrorAction SilentlyContinue |
                Select-Object -First 1 -ExpandProperty DirectoryName
            if ($mingwBin) {
                $env:PATH = "$mingwBin;$env:PATH"
            }
        }
    }
}

function Get-CollectorExe {
    $candidates = @(
        "bin\tsr-collector.exe",
        "target\x86_64-pc-windows-gnullvm\release\tsr-collector.exe",
        "target\x86_64-pc-windows-gnullvm\debug\tsr-collector.exe",
        "target\release\tsr-collector.exe",
        "target\debug\tsr-collector.exe"
    )

    foreach ($candidate in $candidates) {
        $path = Join-Path $Root $candidate
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            return $path
        }
    }
    return $null
}

function Get-BlockerConfigPath {
    $candidates = @(
        "blocker_config.json",
        "collector\blocker_config.json"
    )

    foreach ($candidate in $candidates) {
        $path = Join-Path $Root $candidate
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            return $path
        }
    }

    return (Join-Path $Root "blocker_config.json")
}

function Ensure-CollectorExe {
    $exe = Get-CollectorExe
    if ($exe) {
        return $exe
    }

    Add-LocalToolPaths
    $cargo = Get-Command "cargo.exe" -ErrorAction SilentlyContinue
    if (-not $cargo) {
        throw "Collector binary is missing and cargo.exe was not found. Install Rust, then run cargo build -p tsr-collector."
    }

    Write-LauncherLog "Collector binary not found; building tsr-collector..."
    Push-Location $Root
    try {
        & $cargo.Source build -p tsr-collector *> (Join-Path $LogDir "cargo-build.log")
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed. See logs\cargo-build.log."
        }
    } finally {
        Pop-Location
    }

    $exe = Get-CollectorExe
    if (-not $exe) {
        throw "cargo build completed but tsr-collector.exe was not found."
    }
    return $exe
}

function Ensure-NodeDependencies {
    if (Test-Path -LiteralPath (Join-Path $Root "node_modules\vite\bin\vite.js") -PathType Leaf) {
        return
    }

    $npm = Get-Command "npm.cmd" -ErrorAction SilentlyContinue
    if (-not $npm) {
        throw "node_modules is missing and npm.cmd was not found. Install Node.js, then run npm install."
    }

    Write-LauncherLog "node_modules not found; running npm install..."
    Push-Location $Root
    try {
        & $npm.Source install *> (Join-Path $LogDir "npm-install.log")
        if ($LASTEXITCODE -ne 0) {
            throw "npm install failed. See logs\npm-install.log."
        }
    } finally {
        Pop-Location
    }
}

function Ensure-WebBuild {
    $indexHtml = Join-Path $Root "dist\index.html"
    if (Test-Path -LiteralPath $indexHtml -PathType Leaf) {
        return
    }

    Ensure-NodeDependencies
    $npm = Get-Command "npm.cmd" -ErrorAction SilentlyContinue
    if (-not $npm) {
        throw "dist is missing and npm.cmd was not found. Install Node.js, then run npm install and npm run build."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $Root "package.json") -PathType Leaf)) {
        throw "WebUI build is missing from this release package. Re-download the release zip or run from a source checkout."
    }

    Write-LauncherLog "WebUI build not found; running npm run build..."
    Push-Location $Root
    try {
        & $npm.Source run build *> (Join-Path $LogDir "npm-build.log")
        if ($LASTEXITCODE -ne 0) {
            throw "npm run build failed. See logs\npm-build.log."
        }
    } finally {
        Pop-Location
    }
}

try {
    Write-LauncherLog "Starting Time State Recorder..."

    if (-not (Invoke-LocalHttp -Url $ApiUrl -TimeoutSec 2)) {
        $collectorProcess = Get-ProcessFromPidFile -Path $CollectorPidFile
        if ($null -eq $collectorProcess) {
            $collectorExe = Ensure-CollectorExe
            $collectorArgs = @(
                "serve",
                "--db", "data/local.sqlite3",
                "--addr", "127.0.0.1:$ApiPort",
                "--poll-ms", "1000",
                "--blocker-config", (Get-BlockerConfigPath)
            )
            $collectorProcess = Start-Process -FilePath $collectorExe -ArgumentList $collectorArgs -WorkingDirectory $Root -WindowStyle Hidden -RedirectStandardOutput $CollectorOut -RedirectStandardError $CollectorErr -PassThru
            Set-Content -LiteralPath $CollectorPidFile -Value $collectorProcess.Id -Encoding ASCII
            Write-LauncherLog "Started collector process $($collectorProcess.Id)."
        } else {
            Write-LauncherLog "Collector process $($collectorProcess.Id) is already running."
        }

        if (-not (Wait-LocalHttp -Url $ApiUrl -TimeoutSec 40)) {
            throw "Collector API did not become ready at $ApiUrl. See logs\collector.err.log."
        }
    } else {
        Write-LauncherLog "Collector API is already ready."
    }

    if (-not (Invoke-LocalHttp -Url $WebUrl -TimeoutSec 2)) {
        $webProcess = Get-ProcessFromPidFile -Path $WebPidFile
        if ($null -eq $webProcess) {
            Ensure-WebBuild
            $node = Get-Command "node.exe" -ErrorAction SilentlyContinue
            if (-not $node) {
                throw "node.exe was not found. Install Node.js, then run npm install."
            }

            if (-not (Test-Path -LiteralPath $WebServerScript -PathType Leaf)) {
                throw "Web server script not found: $WebServerScript"
            }

            $webArgs = @($WebServerScript, "--host", "127.0.0.1", "--port", "$WebPort", "--api", "http://127.0.0.1:$ApiPort")
            $webProcess = Start-Process -FilePath $node.Source -ArgumentList $webArgs -WorkingDirectory $Root -WindowStyle Hidden -RedirectStandardOutput $WebOut -RedirectStandardError $WebErr -PassThru
            Set-Content -LiteralPath $WebPidFile -Value $webProcess.Id -Encoding ASCII
            Write-LauncherLog "Started WebUI process $($webProcess.Id)."
        } else {
            Write-LauncherLog "WebUI process $($webProcess.Id) is already running."
        }

        if (-not (Wait-LocalHttp -Url $WebUrl -TimeoutSec 30)) {
            throw "WebUI did not become ready at $WebUrl. See logs\webui.err.log."
        }
    } else {
        Write-LauncherLog "WebUI is already ready."
    }

    if (-not $NoBrowser) {
        Start-Process $WebUrl | Out-Null
        Write-LauncherLog "Opened $WebUrl"
    }

    Write-LauncherLog "Time State Recorder is ready."
    exit 0
} catch {
    Write-LauncherError $_.Exception.Message
    exit 1
}
