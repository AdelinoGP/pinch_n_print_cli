# t95 timing driver: 6 alternating baseline/candidate pairs per measure cell,
# only after the full 16-cell qualification gate is present. Cells are the
# ticket's pre-declared timing set: the four base supports-on cells,
# {ordinary, accelerated} x {classic, arachne} — the cells whose support
# validation loop runs the exact-Z queries this candidate batches. A cell is
# complete only when rows.csv has 14 rows (2 warmups + 12 measured); incomplete
# cells abort instead of being silently skipped.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t95-exact-z-batch\run-cell.ps1'
$root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t95-exact-z-batch'
$qpath = Join-Path $PSScriptRoot 'qualification.json'
if (-not (Test-Path $qpath)) { Write-Host '[ABORT] qualification.json absent'; exit 1 }
$q = Get-Content $qpath -Raw | ConvertFrom-Json
if (-not $q.pass -or $q.cells.Count -ne 16) { Write-Host '[ABORT] qualification gate not passed'; exit 1 }
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($generator in @('classic','arachne')) {
    $cell = "$mode-base-$generator-on"
    $dir = Join-Path $root "measure\$cell"
    $rows = Join-Path $dir 'rows.csv'
    if (Test-Path $rows) {
      $count = @(Import-Csv $rows).Count
      if ($count -eq 14) { Write-Host "[skip] measure $cell (complete)"; continue }
      Write-Host "[ABORT] incomplete measure cell $cell rows=$count"
      exit 2
    }
    Write-Host "[measure] START $cell $(Get-Date -Format o)"
    & pwsh -NoProfile -File $RunCell -Mode $mode -Fixture base -Generator $generator -Supports on -Phase measure
    $code = $LASTEXITCODE
    Write-Host "[measure] END $cell exit=$code $(Get-Date -Format o)"
    if ($code -ne 0) {
        Write-Host "[STOP] timing failure at $cell (stop discipline: no auto-retry)"
        exit $code
    }
  }
}
Write-Host '[t95] measurement campaign complete'
exit 0
