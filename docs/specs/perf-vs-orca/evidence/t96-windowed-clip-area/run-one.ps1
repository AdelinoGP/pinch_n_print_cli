# t96 Phase-1 runner — one slice of the screening cell.
#
# Arms (one host binary, env-gated; the probe patch is reverted before close):
#   baseline      production whole-layer dilation, no instrumentation
#   window        production windowed dilation, no instrumentation
#   probe         production whole-layer dilation + per-call rows and the
#                 windowed counterfactual timed after it
#   probe-window  production windowed dilation + per-call rows and the
#                 whole-layer counterfactual timed after it (order check)
#
# External GetProcessTimes wall + process CPU; output checked against the
# post-t81 ordinary reference for base classic supports-off. Instrumented
# (probe) walls are attribution only, never a wall claim.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('baseline', 'window', 'probe', 'probe-window')][string]$Arm,
    [Parameter(Mandatory)][int]$Run
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t96-windowed-clip-area'
$OutDir = Join-Path $Root "phase1\$Mode"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Snapshot = Join-Path $Root "snapshots\$Mode"
$Config = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-classic-supports-off.json'
$Model = Join-Path $Repo 'tmp\base.stl'
$ReferenceSha = '292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80'
$ReferenceBytes = 21023739
$Id = "$Mode-$Arm-$Run"
$Out = Join-Path $OutDir "$Id.gcode"
$Err = Join-Path $OutDir "$Id.stderr.log"
$Stdout = Join-Path $OutDir "$Id.stdout.log"
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T96ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$environment = @{ RAYON_NUM_THREADS = '12' }
if ($Arm -in @('window', 'probe-window')) { $environment['PNP_T96_WINDOW'] = '1' }
if ($Arm -in @('probe', 'probe-window')) { $environment['PNP_T96_PROBE'] = '1' }
$slicerArgs = @('slice', '--model', ('"' + $Model + '"'), '--config', ('"' + $Config + '"'),
    '--output', ('"' + $Out + '"'), '--module-dir', (Join-Path $Snapshot 'modules'),
    '--no-default-module-paths', '--no-integrated-modules')
$p = Start-Process -FilePath (Join-Path $Snapshot 'pnp_cli.exe') -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -Environment $environment `
    -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Id" }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T96ProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T96ProcessTimes]::T($e) - [T96ProcessTimes]::T($c)) / 10000000.0
$cpu = ([T96ProcessTimes]::T($k) + [T96ProcessTimes]::T($u)) / 10000000.0
$ratio = $cpu / $wall
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
$bytes = (Get-Item $Out).Length
$matchesReference = ($sha -eq $ReferenceSha -and $bytes -eq $ReferenceBytes)
[pscustomobject][ordered]@{
    id = $Id; mode = $Mode; arm = $Arm; run_index = $Run
    threads = 12; exit_code = $p.ExitCode
    window = [bool]$environment.ContainsKey('PNP_T96_WINDOW')
    probe = [bool]$environment.ContainsKey('PNP_T96_PROBE')
    wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = $ratio
    sha256 = $sha; bytes = $bytes
    reference_sha256 = $ReferenceSha; reference_bytes = $ReferenceBytes
    matches_reference = $matchesReference
    exe_sha256 = (Get-FileHash (Join-Path $Snapshot 'pnp_cli.exe') -Algorithm SHA256).Hash.ToLower()
} | ConvertTo-Json | Set-Content (Join-Path $OutDir "$Id.run.json")
if (-not $matchesReference) { throw "output drift: got $sha ($bytes bytes)" }
Write-Host ("[t96] {0} exit={1} wall={2:N3}s cpu={3:N3}s cpu/wall={4:N3} sha={5}" -f $Id, $p.ExitCode, $wall, $cpu, $ratio, $sha)
$p.Dispose()
