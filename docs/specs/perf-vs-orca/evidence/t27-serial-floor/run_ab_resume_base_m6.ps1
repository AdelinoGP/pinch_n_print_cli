# T27 base-on ordinary pair-6 resume instrument (2026-10-09).
#
# Why this exists: the standing driver run_ab_par_predicate.ps1 one-shot-guards
# target\t27-ab\campaign\measure\ordinary\base once the batch has run. The
# human stopped the campaign mid pair 6 (candidate-m6 killed mid-slice;
# baseline-m6 never started), leaving the batch at warmup + pairs 1..5. This
# instrument re-runs ONLY pair 6 -- in the standing driver's even-pair BA
# order, candidate then baseline -- against the same frozen snapshots, using
# the measurement machinery copied verbatim from the standing driver, and
# appends the two rows to the existing rows.csv. It refuses to run unless the
# batch is exactly warmup + pairs 1..5 with no pair-6 row or gcode, and it
# enforces the proved reference SHA on both outputs (stricter than the
# standing driver, which leaves the reference check to the reducer).
#
# A single discarded warm-up slice (baseline arm, output discarded, no row)
# runs first to re-establish the batch's steady machine state after the
# interruption; the standing batch ran its arms back-to-back within seconds.
#
# Usage:
#   pwsh -NoProfile -File run_ab_resume_base_m6.ps1 -CheckOnly   # verify guards only
#   pwsh -NoProfile -File run_ab_resume_base_m6.ps1              # run pair 6
[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')

$Config = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-classic-supports-on.json'
$Model = Join-Path $RepoRoot 'tmp\base.stl'
$Root = Join-Path $RepoRoot 'target\t27-ab\campaign'
$Snapshots = Join-Path $Root 'snapshots'
$Mode = 'ordinary'
$Phase = 'measure'
$Cell = 'base'
$Threads = 12
$RunDir = Join-Path $Root "$Phase\$Mode\$Cell"
$Csv = Join-Path $RunDir 'rows.csv'
$ReferenceSha = 'f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319'
$ExpectedExeSha = @{
    baseline  = 'd0d6e08fffe3625675a47283be1a1befbd4effd0d589aeabbf2ea3d5f0e37c27'
    candidate = '1ea961b76f7280f4b55cbb9a5b64eee2ce6af0b288bdc49116e27e355d5500dd'
}
$DoneMarker = Join-Path $RunDir 'resume-m6.done'

Write-Host '[t27-resume] verifying preconditions'
if (-not (Test-Path $RunDir)) { throw "resume: run dir missing: $RunDir" }
if (Test-Path $DoneMarker) { throw "one-shot guard: $DoneMarker exists; no re-run" }
if ((Get-FileHash $Model -Algorithm SHA256).Hash.ToLower() -ne 'b26c8c79158bd009119e13c89aa0cb9b6f8100bef1de63f58c0d58e2c1c178af') {
    throw 'resume: model identity differs from the frozen job (base)'
}
foreach ($arm in @('baseline', 'candidate')) {
    $exe = Join-Path $Snapshots "$Mode-$arm\pnp_cli.exe"
    if ((Get-FileHash $exe -Algorithm SHA256).Hash.ToLower() -ne $ExpectedExeSha[$arm]) {
        throw "resume: $arm exe identity differs from identity-ab.json"
    }
}
$rows = @(Import-Csv $Csv)
if ($rows.Count -ne 12) { throw "resume: expected 12 data rows (warmup + pairs 1-5), found $($rows.Count)" }
if (($rows | Where-Object { $_.warmup -eq '1' }).Count -ne 2) { throw 'resume: expected exactly 2 warmup rows' }
foreach ($p in 1..5) {
    foreach ($arm in @('baseline', 'candidate')) {
        if (($rows | Where-Object { $_.arm -eq $arm -and $_.pair -eq "$p" }).Count -ne 1) {
            throw "resume: expected exactly 1 row for pair $p arm $arm"
        }
    }
}
if (($rows | Where-Object { $_.pair -eq '6' }).Count -ne 0) { throw 'resume: pair-6 rows already present' }
foreach ($arm in @('baseline', 'candidate')) {
    if (Test-Path (Join-Path $RunDir "$Mode-$Cell-$arm-m6.gcode")) { throw "resume: $arm m6 gcode already exists" }
}
Write-Host '[t27-resume] preconditions verified: 12 data rows, batch is warmup + pairs 1-5; snapshots match identity-ab.json'
if ($CheckOnly) { Write-Host '[t27-resume] check-only mode: no slices run'; return }

if (-not ('T27ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T27ProcessTimes {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h, out System.Runtime.InteropServices.ComTypes.FILETIME c, out System.Runtime.InteropServices.ComTypes.FILETIME e, out System.Runtime.InteropServices.ComTypes.FILETIME k, out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime << 32) + (uint)f.dwLowDateTime;
}
'@
}
function Invoke-Arm {
    param([string]$Arm, [int]$Index, [bool]$Warmup)
    $snapshot = Join-Path $Snapshots "$Mode-$Arm"
    $exe = Join-Path $snapshot 'pnp_cli.exe'
    $modules = Join-Path $snapshot 'modules'
    if (-not (Test-Path $exe)) { throw "missing arm exe: $exe" }
    if (-not (Test-Path (Join-Path $modules 'classic-perimeters'))) { throw "missing arm modules: $modules" }
    $runId = "$Mode-$Cell-$Arm-$(if ($Warmup) { 'w' } else { 'm' })$Index"
    $out = Join-Path $RunDir "$runId.gcode"
    $err = Join-Path $RunDir "$runId.stderr.jsonl"
    $stdout = Join-Path $RunDir "$runId.stdout.log"
    $argList = @('slice', '--model', ('"' + $Model + '"'), '--config', ('"' + $Config + '"'),
        '--output', ('"' + $out + '"'), '--module-dir', ('"' + $modules + '"'),
        '--no-default-module-paths', '--no-integrated-modules')
    $proc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru -NoNewWindow `
        -Environment @{ RAYON_NUM_THREADS = "$Threads" } -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $proc.Handle
    if (-not $proc.WaitForExit(3600000)) { $proc.Kill(); throw "slice timed out: $runId" }
    $proc.Refresh()
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exitT = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T27ProcessTimes]::GetProcessTimes($handle, [ref]$creation, [ref]$exitT, [ref]$kernel, [ref]$user)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([T27ProcessTimes]::T($exitT) - [T27ProcessTimes]::T($creation)) / 10000000.0
    $cpu = ([T27ProcessTimes]::T($kernel) + [T27ProcessTimes]::T($user)) / 10000000.0
    $evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $out -StderrPath $err -ExpectedGenerator classic -ExitCode $proc.ExitCode
    if ($evidence.CompletionStatus -ne 'ok' -or $evidence.Degraded -or $evidence.FatalErrorCount -ne 0 -or $evidence.NonFatalErrorCount -ne 0) {
        throw "unclean completion: $runId"
    }
    $warnings = 0
    foreach ($line in [IO.File]::ReadLines($err)) {
        if ($line.Contains('shadows integrated') -or $line.Contains('PERF-T') -or $line -match '"event"\s*:\s*"(module_complete|stage_complete|profile_summary)"') {
            throw "tainted provenance/instrumentation: $runId"
        }
        if ($line.Contains('is not closed')) { $warnings++ }
    }
    $sha = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
    if ($sha -ne $ReferenceSha) { throw "reference mismatch: $runId sha $sha" }
    $types = [ordered]@{}
    foreach ($line in [IO.File]::ReadLines($out)) {
        if ($line.StartsWith(';TYPE:')) {
            $key = $line.Substring(6)
            if (-not $types.Contains($key)) { $types[$key] = 0 }
            $types[$key]++
        }
    }
    [pscustomobject][ordered]@{
        run_id = $runId; phase = $Phase; mode = $Mode; cell = $Cell; arm = $Arm; pair = $Index; warmup = [int]$Warmup; threads = $Threads
        wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = ($cpu / $wall)
        bytes = (Get-Item $out).Length; sha256 = $sha; type_counts = ($types | ConvertTo-Json -Compress)
        status = $evidence.CompletionStatus; degraded = $evidence.Degraded; fatal = $evidence.FatalErrorCount; non_fatal = $evidence.NonFatalErrorCount
        open_loop_warnings = $warnings; exit_code = $proc.ExitCode; generator = 'classic'
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $proc.Dispose()
    Write-Host ("[t27-resume] {0} {1} wall={2:N3}s cpu={3:N3}s ratio={4:N3} sha={5}" -f $Phase, $runId, $wall, $cpu, ($cpu / $wall), $sha)
}

# Discarded warm-up slice: same launch semantics, output discarded, no row.
Write-Host '[t27-resume] discarded warm-up slice (baseline arm; output discarded, no row)'
$warmExe = Join-Path $Snapshots "$Mode-baseline\pnp_cli.exe"
$warmModules = Join-Path $Snapshots "$Mode-baseline\modules"
$warmOut = Join-Path $RunDir '_resume-discard.gcode'
$warmErr = Join-Path $RunDir '_resume-discard.stderr.jsonl'
$warmStdout = Join-Path $RunDir '_resume-discard.stdout.log'
$warmArgs = @('slice', '--model', ('"' + $Model + '"'), '--config', ('"' + $Config + '"'),
    '--output', ('"' + $warmOut + '"'), '--module-dir', ('"' + $warmModules + '"'),
    '--no-default-module-paths', '--no-integrated-modules')
$warmProc = Start-Process -FilePath $warmExe -ArgumentList $warmArgs -PassThru -NoNewWindow `
    -Environment @{ RAYON_NUM_THREADS = "$Threads" } -RedirectStandardError $warmErr -RedirectStandardOutput $warmStdout
if (-not $warmProc.WaitForExit(3600000)) { $warmProc.Kill(); throw 'discard warm-up timed out' }
$warmProc.Refresh()
if ($warmProc.ExitCode -ne 0) { throw "discard warm-up failed: exit $($warmProc.ExitCode)" }
$warmProc.Dispose()
Remove-Item -Force $warmOut, $warmErr, $warmStdout
Write-Host '[t27-resume] discard warm-up complete; starting pair 6 (candidate then baseline, the standing even-pair order)'

Invoke-Arm -Arm 'candidate' -Index 6 -Warmup $false
Invoke-Arm -Arm 'baseline' -Index 6 -Warmup $false

$after = @(Import-Csv $Csv)
if ($after.Count -ne 14) { throw "resume: expected 14 data rows after pair 6, found $($after.Count)" }
foreach ($arm in @('baseline', 'candidate')) {
    if (($after | Where-Object { $_.arm -eq $arm -and $_.pair -eq '6' }).Count -ne 1) { throw "resume: missing pair-6 row for $arm" }
}
@(
    "completed $(Get-Date -Format o)",
    "candidate-m6 sha=$((($after | Where-Object { $_.arm -eq 'candidate' -and $_.pair -eq '6' })[0]).sha256)",
    "baseline-m6 sha=$((($after | Where-Object { $_.arm -eq 'baseline' -and $_.pair -eq '6' })[0]).sha256)",
    'batch now complete: warmup + pairs 1..6'
) | Set-Content -Path $DoneMarker -Encoding utf8
Write-Host '[t27-resume] pair 6 complete; batch is warmup + pairs 1..6'
