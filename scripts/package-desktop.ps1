param(
  [switch]$Release = $true
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$profile = if ($Release) { "release" } else { "debug" }
$args = @("build")
if ($Release) {
  $args += "--release"
}

Write-Host "Building desktop shell ($profile)..."
cargo @args
if ($LASTEXITCODE -ne 0) {
  throw "cargo build failed with exit code $LASTEXITCODE"
}

$exePath = Join-Path $repoRoot "target\$profile\screen-timeline-recorder.exe"
if (-not (Test-Path $exePath)) {
  throw "Expected desktop executable not found: $exePath"
}

$distRoot = Join-Path $repoRoot "dist\desktop"
New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
$distExe = Join-Path $distRoot "screen-timeline-recorder.exe"
Copy-Item -Force $exePath $distExe

$readmePath = Join-Path $distRoot "README.txt"
$readmeLines = @(
  "Screen Timeline Recorder Desktop"
  ""
  "Executable:"
  "screen-timeline-recorder.exe"
  ""
  "Common launch modes:"
  "1. Open desktop UI"
  "   screen-timeline-recorder.exe desktop --output-dir .\output"
  ""
  "2. Start hidden in tray"
  "   screen-timeline-recorder.exe desktop --background --output-dir .\output"
  ""
  "3. Start hidden and begin recording immediately"
  "   screen-timeline-recorder.exe desktop --background --autorun-record --output-dir .\output"
  ""
  "Tray behavior:"
  "- Closing the main window hides it to tray instead of quitting."
  "- Use the tray menu to open the UI, start, pause, resume, stop recording, or quit."
  ""
  "Global shortcuts:"
  "- Ctrl+Alt+Shift+O: Open main UI"
  "- Ctrl+Alt+Shift+R: Start new recording"
  "- Ctrl+Alt+Shift+P: Pause or resume active recording"
  "- Ctrl+Alt+Shift+S: Stop active recording"
  ""
  "Autostart note:"
  "- The web UI can configure login startup to launch the desktop shell in background mode."
  ""
  "Output folder:"
  "- Recording data is written under .\output when you pass --output-dir .\output."
)
$readmeLines | Set-Content -Encoding UTF8 $readmePath

$zipPath = Join-Path $repoRoot "dist\screen-timeline-recorder-desktop.zip"
if (Test-Path $zipPath) {
  Remove-Item -Force $zipPath
}
Compress-Archive -Path (Join-Path $distRoot "*") -DestinationPath $zipPath

Write-Host ""
Write-Host "Desktop executable ready:"
Write-Host $exePath
Write-Host ""
Write-Host "Packaged desktop folder:"
Write-Host $distRoot
Write-Host ""
Write-Host "Portable zip:"
Write-Host $zipPath
