# t85 uninstrumented arm run: one arm, one cell, external GetProcessTimes
# wall+CPU, snapshot modules. Used for the pre-campaign sanity pair (the
# standing gate itself runs through run-cell.ps1 with its full guards).
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline','candidate')][string]$Arm,
    [Parameter(Mandatory)][ValidateSet('ordinary','accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('benchy','base')][string]$Fixture,
    [Parameter(Mandatory)][ValidateSet('classic','arachne')][string]$Generator,
    [Parameter(Mandatory)][ValidateSet('off','on')][string]$Supports,
    [Parameter(Mandatory)][string]$Tag,
    [Parameter(Mandatory)][int]$Index
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t85-seam-inset-batching'
$OutDir = Join-Path $Root "sanity\$Tag"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Snapshot = Join-Path $Root "snapshots\$Mode-$Arm"
$Config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
$Id = "$Mode-$Fixture-$Generator-$Supports-$Arm-$Index"
$Out = Join-Path $OutDir "$Id.gcode"
$Err = Join-Path $OutDir "$Id.stderr.jsonl"
$Stdout = Join-Path $OutDir "$Id.stdout.log"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T85SanityTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
    '--output',('"'+$Out+'"'),'--module-dir',(Join-Path $Snapshot 'modules'),
    '--no-default-module-paths','--no-integrated-modules')
$p = Start-Process -FilePath (Join-Path $Snapshot 'pnp_cli.exe') -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -Environment @{RAYON_NUM_THREADS='12'} `
    -RedirectStandardError $Err -RedirectStandardOutput $Stdout
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Id" }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T85SanityTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T85SanityTimes]::T($e)-[T85SanityTimes]::T($c))/10000000.0
$cpu = ([T85SanityTimes]::T($k)+[T85SanityTimes]::T($u))/10000000.0
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
[pscustomobject][ordered]@{
    run_id=$Id; arm=$Arm; index=$Index; wall_seconds=[math]::Round($wall,3)
    cpu_seconds=[math]::Round($cpu,3); cpu_wall_ratio=[math]::Round($cpu/$wall,3)
    bytes=(Get-Item $Out).Length; sha256=$sha; exit_code=$p.ExitCode
} | ConvertTo-Json -Compress
$p.Dispose()
Remove-Item -Force $Out, $Err, $Stdout -ErrorAction SilentlyContinue
