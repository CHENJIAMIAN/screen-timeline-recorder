param(
  [string]$InstallDir = "",
  [switch]$AlsoCopyToTargetRelease
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $InstallDir) {
  $InstallDir = Join-Path $repoRoot "tools\ffmpeg"
}

$downloadUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("screen-timeline-ffmpeg-" + [guid]::NewGuid().ToString("N"))
$zipPath = Join-Path $tempRoot "ffmpeg.zip"
$extractDir = Join-Path $tempRoot "extract"

New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null

Write-Host "Downloading ffmpeg..."
Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath

Write-Host "Extracting ffmpeg..."
Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force

$ffmpegExe = Get-ChildItem -Path $extractDir -Recurse -Filter ffmpeg.exe | Select-Object -First 1
$ffprobeExe = Get-ChildItem -Path $extractDir -Recurse -Filter ffprobe.exe | Select-Object -First 1
if (-not $ffmpegExe) {
  throw "ffmpeg.exe not found in downloaded archive"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Force $ffmpegExe.FullName (Join-Path $InstallDir "ffmpeg.exe")
if ($ffprobeExe) {
  Copy-Item -Force $ffprobeExe.FullName (Join-Path $InstallDir "ffprobe.exe")
}

if ($AlsoCopyToTargetRelease) {
  $targetDir = Join-Path $repoRoot "target\release\ffmpeg"
  New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
  Copy-Item -Force (Join-Path $InstallDir "ffmpeg.exe") (Join-Path $targetDir "ffmpeg.exe")
  if (Test-Path (Join-Path $InstallDir "ffprobe.exe")) {
    Copy-Item -Force (Join-Path $InstallDir "ffprobe.exe") (Join-Path $targetDir "ffprobe.exe")
  }
}

Remove-Item -Recurse -Force $tempRoot

Write-Host ""
Write-Host "ffmpeg installed to:"
Write-Host $InstallDir
