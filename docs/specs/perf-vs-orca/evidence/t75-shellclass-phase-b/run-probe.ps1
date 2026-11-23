# T75 attribution probe runner (requires probe.patch applied to the tree).
# Runs the four instrumented cells recorded in split.json with PNP_T75_PROBE=1.
param(
    [string]$BatchId = 't75-probe',
    [int]$Threads = 12,
    [string]$Only = ''
)
$ErrorActionPreference = 'Stop'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $RepoRoot

$ConfigDir = 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
$ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
$OutDir = Join-Path $RepoRoot ('target\t75\' + $BatchId)
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

$Runs = @(
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'classic'; Supports = 'off' }
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'classic'; Supports = 'on' }
    @{ Fixture = 'tmp\base.stl';     Tag = 'base';   Generator = 'arachne'; Supports = 'on' }
    @{ Fixture = 'tmp\3dbenchy.stl'; Tag = 'benchy'; Generator = 'classic'; Supports = 'on' }
)

if (-not ('ProcessTimesT75Probe' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProcessTimesT75Probe {
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
    if (-not [ProcessTimesT75Probe]::GetProcessTimes($Handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
        throw "GetProcessTimes failed"
    }
    [pscustomobject]@{
        WallSeconds = ([ProcessTimesT75Probe]::T($exit) - [ProcessTimesT75Probe]::T($creation)) / 10000000.0
        CpuSeconds = ([ProcessTimesT75Probe]::T($kernel) + [ProcessTimesT75Probe]::T($user)) / 10000000.0
    }
}

foreach ($run in $Runs) {
    $supportsState = $run.Supports
    $runId = "$($run.Tag)-$($run.Generator)-$supportsState"
    if ($Only -ne '' -and $runId -ne $Only) { continue }
    $configPath = Join-Path $RepoRoot (Join-Path $ConfigDir ('pnp-' + $run.Generator + '-supports-' + $supportsState + '.json'))
    $gcodePath = Join-Path $OutDir ($runId + '.gcode')
    $stderrPath = Join-Path $OutDir ($runId + '.stderr.jsonl')
    Write-Host "[t75] START $runId"
    $env:RAYON_NUM_THREADS = "$Threads"
    $env:PNP_T75_PROBE = '1'
    $proc = Start-Process -FilePath $Exe -ArgumentList @(
        'slice',
        '--model', ('"' + (Join-Path $RepoRoot $run.Fixture) + '"'),
        '--output', ('"' + $gcodePath + '"'),
        '--config', ('"' + $configPath + '"'),
        '--module-dir', ('"' + (Join-Path $RepoRoot $ModuleDir) + '"'),
        '--instrument-stderr'
    ) -PassThru -NoNewWindow -RedirectStandardError $stderrPath
    $proc.WaitForExit()
    $times = Get-ProcessTimesSeconds -Handle $proc.Handle
    $wall = $times.WallSeconds
    $cpu = $times.CpuSeconds
    $exitCode = $proc.ExitCode
    $probeLines = Select-String -Path $stderrPath -Pattern 'T75PROBE' | ForEach-Object { $_.Line }
    $probeLines | Set-Content -Path (Join-Path $OutDir ($runId + '.probe.txt')) -Encoding utf8
    $wallStr = '{0:N2}' -f $wall
    $cpuStr = '{0:N2}' -f $cpu
    Write-Host "[t75] DONE $runId wall=${wallStr}s cpu=${cpuStr}s exit=$exitCode probeLines=$($probeLines.Count)"
    Remove-Item Env:PNP_T75_PROBE
}
Write-Host '[t75] all runs complete'
