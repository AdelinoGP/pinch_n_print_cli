# t85 attribution run: one arm, one cell, `--instrument-stderr`, snapshot
# modules. Attribution only — an instrumented wall is never a wall claim
# (docs/17_agent_debugging.md); this exists to read the `PrePass::SeamPlanning`
# stage wall and the seam-planner module wall on each arm.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline','candidate')][string]$Arm,
    [Parameter(Mandatory)][ValidateSet('ordinary','accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('benchy','base')][string]$Fixture,
    [Parameter(Mandatory)][ValidateSet('classic','arachne')][string]$Generator,
    [Parameter(Mandatory)][ValidateSet('off','on')][string]$Supports,
    [Parameter(Mandatory)][string]$Tag
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t85-seam-inset-batching'
$OutDir = Join-Path $Root "attribution\$Tag"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Snapshot = Join-Path $Root "snapshots\$Mode-$Arm"
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
    -PassThru -NoNewWindow -Environment @{RAYON_NUM_THREADS='12'} `
    -RedirectStandardError $Err -RedirectStandardOutput $Stdout
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $Id" }
$p.WaitForExit()
if ($p.ExitCode -ne 0) { throw "exit code $($p.ExitCode) for $Id" }
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
Write-Host "[t85] $Id exit=$($p.ExitCode) sha=$sha bytes=$((Get-Item $Out).Length)"
$p.Dispose()
