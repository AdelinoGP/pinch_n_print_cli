# A/B measurement driver for wayfinder ticket 36 (modal Z/F token emission).
#
# Two arms, one variable: the pnp_cli host binary. The change is host-side only
# (`ModalZfState` in crates/slicer-gcode/src/serialize.rs), so every guest WASM,
# manifest, config and fixture is identical across arms; only the executable
# differs. Output is NOT byte-identical by construction (repeated tokens are
# removed), so the semantic equivalence gate is the independent oracle
# (verify-t36.py --oracle), not a hash equality.
#
# Measurement core is unchanged from the ticket-37 driver / run_bench.ps1
# lineage: process creation-to-exit wall clock and kernel+user CPU via
# GetProcessTimes, peak working set by polling, one CSV row per run,
# per-sample cpu/wall so starved samples can be excluded (the S10.3 trap).
# Uninstrumented runs only; exactly one slice_complete, status ok/degraded,
# zero fatals, per run.
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t36-modal-zf/run-ab.ps1 `
#       -Fixture benchy -Cell classic-off -Mode ordinary -Runs 6
[ CmdletBinding() ]
param(
    [Parameter(Mandatory = $true)][ValidateSet('benchy', 'base')][string]$Fixture,
    [ValidateSet('classic-off', 'arachne-off', 'classic-on', 'arachne-on')]
    [string]$Cell = 'classic-off',
    [ValidateSet('ordinary', 'accelerated')][string]$Mode = 'ordinary',
    [int]$Runs = 6,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [ValidateSet('baseline', 'candidate')][string]$FirstArm = 'baseline',
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)

$ErrorActionPreference = 'Stop'
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')

$generator = if ($Cell.StartsWith('classic')) { 'classic' } else { 'arachne' }
$Config    = Join-Path $RepoRoot ('docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-{0}-supports-{1}.json' -f $generator, $(if ($Cell.EndsWith('on')) { 'on' } else { 'off' }))
$Model     = if ($Fixture -eq 'benchy') { Join-Path $RepoRoot 'tmp\3dbenchy.stl' } else { Join-Path $RepoRoot 'tmp\base.stl' }

$Snapshots = Join-Path $RepoRoot 'target\t36-ab\snapshots'
if ($Mode -eq 'accelerated') {
    $Arms = [ordered]@{
        baseline  = @{
            Exe = Join-Path $Snapshots 'accelerated-baseline\pnp_cli.exe'
            Mod = Join-Path $Snapshots 'accelerated-baseline\modules'
        }
        candidate = @{
            Exe = Join-Path $Snapshots 'accelerated-candidate\pnp_cli.exe'
            Mod = Join-Path $Snapshots 'accelerated-candidate\modules'
        }
    }
} else {
    $Arms = [ordered]@{
        baseline  = @{
            Exe = Join-Path $Snapshots 'ordinary-baseline\pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'modules\core-modules'
        }
        candidate = @{
            Exe = Join-Path $Snapshots 'ordinary-candidate\pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'modules\core-modules'
        }
    }
}

$RunDir  = Join-Path $RepoRoot ('target\t36-ab\runs\' + $Fixture + '-' + $Cell + '-' + $Mode + '-' + $BatchId)
$CsvPath = Join-Path $RunDir 'ab-rows.csv'
New-Item -ItemType Directory -Path $RunDir -Force | Out-Null

foreach ($p in @($Config, $Model)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing artifact: $p" }
}
foreach ($arm in $Arms.Values) {
    if (-not (Test-Path -LiteralPath $arm.Exe)) { throw "missing arm executable: $($arm.Exe)" }
    if (-not (Test-Path -LiteralPath $arm.Mod)) { throw "missing arm module dir: $($arm.Mod)" }
}

# --- One-variable isolation: accelerated arms must share guest bytes --------
if ($Mode -eq 'accelerated' -and -not $env:T36_SKIP_MODULE_IDENTITY) {
    $baseFiles = Get-ChildItem -Recurse -File -LiteralPath $Arms.baseline.Mod | ForEach-Object {
        [pscustomobject]@{ Rel = $_.FullName.Substring($Arms.baseline.Mod.Length); Hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash }
    }
    $candFiles = Get-ChildItem -Recurse -File -LiteralPath $Arms.candidate.Mod | ForEach-Object {
        [pscustomobject]@{ Rel = $_.FullName.Substring($Arms.candidate.Mod.Length); Hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash }
    }
    $baseMap = @{}; foreach ($f in $baseFiles) { $baseMap[$f.Rel] = $f.Hash }
    $candMap = @{}; foreach ($f in $candFiles) { $candMap[$f.Rel] = $f.Hash }
    $diffs = @()
    foreach ($k in $baseMap.Keys) { if (-not $candMap.ContainsKey($k) -or $candMap[$k] -ne $baseMap[$k]) { $diffs += ($k + ' (baseline)') } }
    foreach ($k in $candMap.Keys) { if (-not $baseMap.ContainsKey($k)) { $diffs += ($k + ' (candidate)') } }
    if ($diffs.Count -gt 0) { throw "accelerated arms differ in guest bytes: $($diffs -join ', ')" }
    Write-Host "[t36-ab] accelerated guest module identity verified ($($baseMap.Count) files)"
}

# --- Measurement core (run_bench.ps1 lineage, unchanged math) -----------------
if (-not ('ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProcessTimes {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h, out System.Runtime.InteropServices.ComTypes.FILETIME c, out System.Runtime.InteropServices.ComTypes.FILETIME e, out System.Runtime.InteropServices.ComTypes.FILETIME k, out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime << 32) + (uint)f.dwLowDateTime;
}
'@
}

function Get-ProcessTimesSeconds {
    param([IntPtr]$Handle)
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exit = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [ProcessTimes]::GetProcessTimes($Handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
        throw "GetProcessTimes failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    [pscustomobject]@{
        WallSeconds = ([ProcessTimes]::T($exit) - [ProcessTimes]::T($creation)) / 10000000.0
        CpuSeconds = ([ProcessTimes]::T($kernel) + [ProcessTimes]::T($user)) / 10000000.0
    }
}

function Invoke-Arm {
    param([string]$Arm, [int]$Index, [bool]$Warmup)
    $exe = $Arms[$Arm].Exe
    $moduleDir = $Arms[$Arm].Mod
    $tag = if ($Warmup) { 'w' } else { 'm' }
    $runId = "$Fixture-$Cell-$Mode-$Arm-$tag$Index"
    $out   = Join-Path $RunDir ($runId + '.gcode')
    $err   = Join-Path $RunDir ($runId + '.stderr.jsonl')
    Remove-Item -Force -ErrorAction SilentlyContinue $out, $err
    $argList = @(
        'slice',
        '--model', ('"' + $Model + '"'),
        '--output', ('"' + $out + '"'),
        '--config', ('"' + $Config + '"'),
        '--module-dir', ('"' + $moduleDir + '"')
    )
    $envMap = @{ RAYON_NUM_THREADS = "$Threads" }
    $proc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru -NoNewWindow -Environment $envMap -RedirectStandardError $err
    $handle = $proc.Handle
    $maxWs = 0
    while ($true) {
        $p = $null
        try { $p = Get-Process -Id $proc.Id -ErrorAction Stop } catch { $p = $null }
        if ($null -eq $p) { break }
        if ($p.PeakWorkingSet64 -gt $maxWs) { $maxWs = $p.PeakWorkingSet64 }
        Start-Sleep -Milliseconds 100
    }
    $proc.WaitForExit()
    $proc.Refresh()
    $times = Get-ProcessTimesSeconds -Handle $handle
    $exitCode = $proc.ExitCode

    $evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $out -StderrPath $err -ExpectedGenerator $generator -ExitCode $exitCode
    $warnings = 0
    foreach ($line in [System.IO.File]::ReadLines($err)) {
        if ($line.Contains('is not closed')) { $warnings++ }
        if ($line.Contains('PERF-T')) { throw "probe leakage: $runId" }
    }
    $gcodeBytes = (Get-Item -LiteralPath $out).Length
    $sha = (Get-FileHash -Path $out -Algorithm SHA256).Hash.ToLower()
    $rows = @"
$runId,$Fixture,$Cell,$Mode,$Arm,$Index,$(if ($Warmup) { 1 } else { 0 }),$Threads,$([math]::Round($times.WallSeconds,4)),$([math]::Round($times.CpuSeconds,4)),$([math]::Round($times.CpuSeconds / $times.WallSeconds,4)),$maxWs,$gcodeBytes,$sha,$($evidence.CompletionStatus),$($evidence.FatalErrorCount),$($evidence.NonFatalErrorCount),$($evidence.Degraded),$exitCode,$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ'))
"@
    Add-Content -Path $CsvPath -Value $rows -Encoding utf8
    Write-Host ("[t36-ab] {0} {1} wall={2:N1}s cpu={3:N1}s ratio={4:N2} bytes={5} nonfatal={6} closed_warn={7}" -f $runId, (Get-Date).ToString('HH:mm:ss'), $times.WallSeconds, $times.CpuSeconds, ($times.CpuSeconds / $times.WallSeconds), $gcodeBytes, $evidence.NonFatalErrorCount, $warnings)
}

$header = 'run_id,fixture,cell,mode,arm,run_index,warmup,threads,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,completion_status,fatal_error_count,non_fatal_error_count,degraded,exit_code,timestamp_utc'
if (-not (Test-Path $CsvPath)) {
    Set-Content -Path $CsvPath -Value $header -Encoding utf8
} else {
    $actual = Get-Content -LiteralPath $CsvPath -TotalCount 1
    if ($actual -ne $header) { throw "CSV header mismatch in $CsvPath" }
}

Write-Host "[t36-ab] batch $BatchId fixture=$Fixture cell=$Cell mode=$Mode runs=$Runs warmups=$Warmups threads=$Threads"

for ($w = 1; $w -le $Warmups; $w++) {
    foreach ($arm in $Arms.Keys) { Invoke-Arm -Arm $arm -Index $w -Warmup $true }
}
for ($r = 1; $r -le $Runs; $r++) {
    # Alternate which arm goes first on odd/even repeats so a monotonic
    # external-load trend cannot land on one arm.
    $order = if (($r % 2 -eq 1) -eq ($FirstArm -eq 'baseline')) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { Invoke-Arm -Arm $arm -Index $r -Warmup $false }
}

Write-Host "[t36-ab] batch complete. rows: $CsvPath"
