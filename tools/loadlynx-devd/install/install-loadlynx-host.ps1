param(
    [string]$Version = "latest",
    [string]$InstallDir = "",
    [switch]$Force,
    [switch]$DryRun,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RepoUrl = "https://github.com/IvanLi-CN/loadlynx"

function Show-Usage {
    @"
Install LoadLynx host tools for the current user.

Usage:
  powershell -ExecutionPolicy Bypass -File install-loadlynx-host.ps1 [-Version <tag>] [-InstallDir <dir>] [-Force] [-DryRun]

Defaults:
  -Version latest
  -InstallDir %LOCALAPPDATA%\Programs\LoadLynx\bin
"@
}

function Fail($Message) {
    Write-Error $Message
    exit 1
}

if ($Help) {
    Show-Usage
    exit 0
}

if (-not [Environment]::Is64BitOperatingSystem) {
    Fail "unsupported Windows architecture; expected x86_64"
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\LoadLynx\bin"
}

$Archive = "loadlynx-host-tools-windows-x86_64.tar.gz"
if ($Version -eq "latest") {
    $BaseUrl = "$RepoUrl/releases/latest/download"
} else {
    $BaseUrl = "$RepoUrl/releases/download/$Version"
}
$ArchiveUrl = "$BaseUrl/$Archive"
$ChecksumUrl = "$BaseUrl/SHA256SUMS"

Write-Host "LoadLynx host tools install plan"
Write-Host "  source: $BaseUrl"
Write-Host "  archive: $Archive"
Write-Host "  install_dir: $InstallDir"
Write-Host "  force: $($Force.IsPresent)"

if ($DryRun) {
    Write-Host "dry-run: no files downloaded or installed"
    exit 0
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    Fail "missing required command: tar"
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("loadlynx-install-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $TempDir | Out-Null

try {
    $ArchivePath = Join-Path $TempDir $Archive
    $ChecksumsPath = Join-Path $TempDir "SHA256SUMS"

    Invoke-WebRequest -Uri $ArchiveUrl -OutFile $ArchivePath -MaximumRedirection 5 | Out-Null
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumsPath -MaximumRedirection 5 | Out-Null

    $Expected = ""
    foreach ($Line in Get-Content $ChecksumsPath) {
        $Parts = $Line.Trim() -split '\s+'
        if ($Parts.Length -ge 2 -and $Parts[1] -eq $Archive) {
            $Expected = $Parts[0].ToLowerInvariant()
            break
        }
    }
    if (-not $Expected) {
        Fail "SHA256SUMS does not contain $Archive"
    }

    $Actual = (Get-FileHash -Algorithm SHA256 $ArchivePath).Hash.ToLowerInvariant()
    if ($Actual -ne $Expected) {
        Fail "checksum mismatch for $Archive"
    }

    $ExtractDir = Join-Path $TempDir "extract"
    New-Item -ItemType Directory -Path $ExtractDir | Out-Null
    tar -xzf $ArchivePath -C $ExtractDir

    $Loadlynx = Join-Path $ExtractDir "loadlynx.exe"
    $Devd = Join-Path $ExtractDir "loadlynx-devd.exe"
    if (-not (Test-Path $Loadlynx)) { Fail "archive missing loadlynx.exe" }
    if (-not (Test-Path $Devd)) { Fail "archive missing loadlynx-devd.exe" }

    $InstalledLoadlynx = Join-Path $InstallDir "loadlynx.exe"
    if ((Test-Path $InstalledLoadlynx) -and -not $Force) {
        $InstalledVersion = ""
        try {
            $VersionOutput = & $InstalledLoadlynx --version 2>$null
            if ($VersionOutput) {
                $InstalledVersion = (($VersionOutput | Select-Object -First 1) -split '\s+')[-1]
            }
        } catch {
            $InstalledVersion = ""
        }
        if ($InstalledVersion -and $Version -ne "latest" -and $InstalledVersion -eq $Version) {
            Write-Host "loadlynx $InstalledVersion is already installed; use -Force to reinstall"
            exit 0
        }
        Fail "loadlynx is already installed in $InstallDir; use -Force to replace it"
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -Force $Loadlynx (Join-Path $InstallDir "loadlynx.exe")
    Copy-Item -Force $Devd (Join-Path $InstallDir "loadlynx-devd.exe")

    & (Join-Path $InstallDir "loadlynx.exe") --help | Out-Null
    & (Join-Path $InstallDir "loadlynx-devd.exe") --help | Out-Null

    Write-Host "installed LoadLynx host tools to $InstallDir"
    $PathEntries = @($env:PATH -split ';') | Where-Object { $_ }
    if ($PathEntries -notcontains $InstallDir) {
        Write-Host "PATH note: add this directory before using loadlynx from a new shell:"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';$InstallDir', 'User')"
    }
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
