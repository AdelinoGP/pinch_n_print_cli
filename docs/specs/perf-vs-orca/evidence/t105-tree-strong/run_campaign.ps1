<#
.SYNOPSIS
    Ticket 105 campaign: Orca's organic vs `tree_strong` arm on the four
    supports-on cells.

.DESCRIPTION
    Drives the standing matched-pair rig (../matched-pair/run_scoreboard.ps1)
    once per run so the two arms interleave at the run level inside each cell:
    round k alternates the arm order, so drift cannot favour one arm.

    Per cell: 1 warmup + 4 measured per arm. Every run is its own batch
    (target/matched-pair/t105-<arm>-<tag>-<fixture>-<cell>) because the rig's
    run ids are per-batch; the reducer reads the batch dirs, not the ids.

    Measurement-only: nothing here changes the PnP side, the configs, or the
    production tree. The Orca binary is the installed 2.5.0-beta with the
    source-manifest vendor tree prepared by the orca-rig-repair take.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t105-tree-strong/run_campaign.ps1
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t105-tree-strong/run_campaign.ps1 -Resume
#>
[CmdletBinding()]
param(
    [int]$Rounds = 4,
    [int]$Warmups = 1,
    # Resume a paused campaign: skip any batch whose CSV already holds a row.
    # The interrupted run leaves an empty CSV, so it is re-run, not skipped.
    [switch]$Resume,
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

$Arms = @(
    @{ Tag = 'organic'; Style = 'vendor' },      # vendor default -> organic on tree(auto)
    @{ Tag = 'strong';  Style = 'tree_strong' }
)

function Get-LoadPercent {
    try {
        $sample = Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 1 -ErrorAction Stop
        return [math]::Round([double]$sample.CounterSamples[0].CookedValue, 1)
    } catch {
        return -1.0
    }
}

function Invoke-Arm {
    param(
        [hashtable]$Arm,
        [hashtable]$Target,
        [string]$BatchId,
        [int]$Runs,
        [int]$Warm
    )
    $rigArgs = @{
        Fixtures = $Target.Fixture
        Cells    = $Target.Cell
        Tools    = 'orca'
        Runs     = $Runs
        Warmups  = $Warm
        KeepGcode = 'None'
        SkipFreshness = $true
        BatchId  = $BatchId
    }
    if ($Arm.Style) { $rigArgs['OrcaSupportStyle'] = $Arm.Style }
    $batchCsv = Join-Path $RepoRoot ('target\matched-pair\' + $BatchId + '\scoreboard-rows.csv')
    if ($Resume -and (Test-Path -LiteralPath $batchCsv)) {
        $done = @(Import-Csv -LiteralPath $batchCsv).Count
        if ($done -ge 1) {
            Write-Host ("[t105] skip (done) " + $BatchId)
            return
        }
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
    orca_exe         = $orcaItem.FullName
    orca_bytes       = $orcaItem.Length
    orca_sha256      = (Get-FileHash -LiteralPath $OrcaExe -Algorithm SHA256).Hash.ToLowerInvariant()
    # The launcher stub and OrcaSlicer.dll carry no version resource on this
    # install; the CLI log's first line is the authoritative version string and
    # is filled in after the first run (below).
    orca_cli_version = $null
    vendor_manifest  = (Join-Path $RepoRoot 'target\orca-vendor\BBL.json')
    started_utc      = (Get-Date).ToUniversalTime().ToString('o')
    rounds           = $Rounds
    warmups          = $Warmups
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
Write-Host ("[t105] freshness exit " + $freshnessExit)

foreach ($target in $Cells) {
    foreach ($arm in $Arms) {
        for ($w = 1; $w -le $Warmups; $w++) {
            $batch = 't105-' + $arm.Tag + '-w' + $w + '-' + $target.Fixture + '-' + $target.Cell
            Write-Host ("[t105] warmup " + $batch)
            Invoke-Arm -Arm $arm -Target $target -BatchId $batch -Runs 0 -Warm 1
        }
    }
}

for ($r = 1; $r -le $Rounds; $r++) {
    $loadLines += ('round-' + $r + '-before ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
    foreach ($target in $Cells) {
        $order = if ($r % 2 -eq 1) { $Arms } else { @($Arms[1], $Arms[0]) }
        foreach ($arm in $order) {
            $batch = 't105-' + $arm.Tag + '-r' + $r + '-' + $target.Fixture + '-' + $target.Cell
            Write-Host ("[t105] " + $batch)
            Invoke-Arm -Arm $arm -Target $target -BatchId $batch -Runs 1 -Warm 0
        }
    }
    $loadLines += ('round-' + $r + '-after ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
    $loadLines | Set-Content -Path (Join-Path $OutDir 'load.txt') -Encoding utf8
}

$loadLines += ('end ' + (Get-Date -Format o) + ' load_percent ' + (Get-LoadPercent))
$loadLines | Set-Content -Path (Join-Path $OutDir 'load.txt') -Encoding utf8

# Fill the authoritative CLI version string from the first warmup log.
$firstLog = Get-ChildItem -Path (Join-Path $RepoRoot 'target\matched-pair') -Recurse -Filter 'orca-cli.log' -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -like '*t105-*' } |
    Sort-Object LastWriteTime |
    Select-Object -First 1
if ($firstLog) {
    $line = (Get-Content -LiteralPath $firstLog.FullName -TotalCount 1)
    if ($line) {
        $provenance['orca_cli_version'] = $line.Trim()
        $provenance | ConvertTo-Json | Set-Content -Path (Join-Path $OutDir 'provenance.json') -Encoding utf8
    }
}
Write-Host '[t105] campaign complete'
