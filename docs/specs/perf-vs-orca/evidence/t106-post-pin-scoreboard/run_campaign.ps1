<#
.SYNOPSIS
    Ticket 106 campaign: post-pin supports-on Orca scoreboard re-derivation.

.DESCRIPTION
    Drives the standing matched-pair rig once per run, Orca only, on the four
    supports-on cells. The rig's standing default (ticket 105 ruling) pins
    `support_style = tree_strong` for supports-on runs and asserts the resolved
    appendix per row, so every row here proves which style it ran.

    Per cell: 1 warmup + 3 measured (ticket 103's schedule, so the post-pin
    medians are directly comparable to the pre-pin ones). Every run is its own
    batch (`target/matched-pair/t106-<tag>-<fixture>-<cell>`) because the rig's
    run ids are per-batch; the reducer reads the batch dirs, not the ids.

    Measurement-only. The PnP side is not re-run: ticket 103's PnP ordinary
    medians are the denominators, unchanged by a style pin that affects Orca
    only.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/run_campaign.ps1
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/run_campaign.ps1 -Resume
#>
[CmdletBinding()]
param(
    [int]$Rounds = 3,
    [int]$Warmups = 1,
    # Resume a paused campaign: skip any batch whose CSV already holds a row.
    [switch]$Resume,
    # The pin this take prices. Kept explicit (not left to the rig default) so
    # the campaign record names the style it measured.
    [string]$OrcaSupportStyle = 'tree_strong',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'
$Rig = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\run_scoreboard.ps1'
$OutDir = $PSScriptRoot
$OrcaExe = 'C:\Program Files\OrcaSlicer\orca-slicer.exe'

$Cells = @(
    @{ Fixture = 'benchy'; Cell = 'classic-on' },
    @{ Fixture = 'benchy'; Cell = 'arachne-on' },
    @{ Fixture = 'base';   Cell = 'classic-on' },
    @{ Fixture = 'base';   Cell = 'arachne-on' }
)

function Get-LoadPercent {
    try {
        $sample = Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 1 -ErrorAction Stop
        return [math]::Round([double]$sample.CounterSamples[0].CookedValue, 1)
    } catch {
        return -1.0
    }
}

function Invoke-Cell {
    param(
        [hashtable]$Target,
        [string]$BatchId,
        [int]$Runs,
        [int]$Warm
    )
    $batchCsv = Join-Path $RepoRoot ('target\matched-pair\' + $BatchId + '\scoreboard-rows.csv')
    if ($Resume -and (Test-Path -LiteralPath $batchCsv)) {
        $done = @(Import-Csv -LiteralPath $batchCsv).Count
        if ($done -ge 1) {
            Write-Host ("[t106] skip (done) " + $BatchId)
            return
        }
    }
    $rigArgs = @{
        Fixtures         = $Target.Fixture
        Cells            = $Target.Cell
        Tools            = 'orca'
        Runs             = $Runs
        Warmups          = $Warm
        KeepGcode        = 'None'
        SkipFreshness    = $true
        BatchId          = $BatchId
        OrcaSupportStyle = $OrcaSupportStyle
    }
    & pwsh -NoProfile -File $Rig @rigArgs | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "rig failed for $BatchId (exit $LASTEXITCODE)" }
}

$loadLines = @()
$loadLines += ('start ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
$loadLines | Set-Content -Path (Join-Path $OutDir 'load.txt') -Encoding utf8

# Provenance: the binary and vendor tree the campaign actually ran against.
$orcaItem = Get-Item -LiteralPath $OrcaExe
$provenance = [ordered]@{
    orca_exe          = $orcaItem.FullName
    orca_bytes        = $orcaItem.Length
    orca_sha256       = (Get-FileHash -LiteralPath $OrcaExe -Algorithm SHA256).Hash.ToLowerInvariant()
    orca_cli_version  = $null   # filled from the first run's CLI log, below
    orca_support_style = $OrcaSupportStyle
    vendor_manifest   = (Join-Path $RepoRoot 'target\orca-vendor\BBL.json')
    started_utc       = (Get-Date).ToUniversalTime().ToString('o')
    rounds            = $Rounds
    warmups           = $Warmups
}
$provenance | ConvertTo-Json | Set-Content -Path (Join-Path $OutDir 'provenance.json') -Encoding utf8

# Freshness gate: recorded once for the take. The PnP side is not measured here,
# but the map's discipline still wants the gate's exit code on the record.
Push-Location $RepoRoot
try {
    $freshness = & cargo xtask build-guests --check 2>&1 | Out-String
    $freshnessExit = $LASTEXITCODE
} finally { Pop-Location }
@(
    'exit_code ' + $freshnessExit,
    $freshness
) | Set-Content -Path (Join-Path $OutDir 'freshness.txt') -Encoding utf8
Write-Host ("[t106] freshness exit " + $freshnessExit)

foreach ($target in $Cells) {
    for ($w = 1; $w -le $Warmups; $w++) {
        $batch = 't106-w' + $w + '-' + $target.Fixture + '-' + $target.Cell
        Write-Host ("[t106] warmup " + $batch)
        Invoke-Cell -Target $target -BatchId $batch -Runs 0 -Warm 1
    }
}

for ($r = 1; $r -le $Rounds; $r++) {
    $loadLines += ('round-' + $r + '-before ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
    foreach ($target in $Cells) {
        $batch = 't106-r' + $r + '-' + $target.Fixture + '-' + $target.Cell
        Write-Host ("[t106] " + $batch)
        Invoke-Cell -Target $target -BatchId $batch -Runs 1 -Warm 0
    }
    $loadLines += ('round-' + $r + '-after ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
    $loadLines | Set-Content -Path (Join-Path $OutDir 'load.txt') -Encoding utf8
}

$loadLines += ('end ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
$loadLines | Set-Content -Path (Join-Path $OutDir 'load.txt') -Encoding utf8

# Fill the authoritative CLI version string from the first this-take log.
$firstLog = Get-ChildItem -Path (Join-Path $RepoRoot 'target\matched-pair') -Recurse -Filter 'orca-cli.log' -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -like '*t106-*' } |
    Sort-Object LastWriteTime |
    Select-Object -First 1
if ($firstLog) {
    $line = (Get-Content -LiteralPath $firstLog.FullName -TotalCount 1)
    if ($line) {
        $provenance['orca_cli_version'] = $line.Trim()
        $provenance | ConvertTo-Json | Set-Content -Path (Join-Path $OutDir 'provenance.json') -Encoding utf8
    }
}
Write-Host '[t106] campaign complete'
