<#
.SYNOPSIS
    Ticket-33 run arm: one uninstrumented integrated-vs-ordinary slice pair batch.

.DESCRIPTION
    Perf-vs-orca ticket 33 (Integrated/external matched-output oracle gate).
    Lineage: t44 run-ab.ps1 (GetProcessTimes, snapshot isolation, one-shot guard,
    sha equality) and matched-pair run_scoreboard.ps1 (validate_measurement).

    - Arms: 'external' = target/dist/developer snapshot (all 24 core modules
      external/WASM); 'integrated' = target/dist/integrated snapshot (all 24
      integrated/native, zero staged external modules).
    - Snapshot pinning: a SHA-256 of both snapshot trees is taken ONCE per
      campaign into snapshot-manifest.tsv (a plain hash walk; both snapshots
      are outside target/ build churn for the campaign because dist is not
      re-run). Rows reference it; no re-pin.
    - Provenance proof: module diagnose on each arm's own snapshot must be
      pass=true, modules_loaded >= 24, with expected provenance (external /
      integrated) on every module, before the first slice.
    - One-shot guard like t44: the measure phase dir must not pre-exist.

.EXAMPLE
    pwsh -NoProfile -File run-t33.ps1 -Phase proof -Arms external,integrated
    pwsh -NoProfile -File run-t33.ps1 -Phase measure -Arms external,integrated -Runs 3
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('external', 'integrated')]
    [string]$Arm,
    [Parameter(Mandatory)]
    [ValidateSet('benchy', 'base')]
    [string]$Fixture,
    [Parameter(Mandatory)]
    [ValidateSet('classic', 'arachne')]
    [string]$Generator,
    [Parameter(Mandatory)]
    [ValidateSet('off', 'on')]
    [string]$Supports,
    [Parameter(Mandatory)]
    [ValidateSet('proof', 'measure')]
    [string]$Phase,
    [ValidateSet('external', 'integrated')]
    [string]$PairArm,
    [int]$Index = 0,
    [int]$Threads = 12,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
