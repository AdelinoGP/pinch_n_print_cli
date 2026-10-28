# t91 probe capture: one arm, one cell, probe snapshot, `PNP_T91_PROBE=1`.
# Attribution only — never a wall claim. The probe prints one `T91PROBE` line
# with the query / miss / geometry-build counts and the exact-Z loop timings.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline','candidate')][string]$Arm,
    [ValidateSet('ordinary','accelerated')][string]$Mode = 'ordinary',
    [ValidateSet('benchy','base')][string]$Fixture = 'base',
    [ValidateSet('classic','arachne')][string]$Generator = 'classic',
    [ValidateSet('off','on')][string]$Supports = 'on'
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t91-exact-z-memo'
$OutDir = Join-Path $Root 'probe\captures'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Snapshot = Join-Path $Root "probe\snapshots\$Arm"
$Config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
$Id = "$Mode-$Fixture-$Generator-$Supports-$Arm"
$Out = Join-Path $OutDir "$Id.gcode"
$Err = Join-Path $OutDir "$Id.jsonl"
$Stdout = Join-Path $OutDir "$Id.stdout.log"
$slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
    '--output',('"'+$Out+'"'),'--module-dir',(Join-Path $Snapshot 'modules'),
    '--no-default-module-paths','--no-integrated-modules',
    '--instrument-stderr','--profile','--profile-verbose')
$p = Start-Process -FilePath (Join-Path $Snapshot 'pnp_cli.exe') -ArgumentList $slicerArgs `
    -PassThru -NoNewWindow -Environment @{RAYON_NUM_THREADS='12'; PNP_T91_PROBE='1'} `
    -RedirectStandardError $Err -RedirectStandardOutput $Stdout
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Id" }
$p.WaitForExit()
if ($p.ExitCode -ne 0) { throw "exit code $($p.ExitCode) for $Id" }
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
$probe = Select-String -Path $Err -Pattern '^T91PROBE ' | ForEach-Object { $_.Line }
Write-Host "[t91] $Id exit=$($p.ExitCode) sha=$sha bytes=$((Get-Item $Out).Length)"
Write-Host "[t91] $probe"
$p.Dispose()
