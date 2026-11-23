# t102 pre-gate: 3 alternating pairs on ordinary base classic-on, the ticket's
# pre-declared stop screen. Stops the campaign by rule if wall is not negative
# or CPU exceeds the +3.0 % ceiling.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t102-allocator\run-cell.ps1'
$root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t102-allocator'
$dir = Join-Path $root 'pregate\ordinary-base-classic-on'
if (Test-Path $dir) { Write-Host '[skip] pregate (done)'; exit 0 }
& pwsh -NoProfile -File $RunCell -Mode ordinary -Fixture base -Generator classic -Supports on -Phase pregate
exit $LASTEXITCODE