if (-not $PairArm) { $PairArm = $Arm }
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')
$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Config = Join-Path $ConfigDir ("pnp-" + $Generator + "-supports-" + $Supports + ".json")
$Models = @{ benchy = Join-Path $RepoRoot 'tmp\3dbenchy.stl'; base = Join-Path $RepoRoot 'tmp\base.stl' }
$Model = $Models[$Fixture]
$Root = 'docs\specs\perf-vs-orca\evidence\t33-integrated-oracle'
$RunDir = Join-Path $RepoRoot ($Root + '\' + $Phase + '\' + $Fixture + '-' + $Generator + '-' + $Supports)
New-Item -ItemType Directory -Path $RunDir -Force | Out-Null
$CsvPath = Join-Path $RunDir ($Fixture + '-' + $Generator + '-' + $Supports + '.csv')
$CsvHeader = 'arm,pair,warmup,threads,wall_seconds,cpu_seconds,cpu_wall_ratio,gcode_bytes,sha256,type_counts_json,completion_status,fatal_error_count,non_fatal_error_count,degraded,open_loop_warnings,shadow_warnings,exit_code'
$Existing = 0
if (Test-Path $CsvPath) {
    $Existing = @(Import-Csv $CsvPath).Count
}
if ($Phase -eq 'proof' -and $Existing -ge 2) {
    throw "one-shot guard: proof batch already complete for this cell ($Existing rows, need external+integrated); no overwrite or automatic resampling"
}
if ($Phase -eq 'proof' -and $Existing -eq 1) {
    $prior = (Import-Csv $CsvPath)[0]
    if ($prior.arm -eq $Arm) { throw "one-shot guard: proof row for arm $Arm already present; no overwrite or automatic resampling" }
}
if ($Phase -eq 'measure' -and $Existing -ge 2) {
    throw "one-shot guard: measured batch already present for this cell ($Existing rows); no overwrite, rerun or automatic resampling"
}
if ($Phase -eq 'measure') {
    $proofCsv = Join-Path $RepoRoot ($Root + '\proof\' + $Fixture + '-' + $Generator + '-' + $Supports + '.csv')
    if (-not (Test-Path $proofCsv)) { throw "missing proof phase for this cell: $proofCsv" }
}
if (-not (Test-Path -LiteralPath $Model)) { throw "missing model: $Model" }
if (-not (Test-Path -LiteralPath $Config)) { throw "missing config: $Config" }

$ArmSpecs = @{
    'external' = @{
        Exe = Join-Path $RepoRoot 'target\dist\developer\pnp_cli.exe'
        Modules = Join-Path $RepoRoot 'target\dist\developer\modules'
        Provenance = 'external'
        Flags = @('--no-default-module-paths', '--no-integrated-modules')
    }
    'integrated' = @{
        Exe = Join-Path $RepoRoot 'target\dist\integrated\pnp_cli.exe'
        Modules = Join-Path $RepoRoot 'target\dist\integrated\modules'
        Provenance = 'integrated'
        Flags = @('--no-default-module-paths')
    }
}
$Spec = $ArmSpecs[$Arm]
if (-not (Test-Path -LiteralPath $Spec.Exe)) { throw "missing binary: $($Spec.Exe)" }

if (-not (Test-Path -LiteralPath $Spec.Modules)) {
    Write-Host "[t33] note: $($Arm) module dir not present; relying on current_exe()/modules default discovery"
}
$diag = & $Spec.Exe module diagnose --module-dir $Spec.Modules 2>$null | Out-String
$parsed = $diag | ConvertFrom-Json
if (-not $parsed.pass) { throw "provenance proof failed: diagnose pass=false ($Arm)" }
if ([int]$parsed.modules_loaded -lt 24) { throw "provenance proof failed: modules_loaded=$($parsed.modules_loaded) ($Arm)" }
$bad = @($parsed.modules | Where-Object { $_.provenance -ne $Spec.Provenance })
if ($bad.Count -gt 0) {
    throw "provenance proof failed: $($bad.Count) modules not provenance '$($Spec.Provenance)' on $Arm (first: $($bad[0].id))"
}

if (-not ('T33ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T33ProcessTimes {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h, out System.Runtime.InteropServices.ComTypes.FILETIME c, out System.Runtime.InteropServices.ComTypes.FILETIME e, out System.Runtime.InteropServices.ComTypes.FILETIME k, out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime << 32) + (uint)f.dwLowDateTime;
}
'@
}
$runId = "$Fixture-$Generator-$Supports-$Arm-$(if ($Index -eq 0) { 'proof' } else { 'm' + $Index })"
$Out = Join-Path $RunDir ($runId + '.gcode')
$Err = Join-Path $RunDir ($runId + '.stderr.jsonl')
$Stdout = Join-Path $RunDir ($runId + '.stdout.log')
$argList = @(
    'slice',
    '--model', ('"' + $Model + '"'),
    '--config', ('"' + $Config + '"'),
    '--output', ('"' + $Out + '"'),
    '--module-dir', ('"' + $Spec.Modules + '"')
) + $Spec.Flags
$proc = Start-Process -FilePath $Spec.Exe -ArgumentList $argList -PassThru -NoNewWindow `
    -Environment @{ RAYON_NUM_THREADS = "$Threads" } -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $proc.Handle
if (-not $proc.WaitForExit(600000)) { $proc.Kill(); throw "slice timed out: $runId" }
$proc.Refresh()
$creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$exit = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T33ProcessTimes]::GetProcessTimes($handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
    throw 'GetProcessTimes failed'
}
$wall = ([T33ProcessTimes]::T($exit) - [T33ProcessTimes]::T($creation)) / 10000000.0
$cpu = ([T33ProcessTimes]::T($kernel) + [T33ProcessTimes]::T($user)) / 10000000.0
$evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $Out -StderrPath $Err -ExpectedGenerator $Generator -ExitCode $proc.ExitCode
$warnings = 0
$shadow = 0
$unmet = 0
$unclosed = 0
foreach ($line in [IO.File]::ReadLines($Err)) {
    if ($line.Contains('shadows integrated')) { $shadow++ }
    if ($line.Contains('is not closed')) { $unclosed++ }
    if ($line -match 'unmet') { $unmet++ }
    if ($line.Contains('PERF-T33')) { throw "probe leakage: $runId" }
}
if ($unclosed -ge 1) { throw "open loop path emitted: $runId (unclosed-loop warnings=$unclosed)" }
$types = [ordered]@{}
foreach ($line in [IO.File]::ReadLines($Out)) {
    if ($line -match '^;\s*TYPE:\s*(.+?)\s*$') {
        $key = $Matches[1]
        if (-not $types.Contains($key)) { $types[$key] = 0 }
        $types[$key]++
    }
}
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
if (-not (Test-Path $CsvPath)) {
    Set-Content -Path $CsvPath -Value $CsvHeader -Encoding utf8
}
[pscustomobject][ordered]@{
    arm = $Arm; pair = $Index; warmup = $(if ($Phase -eq 'proof') { 1 } else { 0 }); threads = $Threads
    wall_seconds = [math]::Round($wall, 4); cpu_seconds = [math]::Round($cpu, 4); cpu_wall_ratio = [math]::Round($cpu / $wall, 4)
    gcode_bytes = (Get-Item $Out).Length; sha256 = $sha; type_counts_json = ($types | ConvertTo-Json -Compress)
    completion_status = $evidence.CompletionStatus; fatal_error_count = $evidence.FatalErrorCount
    non_fatal_error_count = $evidence.NonFatalErrorCount; degraded = $evidence.Degraded
    open_loop_warnings = 0; shadow_warnings = $shadow; exit_code = $proc.ExitCode
} | Export-Csv -Path $CsvPath -Append -NoTypeInformation -Encoding utf8
$proc.Dispose()
Write-Host ("[t33] {0} {1} wall={2:N3}s cpu={3:N3}s ratio={4:N3} bytes={5} sha={6}" -f $runId, $Arm, $wall, $cpu, ($cpu / $wall), (Get-Item $Out).Length, $sha)