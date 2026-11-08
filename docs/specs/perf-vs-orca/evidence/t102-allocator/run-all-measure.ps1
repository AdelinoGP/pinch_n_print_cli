# t102 timing driver: 6 alternating baseline/candidate pairs per measure cell,
# only after the full 16-cell qualification gate is present. Cells are the
# ticket's pre-declared timing set: the four base supports-on cells,
# {ordinary, accelerated} x {classic, arachne}. A cell is complete only when
# rows.csv has 16 rows (2 warmups x 2 arms + 12 measured); incomplete cells
# abort instead of being silently skipped.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t102-allocator\run-cell.ps1'
$root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t102-allocator'
$qpath = Join-Path $PSScriptRoot 'qualification.json'
if (-not (Test-Path $qpath)) { Write-Host '[ABORT] qualification.json absent'; exit 1 }
$q = Get-Content $qpath -Raw | ConvertFrom-Json
if (-not $q.pass -or $q.cells.Count -ne 16) { Write-Host '[ABORT] qualification gate not passed'; exit 1 }
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($generator in @('classic','arachne')) {
    $cell = "$mode-base-$generator-on"
    $rows = Join-Path $root "measure\$cell\rows.csv"
    if (Test-Path $rows) {
      $count = @(Import-Csv $rows).Count
      if ($count -eq 16) { Write-Host "[skip] measure $cell (complete)"; continue }
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
Write-Host '[t102] measurement campaign complete'
exit 0
