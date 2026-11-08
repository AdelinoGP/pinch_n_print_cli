# t78 attribution run: same cell with the stage/module profile table on stderr.
# Instrumented output is never used for wall claims; it only attributes the
# uninstrumented pair and must reproduce the plain run's output hash.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Model,
    [Parameter(Mandatory)][string]$Config,
    [Parameter(Mandatory)][string]$Out,
    [Parameter(Mandatory)][string]$Err
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Repo
$exe = Join-Path $Repo 'target\release\pnp_cli.exe'
$env:RAYON_NUM_THREADS = '12'
$args = @(
    'slice', '--model', $Model, '--config', $Config, '--output', $Out,
    '--module-dir', (Join-Path $Repo 'modules\core-modules'),
    '--no-default-module-paths', '--no-integrated-modules',
    '--instrument-stderr', '--profile', '--profile-verbose'
)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = Start-Process -FilePath $exe -ArgumentList $args -PassThru -NoNewWindow `
    -RedirectStandardError $Err -RedirectStandardOutput ($Out + '.stdout.log')
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw 'timeout' }
$sw.Stop()
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
[pscustomobject][ordered]@{
    exit_code = $p.ExitCode
    wall_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 3)
    bytes = (Get-Item $Out).Length
    sha256 = $sha
} | ConvertTo-Json -Compress
$p.Dispose()
