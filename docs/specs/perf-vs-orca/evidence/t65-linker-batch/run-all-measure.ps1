# t65 timing driver: 6 alternating baseline/candidate pairs per measure cell,
# only after the full 16-cell qualification gate is present. Cells are the
# ticket's pre-declared timing set: the four ARACHNE cells (benchy + base x
# supports off/on — the linker-bearing cells per t62's measured split), both
# modes. Classic cells are excluded by pre-declaration (linker 9.2-9.8% of
# guest fuel there; expected wall effect below the paired spread).
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t65-linker-batch\run-cell.ps1'
$qpath = Join-Path $PSScriptRoot 'qualification.json'
if (-not (Test-Path $qpath)) { Write-Host '[ABORT] qualification.json absent'; exit 1 }
$q = Get-Content $qpath -Raw | ConvertFrom-Json
if (-not $q.pass -or $q.cells.Count -ne 16) { Write-Host '[ABORT] qualification gate not passed'; exit 1 }
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($fixture in @('benchy','base')) {
    foreach ($supports in @('off','on')) {
      $cell = "$mode-$fixture-arachne-$supports"
      $dir = Join-Path $Repo (".local-artifacts\perimeter-reference-preparation\t65-linker-batch\measure\$cell")
      if (Test-Path $dir) { Write-Host "[skip] measure $cell (done)"; continue }
      Write-Host "[measure] START $cell $(Get-Date -Format o)"
      & pwsh -NoProfile -File $RunCell -Mode $mode -Fixture $fixture -Generator arachne -Supports $supports -Phase measure
      $code = $LASTEXITCODE
      Write-Host "[measure] END $cell exit=$code $(Get-Date -Format o)"
      if ($code -ne 0) {
          Write-Host "[STOP] timing failure at $cell (stop discipline: no auto-retry)"
          exit $code
      }
    }
  }
}
Write-Host '[t65] measurement campaign complete'
exit 0