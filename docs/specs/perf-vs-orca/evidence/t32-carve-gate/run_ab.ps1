# A/B measurement driver for wayfinder ticket 32 (emit-pass carve bbox gate).
#
# Two arms, one variable: the tree-support-planner guest WASM. Everything else
# (pnp_cli.exe, module set, config, fixture, thread count) is byte-identical.
#
# Interleaved schedule: for each repeat r, run baseline then candidate (arm
# order alternates by parity) so external load is shared rather than allocated
# to one arm. Uninstrumented runs only; wall = process creation-to-exit via
# GetProcessTimes, CPU = kernel+user; per-sample cpu/wall is recorded so
# starved samples can be excluded (the §10.3 trap).
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t32-carve-gate/run_ab.ps1 -Fixture benchy -Runs 10
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t32-carve-gate/run_ab.ps1 -Fixture base -Runs 1 -Warmups 0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('benchy', 'base')][string]$Fixture,
    [int]$Runs = 10,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [ValidateSet('ordinary', 'accelerated')][string]$Mode = 'ordinary',
    [ValidateSet('baseline', 'candidate')][string]$FirstArm = 'baseline',
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)

$ErrorActionPreference = 'Stop'

$Config     = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-classic-supports-on.json'
$Exe        = if ($Mode -eq 'accelerated') {
    Join-Path $RepoRoot 'target\dist-accelerated\developer\pnp_cli.exe'
} else {
    Join-Path $RepoRoot 'target\release\pnp_cli.exe'
}
$Model      = if ($Fixture -eq 'benchy') { Join-Path $RepoRoot 'tmp\3dbenchy.stl' } else { Join-Path $RepoRoot 'tmp\base.stl' }
$armPrefix = if ($Mode -eq 'accelerated') { 'mods-acc-' } else { 'mods-' }
$Arms = [ordered]@{
    baseline  = Join-Path $RepoRoot ('target\t32-ab\' + $armPrefix + 'baseline')
    candidate = Join-Path $RepoRoot ('target\t32-ab\' + $armPrefix + 'candidate')
}

$RunDir  = Join-Path $RepoRoot ('target\t32-ab\runs\' + $Fixture + '-' + $Mode + '-' + $BatchId)
$CsvPath = Join-Path $RunDir 'ab-rows.csv'
New-Item -ItemType Directory -Path $RunDir -Force | Out-Null

foreach ($p in @($Config, $Exe, $Model)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing artifact: $p" }
}
foreach ($arm in $Arms.Values) {
    $wasm = Join-Path $arm 'tree-support-planner\tree-support-planner.wasm'
    if (-not (Test-Path -LiteralPath $wasm)) { throw "missing arm artifact: $wasm" }
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
    $moduleDir = $Arms[$Arm]
    $tag = if ($Warmup) { 'w' } else { 'm' }
    $runId = "$Fixture-$Arm-$tag$Index"
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
    $proc = Start-Process -FilePath $Exe -ArgumentList $argList -PassThru -NoNewWindow -Environment $envMap -RedirectStandardError $err
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

    # Evidence: exactly one slice_complete, status ok/degraded, no fatals.
    $status = ''; $fatal = -1; $nonFatal = -1; $degraded = $null; $completionCount = 0
    foreach ($line in [System.IO.File]::ReadLines($err)) {
        if ($line.TrimStart().StartsWith('{')) {
            try { $ev = $line | ConvertFrom-Json } catch { continue }
            if ($ev.event -eq 'slice_complete') {
                $completionCount++
                $status = [string]$ev.status; $fatal = [int]$ev.fatal_error_count
                $nonFatal = [int]$ev.non_fatal_error_count; $degraded = [bool]$ev.degraded
            }
        }
    }
    if ($exitCode -ne 0) { throw "run failed ($runId): exit $exitCode" }
    if ($completionCount -ne 1 -or $status -notin @('ok', 'degraded') -or $fatal -ne 0) {
        throw "run rejected ($runId): completion_count=$completionCount status=$status fatal=$fatal"
    }
    $gcodeBytes = 0; $sha = ''
    if (Test-Path -LiteralPath $out) {
        $gcodeBytes = (Get-Item -LiteralPath $out).Length
        $sha = (Get-FileHash -Path $out -Algorithm SHA256).Hash.ToLower()
    }
    $rows = @"
$runId,$Fixture,$Arm,$Index,$(if ($Warmup) { 1 } else { 0 }),$Threads,$([math]::Round($times.WallSeconds,4)),$([math]::Round($times.CpuSeconds,4)),$([math]::Round($times.CpuSeconds / $times.WallSeconds,4)),$maxWs,$gcodeBytes,$sha,$status,$fatal,$nonFatal,$degraded,$exitCode,$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ'))
"@
    Add-Content -Path $CsvPath -Value $rows -Encoding utf8
    Write-Host ("[t32-ab] {0} {1} wall={2:N1}s cpu={3:N1}s ratio={4:N2} nonfatal={5} degraded={6}" -f $runId, (Get-Date).ToString('HH:mm:ss'), $times.WallSeconds, $times.CpuSeconds, ($times.CpuSeconds / $times.WallSeconds), $nonFatal, $degraded)
}

$header = 'run_id,fixture,arm,run_index,warmup,threads,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,completion_status,fatal_error_count,non_fatal_error_count,degraded,exit_code,timestamp_utc'
if (-not (Test-Path $CsvPath)) {
    Set-Content -Path $CsvPath -Value $header -Encoding utf8
} else {
    $actual = Get-Content -LiteralPath $CsvPath -TotalCount 1
    if ($actual -ne $header) { throw "CSV header mismatch in $CsvPath" }
}

Write-Host "[t32-ab] batch $BatchId fixture=$Fixture runs=$Runs warmups=$Warmups threads=$Threads"

for ($w = 1; $w -le $Warmups; $w++) {
    foreach ($arm in $Arms.Keys) { Invoke-Arm -Arm $arm -Index $w -Warmup $true }
}
for ($r = 1; $r -le $Runs; $r++) {
    # Alternate which arm goes first on odd/even repeats so a monotonic
    # external-load trend cannot land on one arm.
    $order = if (($r % 2 -eq 1) -eq ($FirstArm -eq 'baseline')) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { Invoke-Arm -Arm $arm -Index $r -Warmup $false }
}

Write-Host "[t32-ab] batch complete. rows: $CsvPath"
