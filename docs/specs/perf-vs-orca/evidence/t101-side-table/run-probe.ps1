# t101 Phase-1 screening capture — one probe slice, table shape + drop census.
#
# Runs the probe snapshot's `pnp_cli.exe` on the matched job with
# `PNP_T101_PROBE=1`, records external GetProcessTimes wall + process CPU, the
# output SHA against the post-t81 ordinary reference set, and the `T101` probe
# lines (shape census, top bodies, drop census).
#
# `-SkipTable` sets `PNP_T101_SKIP_TABLE=1`, which makes the assembly closure
# build no `SupportEntityIdentity` row at all. The A/B pair (set vs unset) is
# the table's in-situ price; both arms must emit byte-identical G-code.
#
# Never a wall claim for the instrumented runs themselves: the probe's own
# stderr lines are small, but the standing profile flags inflate the run.
[CmdletBinding()]
param(
    [ValidateSet('ordinary','accelerated')][string]$Mode = 'ordinary',
    [ValidateSet('benchy','base')][string]$Fixture = 'base',
    [ValidateSet('classic','arachne')][string]$Generator = 'classic',
    [ValidateSet('off','on')][string]$Supports = 'on',
    [switch]$SkipTable,
    [switch]$Census,
    [switch]$NoProfile,
    [string]$Tag = '',
    [string]$Exe = ''
)
$ErrorActionPreference = 'Stop'
# Module autoloading is disabled in this environment; Get-FileHash lives in the
# Utility module, so import it explicitly.
Import-Module Microsoft.PowerShell.Utility -ErrorAction SilentlyContinue
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t101-side-table'
$OutDir = Join-Path $Root 'probe\captures'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if ($Exe -eq '') { $Exe = Join-Path $Root 'probe\snapshots\probe\pnp_cli.exe' }
$Modules = Join-Path $Root 'probe\snapshots\probe\modules'
$Config = Join-Path $Repo "docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
if ($Tag -eq '') {
    $Tag = "$Mode-$Fixture-$Generator-$Supports"
    if ($SkipTable) { $Tag += '-skipped' }
}
$Out = Join-Path $OutDir "$Tag.gcode"
$Err = Join-Path $OutDir "$Tag.jsonl"
$Stdout = Join-Path $OutDir "$Tag.stdout.log"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T101ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
    '--output',('"'+$Out+'"'),'--module-dir',('"'+$Modules+'"'),
    '--no-default-module-paths','--no-integrated-modules','--instrument-stderr')
if (-not $NoProfile) { $slicerArgs += @('--profile','--profile-verbose') }
$env:RAYON_NUM_THREADS = '12'
$env:PNP_T101_PROBE = '1'
$env:PNP_T101_TAG = $Tag
if ($SkipTable) { $env:PNP_T101_SKIP_TABLE = '1' } else { Remove-Item Env:\PNP_T101_SKIP_TABLE -ErrorAction SilentlyContinue }
if ($Census) { $env:PNP_T101_CENSUS = '1' } else { Remove-Item Env:\PNP_T101_CENSUS -ErrorAction SilentlyContinue }
$p = Start-Process -FilePath $Exe -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Tag" }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T101ProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T101ProcessTimes]::T($e)-[T101ProcessTimes]::T($c))/10000000.0
$cpu = (([T101ProcessTimes]::T($k)+[T101ProcessTimes]::T($u))/10000000.0)
$sha = (Get-FileHash -LiteralPath $Out -Algorithm SHA256).Hash.ToLower()
$bytes = (Get-Item -LiteralPath $Out).Length
$probe = @(Select-String -LiteralPath $Err -Pattern '^T101 ' | ForEach-Object { $_.Line })
$degraded = (Get-Content -LiteralPath $Err -Raw | Select-String -Pattern 'degraded=true' -Quiet)
$record = [ordered]@{
    id = $Tag
    cell = "$Mode-$Fixture-$Generator-$Supports"
    generator = $Generator
    supports = $Supports
    skip_table = [bool]$SkipTable
    census = [bool]$Census
    profile = -not $NoProfile
    exit_code = $p.ExitCode
    wall_s = $wall
    cpu_s = $cpu
    cpu_wall = if ($wall -gt 0) { $cpu / $wall } else { 0 }
    sha256 = $sha
    bytes = $bytes
    degraded = [bool]$degraded
    exe_sha = (Get-FileHash -LiteralPath $Exe -Algorithm SHA256).Hash.ToLower()
    probe = $probe
}
$json = Join-Path $OutDir "$Tag.json"
$record | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $json
Write-Host "[t101] $Tag wall=$([math]::Round($wall,3))s cpu=$([math]::Round($cpu,3))s sha=$($sha.Substring(0,16)) bytes=$bytes skip=$([bool]$SkipTable)"
$probe | ForEach-Object { Write-Host $_ }
