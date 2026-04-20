param(
  [int]$RecordSeconds = 10,
  [string]$OutputDir = "output",
  [switch]$PauseResume,
  [int]$PauseAfterSeconds = 3,
  [int]$PauseHoldSeconds = 2
)

$ErrorActionPreference = "Stop"

function Get-NewestSession {
  param(
    [string]$Root,
    [hashtable]$Before
  )

  $deadline = (Get-Date).AddSeconds(20)
  while ((Get-Date) -lt $deadline) {
    if (Test-Path $Root) {
      $created = Get-ChildItem $Root -Directory | Where-Object { -not $Before.ContainsKey($_.Name) } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
      if ($created) {
        return $created
      }
    }
    Start-Sleep -Milliseconds 500
  }
  return $null
}

function Get-DirectorySizeBytes {
  param([string]$Path)

  if (-not (Test-Path $Path)) {
    return 0
  }

  $sum = 0
  Get-ChildItem $Path -Recurse -File | ForEach-Object {
    $sum += $_.Length
  }
  return $sum
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repoRoot "target\debug\screen-timeline-recorder.exe"

if (-not (Test-Path $exe)) {
  throw "missing executable: $exe`nRun `cargo build` first."
}

New-Item -ItemType Directory -Force -Path (Join-Path $repoRoot $OutputDir) | Out-Null

$before = @{}
$sessionsRoot = Join-Path $repoRoot (Join-Path $OutputDir "sessions")
if (Test-Path $sessionsRoot) {
  Get-ChildItem $sessionsRoot -Directory | ForEach-Object { $before[$_.Name] = $true }
}

$proc = $null
$session = $null

try {
  $proc = Start-Process $exe -ArgumentList @("record-video", "--output-dir", $OutputDir) -WorkingDirectory $repoRoot -PassThru
  Write-Host "record pid=$($proc.Id)"

  Start-Sleep -Seconds 2
  $session = Get-NewestSession $sessionsRoot $before
  if (-not $session) {
    throw "no session directory created"
  }

  Write-Host "session_id=$($session.Name)"

  if ($PauseResume) {
    Start-Sleep -Seconds ([Math]::Max(0, $PauseAfterSeconds - 2))
    & $exe pause $session.Name --output-dir $OutputDir | Out-Null
    Write-Host "paused"
    Start-Sleep -Seconds $PauseHoldSeconds
    & $exe resume $session.Name --output-dir $OutputDir | Out-Null
    Write-Host "resumed"
  }

  $remaining = [Math]::Max(1, $RecordSeconds - 2 - ($(if ($PauseResume) { $PauseHoldSeconds } else { 0 })))
  Start-Sleep -Seconds $remaining
  & $exe stop $session.Name --output-dir $OutputDir | Out-Null
  Start-Sleep -Seconds 2

  $statusJson = & $exe status $session.Name --output-dir $OutputDir | Out-String
  $status = $statusJson | ConvertFrom-Json
  $sizeBytes = Get-DirectorySizeBytes $session.FullName
  $segmentCount = if (Test-Path (Join-Path $session.FullName "segments")) {
    (Get-ChildItem (Join-Path $session.FullName "segments") -Filter *.mp4 -File | Measure-Object).Count
  } else {
    0
  }

  Write-Host ("state={0} segments={1} finished_at={2} size_bytes={3}" -f `
    $status.state, `
    $segmentCount, `
    $status.stats.finished_at, `
    $sizeBytes)
}
finally {
  if ($proc -and -not $proc.HasExited) {
    if ($session) {
      try {
        & $exe stop $session.Name --output-dir $OutputDir | Out-Null
      }
      catch {
      }
      Start-Sleep -Seconds 1
    }

    if (-not $proc.HasExited) {
      Stop-Process -Id $proc.Id -Force
    }
  }
}
