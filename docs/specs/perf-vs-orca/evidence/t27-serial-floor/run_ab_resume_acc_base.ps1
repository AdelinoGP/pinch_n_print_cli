# T27 accelerated-base measure resume instrument (2026-10-09).
#
# Why this exists: the standing driver run_ab_par_predicate.ps1 one-shot-guards
# target\t27-ab\campaign\measure\accelerated\base once the batch has run. The
# batch was interrupted by an external process-tree termination at 2026-10-09
# ~14:05:17 local: the whole bash->pwsh->pnp_cli tree died at once while
# candidate-m1 was ~2 s into its slice. No machine reboot, no WER/crash dump;
# cause not identified from host logs. State at the interruption: warmups plus
# baseline-m1 complete; candidate-m1 killed (no gcode, no row; its partial
# stderr/stdout preserved as *.attempt1-killed.*).
#
# This instrument is prefix-driven: it re-derives the canonical standing
# sequence (2 warmups + 6 alternating pairs, odd pairs AB / even pairs BA),
# verifies rows.csv is a prefix of that sequence, and runs ONLY the missing
# suffix with the measurement machinery copied verbatim from the standing
# driver. Rows are appended to the same rows.csv; the proved reference SHA is
# enforced on every output. Re-invoke after any interruption: a completed run
# is never re-run (the prefix check refuses inconsistent states).
#
# One discarded warm-up slice (baseline arm, output discarded, no row) runs
# once -- before the first resumed run only -- to re-establish the batch's
# steady machine state after the interruption; it is skipped when the batch
# already has resumed runs (rows beyond the initial 3).
#
# Disclosed caveat: pair 1 straddles the interruption (baseline-m1 measured
# before, candidate-m1 after); the discard warm-up plus the standing
# starvation filter are the controls. Any further interruption is handled by
# re-invoking this same instrument.
#
# Usage:
#   pwsh -NoProfile -File run_ab_resume_acc_base.ps1 -CheckOnly      # guards only
#   pwsh -NoProfile -File run_ab_resume_acc_base.ps1                 # all missing runs
#   pwsh -NoProfile -File run_ab_resume_acc_base.ps1 -MaxRuns 3      # chunked
[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [int]$MaxRuns = 99,
    [int]$Threads = 12,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')

$Config = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-classic-supports-on.json'
$Model = Join-Path $RepoRoot 'tmp\base.stl'
$Root = Join-Path $RepoRoot 'target\t27-ab\campaign'
$Snapshots = Join-Path $Root 'snapshots'
$Mode = 'accelerated'
$Phase = 'measure'
$Cell = 'base'
if ($Threads -ne 12) { throw 'standing protocol requires 12 threads' }
$RunDir = Join-Path $Root "$Phase\$Mode\$Cell"
$Csv = Join-Path $RunDir 'rows.csv'
$ReferenceSha = 'f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319'
$ExpectedExeSha = @{
    baseline  = 'c46eff52d293f2150d8394db857841595e090ec1b91bb04eb4761deda7a083f2'
    candidate = '1ef4da21e3be76b114a0370cd805d16538d904175a2be542fdf8788f16507954'
}
$DoneMarker = Join-Path $RunDir 'resume-acc-base.done'

# Canonical standing sequence: warmups, then alternating pairs
# (odd pairs baseline->candidate, even pairs candidate->baseline).
$Canonical = @()
$Canonical += ,@{ Arm = 'baseline';  Index = 0; Warmup = $true }
$Canonical += ,@{ Arm = 'candidate'; Index = 0; Warmup = $true }
for ($i = 1; $i -le 6; $i++) {
    $order = if ($i % 2 -eq 1) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { $Canonical += ,@{ Arm = $arm; Index = $i; Warmup = $false } }
}
function Get-RunId {
    param($Slot)
    "$Mode-$Cell-$($Slot.Arm)-$(if ($Slot.Warmup) { 'w' } else { 'm' })$($Slot.Index)"
}

Write-Host '[t27-resume-acc] verifying preconditions'
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
if ($rows.Count -gt $Canonical.Count) {
    throw "resume: rows.csv has $($rows.Count) rows; canonical sequence has $($Canonical.Count)"
}
for ($k = 0; $k -lt $rows.Count; $k++) {
    $want = Get-RunId $Canonical[$k]
    if ($rows[$k].run_id -ne $want) {
        throw "resume: rows.csv row $k is '$($rows[$k].run_id)'; canonical requires '$want'"
    }
    if ($rows[$k].sha256 -ne $ReferenceSha) {
        throw "resume: row '$($rows[$k].run_id)' sha does not match the proved reference"
    }
    if (-not (Test-Path (Join-Path $RunDir "$($rows[$k].run_id).gcode"))) {
        throw "resume: gcode missing for completed row '$($rows[$k].run_id)'"
    }
}
$missing = @()
for ($k = $rows.Count; $k -lt $Canonical.Count; $k++) { $missing += ,$Canonical[$k] }
Write-Host "[t27-resume-acc] prefix verified: $($rows.Count) completed row(s); $($missing.Count) missing run(s)"
foreach ($slot in $missing) { Write-Host "  missing: $(Get-RunId $slot)" }
if ($CheckOnly) { Write-Host '[t27-resume-acc] check-only mode: no slices run'; return }
if ($missing.Count -eq 0) {
    Set-Content -Path $DoneMarker -Value "already complete $(Get-Date -Format o)"
    Write-Host '[t27-resume-acc] nothing missing; done marker written'
    return
}

# A missing run (no row) may carry partial artifacts left by a killed attempt.
# They are inert (the reducer reads rows.csv only); remove them so the re-run
# starts fresh. The per-run reference-SHA check guards the new output.
foreach ($slot in $missing) {
    $rid = Get-RunId $slot
    foreach ($f in @(
        (Join-Path $RunDir "$rid.gcode"),
        (Join-Path $RunDir "$rid.stderr.jsonl"),
        (Join-Path $RunDir "$rid.stdout.log"))) {
        if (Test-Path $f) {
            Write-Host "[t27-resume-acc] removing stale partial artifact for '$rid': $(Split-Path $f -Leaf)"
            Remove-Item -Force $f
        }
    }
}

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
    Write-Host ("[t27-resume-acc] {0} {1} wall={2:N3}s cpu={3:N3}s ratio={4:N3} sha={5}" -f $Phase, $runId, $wall, $cpu, ($cpu / $wall), $sha)
}

