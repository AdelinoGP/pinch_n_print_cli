# T27 probe iteration 2 — same four cells + a plain benchy slice with a 54MB-class
# output is NOT rerun; the tail attribution needs base-classic-on only.
# Adds: per-layer Slice internals, phase-B sub-split, tail decomposition (estimate_print).
param(
    [string]$BatchId = 't27-probe2',
    [int]$Threads = 12
)
$ErrorActionPreference = 'Stop'
$RepoRoot = 'D:\slicerProject\pinch_n_print_cli_2'
Set-Location $RepoRoot

$ConfigDir = 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
$ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
$OutDir = Join-Path $RepoRoot ('target\matched-pair\' + $BatchId)
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

$Runs = @(
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'classic'; Supports = 'on' }
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'classic'; Supports = 'off' }
    @{ Fixture = 'tmp\3dbenchy.stl'; Tag = 'benchy'; Generator = 'classic'; Supports = 'on' }
    @{ Fixture = 'tmp\3dbenchy.stl'; Tag = 'benchy'; Generator = 'classic'; Supports = 'off' }
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

function Get-ProcessTimesSeconds {
    param([IntPtr]$Handle)
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exit = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [ProcessTimesT27]::GetProcessTimes($Handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
        throw "GetProcessTimes failed"
    }
    [pscustomobject]@{
        WallSeconds = ([ProcessTimesT27]::T($exit) - [ProcessTimesT27]::T($creation)) / 10000000.0
        CpuSeconds = ([ProcessTimesT27]::T($kernel) + [ProcessTimesT27]::T($user)) / 10000000.0
    }
}

foreach ($run in $Runs) {
    $supportsState = $run.Supports
    $configPath = Join-Path $RepoRoot (Join-Path $ConfigDir ('pnp-' + $run.Generator + '-supports-' + $supportsState + '.json'))
    $runId = "$($run.Tag)-$($run.Generator)-$supportsState"
    $gcodePath = Join-Path $OutDir ($runId + '.gcode')
    $stderrPath = Join-Path $OutDir ($runId + '.stderr.jsonl')
    Write-Host "[t27] START $runId"
    $envMap = @{ RAYON_NUM_THREADS = "$Threads" }
    $proc = Start-Process -FilePath $Exe -ArgumentList @(
        'slice',
        '--model', ('"' + (Join-Path $RepoRoot $run.Fixture) + '"'),
        '--output', ('"' + $gcodePath + '"'),
        '--config', ('"' + $configPath + '"'),
        '--module-dir', ('"' + $ModuleDir + '"')
    ) -PassThru -NoNewWindow -RedirectStandardError $stderrPath -Environment $envMap
    $proc.WaitForExit()
    $times = Get-ProcessTimesSeconds -Handle $proc.Handle
    $wall = $times.WallSeconds
    $cpu = $times.CpuSeconds
    $exitCode = $proc.ExitCode
    $probeLines = Select-String -Path $stderrPath -Pattern 'PERF-T27-PROBE' | ForEach-Object { $_.Line }
    $probeLines | Set-Content -Path (Join-Path $OutDir ($runId + '.probe.txt')) -Encoding utf8
    $wallStr = '{0:N2}' -f $wall
    $cpuStr = '{0:N2}' -f $cpu
    Write-Host "[t27] DONE $runId wall=${wallStr}s cpu=${cpuStr}s exit=$exitCode probeLines=$($probeLines.Count)"
}
Write-Host '[t27] all runs complete'