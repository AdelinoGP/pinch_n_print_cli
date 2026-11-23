# t79 screening run: one instrumented slice against the in-tree modules, with an
# external wall clock and the process peak working set recorded. Screening only:
# the module bracket is the screening statistic, never a wall claim.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Model,
    [Parameter(Mandatory)][string]$Config,
    [Parameter(Mandatory)][string]$Out,
    [Parameter(Mandatory)][string]$Err
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
Set-Location $Repo
$exe = Join-Path $Repo 'target\release\pnp_cli.exe'
$env:RAYON_NUM_THREADS = '12'
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$args = @(
    'slice', '--model', ('"'+$Model+'"'), '--config', ('"'+$Config+'"'),
    '--output', ('"'+$Out+'"'),
    '--module-dir', (Join-Path $Repo 'modules\core-modules'),
    '--no-default-module-paths', '--no-integrated-modules',
    '--instrument-stderr', '--profile', '--profile-verbose'
)
$p = Start-Process -FilePath $exe -ArgumentList $args -PassThru -NoNewWindow `
    -RedirectStandardError $Err -RedirectStandardOutput ($Out + '.stdout.log')
$peak = 0L
$deadline = (Get-Date).AddSeconds(1800)
while (-not $p.HasExited) {
    if ((Get-Date) -gt $deadline) { $p.Kill(); throw 'timeout' }
    try {
        $p.Refresh()
        if ($p.WorkingSet64 -gt $peak) { $peak = $p.WorkingSet64 }
    } catch { }
    Start-Sleep -Milliseconds 200
}
$p.WaitForExit()
$sw.Stop()
$p.Refresh()
$peakMb = [math]::Round($peak / 1MB, 1)
$valid = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $Out -StderrPath $Err -ExpectedGenerator 'classic' -ExitCode $p.ExitCode
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
[pscustomobject][ordered]@{
    exit_code = $p.ExitCode
    wall_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 3)
    peak_working_set_mb = $peakMb
    bytes = (Get-Item $Out).Length
    sha256 = $sha
    status = $valid.CompletionStatus
    degraded = $valid.Degraded
    fatal = $valid.FatalErrorCount
    non_fatal = $valid.NonFatalErrorCount
} | ConvertTo-Json -Compress
$p.Dispose()