# Discarded warm-up slice: only before the first resumed run. If a previous
# resume attempt was killed during its own warm-up, the stale discard files
# are removed first (they are discard outputs by construction).
$needWarmup = ($rows.Count -le 3)
if ($needWarmup) {
    Write-Host '[t27-resume-acc] discarded warm-up slice (baseline arm; output discarded, no row)'
    $warmExe = Join-Path $Snapshots "$Mode-baseline\pnp_cli.exe"
    $warmModules = Join-Path $Snapshots "$Mode-baseline\modules"
    $warmOut = Join-Path $RunDir '_resume-discard.gcode'
    $warmErr = Join-Path $RunDir '_resume-discard.stderr.jsonl'
    $warmStdout = Join-Path $RunDir '_resume-discard.stdout.log'
    Remove-Item -Force -ErrorAction SilentlyContinue $warmOut, $warmErr, $warmStdout
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
    Write-Host '[t27-resume-acc] discard warm-up complete'
} else {
    Write-Host '[t27-resume-acc] warm-up skipped (batch already has resumed runs)'
}

$todo = @($missing | Select-Object -First $MaxRuns)
foreach ($slot in $todo) {
    Invoke-Arm -Arm $slot.Arm -Index $slot.Index -Warmup $slot.Warmup
}

$after = @(Import-Csv $Csv)
Write-Host "[t27-resume-acc] rows now $($after.Count) of $($Canonical.Count)"
if ($after.Count -eq $Canonical.Count) {
    foreach ($r in $after) {
        if ($r.sha256 -ne $ReferenceSha) { throw "resume: post-run sha check failed on '$($r.run_id)'" }
    }
    @(
        "completed $(Get-Date -Format o)",
        "rows=$($after.Count)",
        "reference sha=$ReferenceSha",
        'accelerated/base measure batch complete: warmups + pairs 1..6'
    ) | Set-Content -Path $DoneMarker -Encoding utf8
    Write-Host '[t27-resume-acc] batch complete; done marker written'
} else {
    Write-Host "[t27-resume-acc] batch incomplete ($($after.Count)/$($Canonical.Count)); re-invoke to continue"
}
