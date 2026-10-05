# A/B measurement driver for wayfinder ticket 37 (InfillPostProcess
# empty-output commit protocol).
#
# Two arms, one variable: the pnp_cli host binary. The fix is host-side only
# (dispatch / native marshal), so every guest WASM, manifest, config, fixture,
# and thread count is identical across arms; only the executable differs.
#
# Measurement core is unchanged from the ticket-32 driver / run_bench.ps1
# lineage: process creation-to-exit wall clock and kernel+user CPU via
# GetProcessTimes, peak working set by polling, one CSV row per run,
# per-sample cpu/wall so starved samples can be excluded (the §10.3 trap).
# Uninstrumented runs only; exactly one slice_complete, status ok/degraded,
# zero fatals, per run.
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t37-empty-commit-protocol/run_ab.ps1 `
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

$generator = if ($Cell.StartsWith('classic')) { 'classic' } else { 'arachne' }
$Config    = Join-Path $RepoRoot ('docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-{0}-supports-{1}.json' -f $generator, $(if ($Cell.EndsWith('on')) { 'on' } else { 'off' }))
$Model     = if ($Fixture -eq 'benchy') { Join-Path $RepoRoot 'tmp\3dbenchy.stl' } else { Join-Path $RepoRoot 'tmp\base.stl' }

if ($Mode -eq 'accelerated') {
    # The complete `cargo xtask dist --accelerated` snapshots, archived to
    # mode-distinct dirs so the candidate build cannot overwrite the baseline.
    $Arms = [ordered]@{
        baseline  = @{
            Exe = Join-Path $RepoRoot 'target\t37-ab\acc-baseline\pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'target\t37-ab\acc-baseline\modules'
        }
        candidate = @{
            Exe = Join-Path $RepoRoot 'target\t37-ab\acc-candidate\pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'target\t37-ab\acc-candidate\modules'
        }
    }
} else {
    # Ordinary: release host binary + the in-tree module dir (recipe §3.4).
    $Arms = [ordered]@{
        baseline  = @{
            Exe = Join-Path $RepoRoot 'target\t37-ab\baseline-ordinary-pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'modules\core-modules'
        }
        candidate = @{
            Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
            Mod = Join-Path $RepoRoot 'modules\core-modules'
        }
    }
}

$RunDir  = Join-Path $RepoRoot ('target\t37-ab\runs\' + $Fixture + '-' + $Cell + '-' + $Mode + '-' + $BatchId)
$CsvPath = Join-Path $RunDir 'ab-rows.csv'
New-Item -ItemType Directory -Path $RunDir -Force | Out-Null

foreach ($p in @($Config, $Model)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing artifact: $p" }
}
foreach ($arm in $Arms.Values) {
    if (-not (Test-Path -LiteralPath $arm.Exe)) { throw "missing arm executable: $($arm.Exe)" }
    if (-not (Test-Path -LiteralPath $arm.Mod)) { throw "missing arm module dir: $($arm.Mod)" }
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
$runId,$Fixture,$Cell,$Mode,$Arm,$Index,$(if ($Warmup) { 1 } else { 0 }),$Threads,$([math]::Round($times.WallSeconds,4)),$([math]::Round($times.CpuSeconds,4)),$([math]::Round($times.CpuSeconds / $times.WallSeconds,4)),$maxWs,$gcodeBytes,$sha,$status,$fatal,$nonFatal,$degraded,$exitCode,$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ'))
"@
    Add-Content -Path $CsvPath -Value $rows -Encoding utf8
    Write-Host ("[t37-ab] {0} {1} wall={2:N1}s cpu={3:N1}s ratio={4:N2} bytes={5} nonfatal={6} degraded={7}" -f $runId, (Get-Date).ToString('HH:mm:ss'), $times.WallSeconds, $times.CpuSeconds, ($times.CpuSeconds / $times.WallSeconds), $gcodeBytes, $nonFatal, $degraded)
}

$header = 'run_id,fixture,cell,mode,arm,run_index,warmup,threads,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,completion_status,fatal_error_count,non_fatal_error_count,degraded,exit_code,timestamp_utc'
if (-not (Test-Path $CsvPath)) {
    Set-Content -Path $CsvPath -Value $header -Encoding utf8
} else {
    $actual = Get-Content -LiteralPath $CsvPath -TotalCount 1
    if ($actual -ne $header) { throw "CSV header mismatch in $CsvPath" }
}

Write-Host "[t37-ab] batch $BatchId fixture=$Fixture cell=$Cell mode=$Mode runs=$Runs warmups=$Warmups threads=$Threads"

for ($w = 1; $w -le $Warmups; $w++) {
    foreach ($arm in $Arms.Keys) { Invoke-Arm -Arm $arm -Index $w -Warmup $true }
}
for ($r = 1; $r -le $Runs; $r++) {
    # Alternate which arm goes first on odd/even repeats so a monotonic
    # external-load trend cannot land on one arm.
    $order = if (($r % 2 -eq 1) -eq ($FirstArm -eq 'baseline')) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { Invoke-Arm -Arm $arm -Index $r -Warmup $false }
}

Write-Host "[t37-ab] batch complete. rows: $CsvPath"
