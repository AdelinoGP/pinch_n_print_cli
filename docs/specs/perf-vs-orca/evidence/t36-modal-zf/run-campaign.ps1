# Campaign driver for wayfinder ticket 36: runs the protocol batches in order.
#
#   pwsh -NoProfile -File run-campaign.ps1 -BatchId 20261003-a
#
# Protocol (docs/specs/perf-vs-orca/evidence/t36-modal-zf/protocol.json):
# fixtures benchy, cells classic-off + arachne-off, modes ordinary + accelerated,
# 1 warmup per arm and 6 measured pairs per (fixture, cell, mode).
[ CmdletBinding() ]
param(
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [int]$Runs = 6,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
$Driver = Join-Path $PSScriptRoot 'run-ab.ps1'

foreach ($fixture in @('benchy')) {
    foreach ($cell in @('classic-off', 'arachne-off')) {
        foreach ($mode in @('ordinary', 'accelerated')) {
            Write-Host "=== t36 campaign: $fixture $cell $mode (batch $BatchId) ==="
            & $Driver -Fixture $fixture -Cell $cell -Mode $mode -Runs $Runs -Warmups $Warmups -Threads $Threads -BatchId $BatchId -RepoRoot $RepoRoot
            if (-not $?) { throw "driver failed: $fixture/$cell/$mode" }
        }
    }
}

Write-Host "[t36] campaign complete. Reduce with:"
Write-Host "  python docs/specs/perf-vs-orca/evidence/t36-modal-zf/verify-t36.py --reduce target/t36-ab"
