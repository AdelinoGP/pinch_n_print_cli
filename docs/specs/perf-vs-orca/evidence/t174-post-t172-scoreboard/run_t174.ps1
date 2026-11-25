<#
.SYNOPSIS
    Run the t174 three-tool eight-cell scoreboard campaign with 20 s box-load
    sampling into target/t174-load.csv.
#>
[CmdletBinding()]
param(
    [string]$BatchId = 't174-post-t172-scoreboard',
    [int]$Runs = 3,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [int]$LoadIntervalSec = 20
)

$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$ScoreboardScript = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\run_scoreboard.ps1'
$LoadCsv = Join-Path $RepoRoot ("target\" + $BatchId + "-load.csv")

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LoadCsv) | Out-Null
Remove-Item -Force -ErrorAction SilentlyContinue $LoadCsv

$loadJob = Start-Job -ArgumentList $LoadCsv, $LoadIntervalSec -ScriptBlock {
    param([string]$OutPath, [int]$Interval)
    while ($true) {
        try {
            $ts = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ss')
            $cpu = [int][math]::Round(((Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average))
            $mem = (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory
            Add-Content -Path $OutPath -Value "$ts,$cpu,$mem" -Encoding utf8
        } catch {
            # best-effort sampler; never abort campaign
        }
        Start-Sleep -Seconds $Interval
    }
}

try {
    & $ScoreboardScript `
        -Fixtures benchy,base `
        -Cells classic-off,classic-on,arachne-off,arachne-on `
        -Tools orca,pnp-ordinary,pnp-accelerated `
        -Runs $Runs `
        -Warmups $Warmups `
        -Threads $Threads `
        -BatchId $BatchId `
        -KeepGcode Last
    if ($LASTEXITCODE -ne 0) {
        throw "run_scoreboard.ps1 exited with code $LASTEXITCODE"
    }
} finally {
    if ($loadJob) {
        Stop-Job -Job $loadJob -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $loadJob -Force -ErrorAction SilentlyContinue | Out-Null
    }
}
