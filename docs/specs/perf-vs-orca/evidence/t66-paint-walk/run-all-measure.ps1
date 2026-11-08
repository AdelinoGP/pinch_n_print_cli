# t66 timing driver: 6 alternating baseline/candidate pairs per measure cell,
# only after the full 16-cell qualification gate is present. Cells are the
# ticket's pre-declared timing set: the standing four-cell set — classic
# supports-on (benchy + base, ordinary + accelerated). The dead paint walk
# lives in the supports-on renderer (t64); the renderer split is
# generator-invariant, so arachne-on cells carry the same effect class and
# are deliberately excluded from this pre-declaration (their outputs remain
# covered by the 16-cell qualification).
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t66-paint-walk\run-cell.ps1'
$qpath = Join-Path $PSScriptRoot 'qualification.json'
if (-not (Test-Path $qpath)) { Write-Host '[ABORT] qualification.json absent'; exit 1 }
$q = Get-Content $qpath -Raw | ConvertFrom-Json
if (-not $q.pass -or $q.cells.Count -ne 16) { Write-Host '[ABORT] qualification gate not passed'; exit 1 }
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($fixture in @('benchy','base')) {
    $cell = "$mode-$fixture-classic-on"
    $dir = Join-Path $Repo (".local-artifacts\perimeter-reference-preparation\t66-paint-walk\measure\$cell")
    if (Test-Path $dir) { Write-Host "[skip] measure $cell (done)"; continue }
    Write-Host "[measure] START $cell $(Get-Date -Format o)"
    & pwsh -NoProfile -File $RunCell -Mode $mode -Fixture $fixture -Generator classic -Supports on -Phase measure
    $code = $LASTEXITCODE
    Write-Host "[measure] END $cell exit=$code $(Get-Date -Format o)"
    if ($code -ne 0) {
        Write-Host "[STOP] timing failure at $cell (stop discipline: no auto-retry)"
        exit $code
    }
  }
}
Write-Host '[t66] measurement campaign complete'
exit 0
