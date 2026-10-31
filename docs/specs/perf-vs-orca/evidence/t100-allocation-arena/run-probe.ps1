# t100 Phase-1 screening capture — one probe slice, attribution only.
#
# Runs the probe snapshot's `pnp_cli.exe` on the matched job with
# `PNP_T100_PROBE=1`, records external GetProcessTimes wall + process CPU, the
# output SHA against the post-t81 ordinary reference set, and the `T100` probe
# lines (census + bulk-free counterfactual).
#
# `-Bulk before|after|both` prices a bulk free of `-BulkBytes` bytes in
# `-BulkBlocks` block counts (`1` = a per-print arena, `495` = one arena per
# layer). `-BulkBytes 0` leaves the counterfactual off, which is how the census
# run learns the byte volume the later runs price.
#
# Never a wall claim: the probe's own eprintln calls and the instrumented
# profile flags inflate the run; the wall/CPU recorded here only supplies
# context and the run's own cpu/wall ratio.
[CmdletBinding()]
param(
    [ValidateSet('ordinary','accelerated')][string]$Mode = 'ordinary',
    [ValidateSet('benchy','base')][string]$Fixture = 'base',
    [ValidateSet('classic','arachne')][string]$Generator = 'classic',
    [ValidateSet('off','on')][string]$Supports = 'on',
    [ValidateSet('','before','after','both')][string]$Bulk = '',
    [string]$BulkBytes = '0',
    [string]$BulkBlocks = '1,495',
    [string]$Tag = '',
    [string]$Exe = ''
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t100-allocation-arena'
$OutDir = Join-Path $Root 'probe\captures'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if ($Exe -eq '') { $Exe = Join-Path $Root 'probe\snapshots\probe\pnp_cli.exe' }
$Modules = Join-Path $Root 'probe\snapshots\probe\modules'
$Config = Join-Path $Repo "docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
if ($Tag -eq '') { $Tag = "$Mode-$Fixture-$Generator-$Supports" }
$Out = Join-Path $OutDir "$Tag.gcode"
$Err = Join-Path $OutDir "$Tag.jsonl"
$Stdout = Join-Path $OutDir "$Tag.stdout.log"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T100ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
    '--output',('"'+$Out+'"'),'--module-dir',('"'+$Modules+'"'),
    '--no-default-module-paths','--no-integrated-modules',
    '--instrument-stderr','--profile','--profile-verbose')
$env:RAYON_NUM_THREADS = '12'
$env:PNP_T100_PROBE = '1'
$env:PNP_T100_TAG = $Tag
if ($Bulk -eq '') { Remove-Item Env:\PNP_T100_BULK -ErrorAction SilentlyContinue }
else { $env:PNP_T100_BULK = $Bulk; $env:PNP_T100_BULK_BYTES = $BulkBytes; $env:PNP_T100_BULK_BLOCKS = $BulkBlocks }
$p = Start-Process -FilePath $Exe -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Tag" }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T100ProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T100ProcessTimes]::T($e)-[T100ProcessTimes]::T($c))/10000000.0
$cpu = (([T100ProcessTimes]::T($k)+[T100ProcessTimes]::T($u))/10000000.0)
$sha = (Get-FileHash -LiteralPath $Out -Algorithm SHA256).Hash.ToLower()
$bytes = (Get-Item -LiteralPath $Out).Length
$probe = @(Select-String -LiteralPath $Err -Pattern '^T100 ' | ForEach-Object { $_.Line })
$degraded = (Get-Content -LiteralPath $Err -Raw | Select-String -Pattern 'degraded=true' -Quiet)
$record = [ordered]@{
    id = $Tag
    cell = "$Mode-$Fixture-$Generator-$Supports"
    generator = $Generator
    supports = $Supports
    bulk = $Bulk
    bulk_bytes = $BulkBytes
    bulk_blocks = $BulkBlocks
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
Write-Host "[t100] $Tag wall=$([math]::Round($wall,3))s cpu=$([math]::Round($cpu,3))s sha=$($sha.Substring(0,16)) bytes=$bytes"
$probe | ForEach-Object { Write-Host $_ }
