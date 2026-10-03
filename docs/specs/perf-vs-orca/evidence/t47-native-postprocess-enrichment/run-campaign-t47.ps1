<#
.SYNOPSIS
    Ticket-47 campaign driver: interleaved integrated-vs-external per cell.

.DESCRIPTION
    Perf-vs-orca ticket 47 (t33 gate re-run from proof after the native
    postprocess-view enrichment repair). schedule = first run both proof arms
    for the cell (arm order alternating by fixture to decorrelate), then
    measured batch: for run i = 1..Runs, order = (external, integrated) when
    i is odd else (integrated, external). Rows append to the cell CSV written
    by run-t47.ps1.

    Summarize+gate with summarize-t47.ps1 afterwards.
.EXAMPLE
    pwsh -NoProfile -File run-campaign-t47.ps1 -Cells benchy-classic-off -Runs 3
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string[]]$Cells,   # e.g. 'benchy-classic-off', 'base-arachne-on'
    [int]$Runs = 3,
    [switch]$SkipProof,
    [ValidateSet('external', 'integrated')]
    [string]$ProofFirstArm = 'external'
)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
foreach ($cell in $Cells) {
    $part = $cell -split '-'
    if ($part.Count -lt 3) { throw "cell must be fixture-generator-supports: $cell" }
    $fixture = $part[0]
    $generator = $part[1]
    $supports = $part[2]
    Write-Host "[t47-campaign] === cell $cell ==="
    if (-not $SkipProof) {
        $order = if ($ProofFirstArm -eq 'external') { @('external', 'integrated') } else { @('integrated', 'external') }
        foreach ($arm in $order) {
            & (Join-Path $here 'run-t47.ps1') -Phase proof -Arm $arm -PairArm $arm `
                -Fixture $fixture -Generator $generator -Supports $supports
            if ($LASTEXITCODE -ne 0) { throw "proof arm failed: $arm on $cell" }
        }
    }
    for ($i = 1; $i -le $Runs; $i++) {
        $order = if ($i % 2 -eq 1) { @('external', 'integrated') } else { @('integrated', 'external') }
        foreach ($arm in $order) {
            & (Join-Path $here 'run-t47.ps1') -Phase measure -Arm $arm -PairArm $arm `
                -Fixture $fixture -Generator $generator -Supports $supports -Index $i -CampaignRuns $Runs
            if ($LASTEXITCODE -ne 0) { throw "measure arm failed: $arm on $cell run $i" }
        }
    }
}
Write-Host '[t47-campaign] complete.'