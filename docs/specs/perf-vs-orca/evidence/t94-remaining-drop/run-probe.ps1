# t94 Phase-1 screening capture — one probe slice, attribution only.
#
# Runs the probe snapshot's `pnp_cli.exe` with `PNP_T94_PROBE=1` on the matched
# job, records external GetProcessTimes wall + process CPU, the output SHA
# against the post-t81 ordinary reference set, and the `T94` probe lines.
# `-Parallel 1` sets `PNP_T94_PARALLEL=1`, which frees the emitted collection
# with `into_par_iter().for_each(drop)` instead of a serial drop. It changes no
# emitted byte; it prices the counterfactual.
#
# Never a wall claim: the probe's own eprintln calls and the instrumented
# profile flags inflate the run; the wall/CPU recorded here only supplies the
# run's own cpu/wall ratio for the accumulated-to-wall conversion.
[CmdletBinding()]
param(
    [ValidateSet('ordinary','accelerated')][string]$Mode = 'ordinary',
    [ValidateSet('benchy','base')][string]$Fixture = 'base',
    [ValidateSet('classic','arachne')][string]$Generator = 'classic',
    [ValidateSet('off','on')][string]$Supports = 'on',
    [ValidateSet(0,1)][int]$Parallel = 0
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t94-remaining-drop'
$OutDir = Join-Path $Root 'probe\captures'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Snapshot = Join-Path $Root 'probe\snapshots\probe'
$Config = Join-Path $Repo "docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
$Arm = if ($Parallel -eq 1) { 'parallel' } else { 'serial' }
$Id = "$Mode-$Fixture-$Generator-$Supports-$Arm"
$Cell = "$Mode-$Fixture-$Generator-$Supports"
$Out = Join-Path $OutDir "$Id.gcode"
$Err = Join-Path $OutDir "$Id.jsonl"
$Stdout = Join-Path $OutDir "$Id.stdout.log"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T94ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
    '--output',('"'+$Out+'"'),'--module-dir',(Join-Path $Snapshot 'modules'),
    '--no-default-module-paths','--no-integrated-modules',
    '--instrument-stderr','--profile','--profile-verbose')
$env:RAYON_NUM_THREADS = '12'
$env:PNP_T94_PROBE = '1'
if ($Parallel -eq 1) { $env:PNP_T94_PARALLEL = '1' } else { Remove-Item Env:\PNP_T94_PARALLEL -ErrorAction SilentlyContinue }
$p = Start-Process -FilePath (Join-Path $Snapshot 'pnp_cli.exe') -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Id" }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T94ProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T94ProcessTimes]::T($e)-[T94ProcessTimes]::T($c))/10000000.0
$cpu = ([T94ProcessTimes]::T($k)+[T94ProcessTimes]::T($u))/10000000.0
$ratio = $cpu / $wall
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
$bytes = (Get-Item $Out).Length
$refs = Get-Content (Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t91-exact-z-memo\references.json') -Raw | ConvertFrom-Json
$reference = $refs.cells.$Cell
if ($null -eq $reference) { throw "no reference for $Id" }
if ($sha -ne $reference.sha256) { throw "output drift: got $sha want $($reference.sha256)" }
if ($bytes -ne [int]$reference.bytes) { throw "output size drift: got $bytes want $($reference.bytes)" }
Write-Host ("[t94] {0} exit={1} wall={2:N3}s cpu={3:N3}s cpu/wall={4:N3} sha={5} bytes={6}" -f $Id,$p.ExitCode,$wall,$cpu,$ratio,$sha,$bytes)
[pscustomobject][ordered]@{
    id=$Id; mode=$Mode; fixture=$Fixture; generator=$Generator; supports=$Supports
    threads=12; exit_code=$p.ExitCode; probe='PNP_T94_PROBE'; parallel=$Parallel
    wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=$ratio
    sha256=$sha; bytes=$bytes; reference_sha256=$reference.sha256
    exe_sha256=(Get-FileHash (Join-Path $Snapshot 'pnp_cli.exe') -Algorithm SHA256).Hash.ToLower()
} | ConvertTo-Json | Set-Content (Join-Path $OutDir "$Id.run.json")
Get-Content $Err | Where-Object { $_ -match '^T94' } | Set-Content (Join-Path $OutDir "$Id.probe.log")
$p.Dispose()
