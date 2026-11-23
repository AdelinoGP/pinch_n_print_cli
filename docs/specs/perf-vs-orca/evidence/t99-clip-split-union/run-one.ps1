# t99 Phase-1 runner — one slice of the screening cell.
#
# Arms (one host binary, env-gated; the probe patch is reverted before close):
#   probe-prod-first   production whole-layer dilation first, then the
#                      split-and-union counterfactual
#   probe-split-first  the counterfactual first, then production (order check:
#                      a price that only exists in one order is an artifact)
#
# External GetProcessTimes wall + process CPU; output checked against the
# post-t81 ordinary reference for base classic supports-off. Instrumented
# (probe) walls are attribution only, never a wall claim.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('probe-prod-first', 'probe-split-first')][string]$Arm,
    [Parameter(Mandatory)][int]$Run
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t99-clip-split-union'
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
$Rows = Join-Path $OutDir "$Id.rows.csv"
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
Set-Content -LiteralPath $Rows -Value 'call,layer,print_z,operand_polys,operand_verts,prod_ms,offset_pieces_ms,union_ms,split_polys,prod_polys,exact_equal,set_equal,diff_polys'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T99ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$environment = @{
    RAYON_NUM_THREADS = '12'
    PNP_T99_PROBE     = '1'
    PNP_T99_ROWS      = $Rows
}
if ($Arm -eq 'probe-split-first') { $environment['PNP_T99_ORDER'] = 'split' }
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
if (-not [T99ProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T99ProcessTimes]::T($e) - [T99ProcessTimes]::T($c)) / 10000000.0
$cpu = ([T99ProcessTimes]::T($k) + [T99ProcessTimes]::T($u)) / 10000000.0
$ratio = $cpu / $wall
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
$bytes = (Get-Item $Out).Length
$matchesReference = ($sha -eq $ReferenceSha -and $bytes -eq $ReferenceBytes)
$rowCount = (Get-Content -LiteralPath $Rows | Measure-Object -Line).Lines - 1
[pscustomobject][ordered]@{
    id = $Id; mode = $Mode; arm = $Arm; run_index = $Run
    threads = 12; exit_code = $p.ExitCode
    order = $(if ($Arm -eq 'probe-split-first') { 'split-first' } else { 'prod-first' })
    rows = $rowCount
    wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = $ratio
    sha256 = $sha; bytes = $bytes
    reference_sha256 = $ReferenceSha; reference_bytes = $ReferenceBytes
    matches_reference = $matchesReference
    exe_sha256 = (Get-FileHash (Join-Path $Snapshot 'pnp_cli.exe') -Algorithm SHA256).Hash.ToLower()
} | ConvertTo-Json | Set-Content (Join-Path $OutDir "$Id.run.json")
if (-not $matchesReference) { throw "output drift: got $sha ($bytes bytes)" }
Write-Host ("[t99] {0} exit={1} rows={2} wall={3:N3}s cpu={4:N3}s cpu/wall={5:N3} sha={6}" -f $Id, $p.ExitCode, $rowCount, $wall, $cpu, $ratio, $sha)
$p.Dispose()
