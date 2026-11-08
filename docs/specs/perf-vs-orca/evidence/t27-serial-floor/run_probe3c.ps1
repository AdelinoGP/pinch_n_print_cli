# T27 probe iteration 3 — SupportAnalysis serial-sweep interior split and the
# exact bbox pre-filter's hit rate, at HEAD (post t19 KEEP). Supports-on cells
# only: the sweep runs only when `enable_support` is true.
# Probe lines are `PERF-T27-PROBE3 ...` on stderr, outside the JSONL contract.
param(
    [string]$BatchId = 't27-probe3c',
    [int]$Threads = 12
)
$ErrorActionPreference = 'Stop'
$RepoRoot = 'D:\slicerProject\pinch_n_print_cli_2'
Set-Location $RepoRoot
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')

$ConfigDir = 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
$ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
$OutDir = Join-Path $RepoRoot ('target\matched-pair\' + $BatchId)
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

$Runs = @(
    @{ Fixture = 'tmp\3dbenchy.stl'; Tag = 'benchy'; Generator = 'classic'; Supports = 'on' }
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'classic'; Supports = 'on' }
)

if (-not ('ProcessTimesT27' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProcessTimesT27 {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h, out System.Runtime.InteropServices.ComTypes.FILETIME c, out System.Runtime.InteropServices.ComTypes.FILETIME e, out System.Runtime.InteropServices.ComTypes.FILETIME k, out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime << 32) + (uint)f.dwLowDateTime;
}
'@
}

$Rows = @()
foreach ($run in $Runs) {
    $supportsState = $run.Supports
    $configPath = Join-Path $RepoRoot (Join-Path $ConfigDir ('pnp-' + $run.Generator + '-supports-' + $supportsState + '.json'))
    $runId = "$($run.Tag)-$($run.Generator)-$supportsState"
    $gcodePath = Join-Path $OutDir ($runId + '.gcode')
    $stderrPath = Join-Path $OutDir ($runId + '.stderr.jsonl')
    Write-Host "[t27-p3] START $runId"
    $envMap = @{ RAYON_NUM_THREADS = "$Threads" }
    $proc = Start-Process -FilePath $Exe -ArgumentList @(
        'slice',
        '--model', ('"' + (Join-Path $RepoRoot $run.Fixture) + '"'),
        '--output', ('"' + $gcodePath + '"'),
        '--config', ('"' + $configPath + '"'),
        '--module-dir', ('"' + $ModuleDir + '"'),
        '--no-default-module-paths',
        '--no-integrated-modules'
    ) -PassThru -NoNewWindow -RedirectStandardError $stderrPath -Environment $envMap
    $handle = $proc.Handle
    if (-not $proc.WaitForExit(1800000)) { $proc.Kill(); throw "slice timed out: $runId" }
    $proc.Refresh()
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exitT = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [ProcessTimesT27]::GetProcessTimes($handle, [ref]$creation, [ref]$exitT, [ref]$kernel, [ref]$user)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([ProcessTimesT27]::T($exitT) - [ProcessTimesT27]::T($creation)) / 10000000.0
    $cpu = ([ProcessTimesT27]::T($kernel) + [ProcessTimesT27]::T($user)) / 10000000.0

    $evidence = Test-MeasurementEvidence -ConfigPath $configPath -OutputPath $gcodePath -StderrPath $stderrPath -ExpectedGenerator $run.Generator -ExitCode $proc.ExitCode
    if ($evidence.CompletionStatus -ne 'ok' -or $evidence.Degraded -or $evidence.FatalErrorCount -ne 0) {
        throw "unclean completion: $runId status=$($evidence.CompletionStatus) degraded=$($evidence.Degraded) fatal=$($evidence.FatalErrorCount)"
    }
    $shadow = 0
    foreach ($line in [IO.File]::ReadLines($stderrPath)) {
        if ($line.Contains('shadows integrated')) { $shadow++ }
    }
    if ($shadow -ne 0) { throw "integrated shadow warning present: $runId ($shadow)" }

    $probeLines = Select-String -Path $stderrPath -Pattern 'PERF-T27-PROBE3' | ForEach-Object { $_.Line }
    $probeLines | Set-Content -Path (Join-Path $OutDir ($runId + '.probe.txt')) -Encoding utf8

    $rows += [pscustomobject][ordered]@{
        run_id = $runId; threads = $Threads; wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = ($cpu / $wall)
        bytes = (Get-Item $gcodePath).Length
        sha256 = (Get-FileHash $gcodePath -Algorithm SHA256).Hash.ToLower()
        status = $evidence.CompletionStatus; degraded = $evidence.Degraded; fatal = $evidence.FatalErrorCount; non_fatal = $evidence.NonFatalErrorCount
        shadow_warnings = $shadow; exit_code = $proc.ExitCode; generator = $run.Generator
        probe_lines = $probeLines.Count
    }
    Write-Host ("[t27-p3] DONE {0} wall={1:N2}s cpu={2:N2}s probeLines={3}" -f $runId, $wall, $cpu, $probeLines.Count)
    $proc.Dispose()
}
$Rows | Export-Csv -Path (Join-Path $OutDir 'rows.csv') -NoTypeInformation -Encoding utf8
Write-Host '[t27-p3] all runs complete'
