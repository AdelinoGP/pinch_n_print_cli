# Ticket-37/GetProcessTimes lineage. External-only, uninstrumented slices.
# Only the infill-linker component differs within each mode's isolated pair.
# Proof phase MUST pass before either mode's measurement phase begins.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('proof', 'measure')][string]$Phase,
    [int]$Runs = 6,
    [int]$Threads = 12,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')
$Config = Join-Path $PSScriptRoot 'benchy-arachne.json'
$Model = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
$Snapshots = Join-Path $RepoRoot 'target\t44-ab\snapshots'
$RunDir = Join-Path $RepoRoot "target\t44-ab\$Phase\$Mode"
if (Test-Path $RunDir) { throw "one-shot guard: $RunDir exists; no overwrite or automatic retry" }
if ($Threads -ne 12 -or $Runs -lt 5) { throw 'standing protocol requires 12 threads and at least five pairs' }
if ((Get-FileHash $Model -Algorithm SHA256).Hash.ToLower() -ne '6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7') {
    throw 'Benchy identity differs from the frozen job'
}
$ExpectedSha = $null
if ($Phase -eq 'measure') {
    foreach ($m in @('ordinary', 'accelerated')) {
        $proof = @(Import-Csv (Join-Path $RepoRoot "target\t44-ab\proof\$m\rows.csv"))
        if ($proof.Count -ne 2 -or $proof[0].sha256 -ne $proof[1].sha256) { throw "missing/failed proof for $m" }
        if ($m -eq $Mode) { $ExpectedSha = $proof[0].sha256 }
    }
}
New-Item -ItemType Directory -Path $RunDir | Out-Null
$Csv = Join-Path $RunDir 'rows.csv'
if (-not ('T44ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T44ProcessTimes {
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
    $runId = "$Mode-$Arm-$(if ($Warmup) { 'w' } else { 'm' })$Index"
    $out = Join-Path $RunDir "$runId.gcode"
    $err = Join-Path $RunDir "$runId.stderr.jsonl"
    $stdout = Join-Path $RunDir "$runId.stdout.log"
    $argList = @('slice', '--model', ('"' + $Model + '"'), '--config', ('"' + $Config + '"'),
        '--output', ('"' + $out + '"'), '--module-dir', ('"' + $modules + '"'),
        '--no-default-module-paths', '--no-integrated-modules')
    $proc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru -NoNewWindow `
        -Environment @{ RAYON_NUM_THREADS = "$Threads" } -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $proc.Handle
    if (-not $proc.WaitForExit(600000)) { $proc.Kill(); throw "slice timed out: $runId" }
    $proc.Refresh()
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exit = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T44ProcessTimes]::GetProcessTimes($handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([T44ProcessTimes]::T($exit) - [T44ProcessTimes]::T($creation)) / 10000000.0
    $cpu = ([T44ProcessTimes]::T($kernel) + [T44ProcessTimes]::T($user)) / 10000000.0
    $evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $out -StderrPath $err -ExpectedGenerator arachne -ExitCode $proc.ExitCode
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
    if ($null -ne $ExpectedSha -and $sha -ne $ExpectedSha) { throw "output drift: $runId" }
    $types = [ordered]@{}
    foreach ($line in [IO.File]::ReadLines($out)) {
        if ($line.StartsWith(';TYPE:')) {
            $key = $line.Substring(6)
            if (-not $types.Contains($key)) { $types[$key] = 0 }
            $types[$key]++
        }
    }
    [pscustomobject][ordered]@{
        run_id = $runId; phase = $Phase; mode = $Mode; arm = $Arm; pair = $Index; warmup = [int]$Warmup; threads = $Threads
        wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = ($cpu / $wall)
        bytes = (Get-Item $out).Length; sha256 = $sha; type_counts = ($types | ConvertTo-Json -Compress)
        status = $evidence.CompletionStatus; degraded = $evidence.Degraded; fatal = $evidence.FatalErrorCount; non_fatal = $evidence.NonFatalErrorCount
        open_loop_warnings = $warnings; exit_code = $proc.ExitCode; generator = 'arachne'
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $proc.Dispose()
    Write-Host ("[t44] {0} {1} wall={2:N3}s cpu={3:N3}s ratio={4:N3} sha={5}" -f $Phase, $runId, $wall, $cpu, ($cpu / $wall), $sha)
}
foreach ($arm in @('baseline', 'candidate')) { Invoke-Arm -Arm $arm -Index 0 -Warmup $true }
$proofRows = @(Import-Csv $Csv)
if ($proofRows[0].sha256 -ne $proofRows[1].sha256 -or $proofRows[0].open_loop_warnings -ne $proofRows[1].open_loop_warnings) {
    throw 'baseline/candidate proof mismatch'
}
if ($Phase -eq 'proof') { exit 0 }
for ($i = 1; $i -le $Runs; $i++) {
    $order = if ($i % 2 -eq 1) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { Invoke-Arm -Arm $arm -Index $i -Warmup $false }
}
