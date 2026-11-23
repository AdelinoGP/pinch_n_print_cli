# t93 paired A/B driver: the four pre-declared base classic cells,
# {ordinary, accelerated} x {supports off, on}, 2 warmups + 6 alternating pairs
# each. Requires qualification.json with all 16 proof cells. Resumable by the
# one-shot cell guard in run-cell.ps1.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t93-deep-infill-offsets\run-cell.ps1'
$root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t93-deep-infill-offsets'
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($supports in @('off','on')) {
    $cell = "$mode-base-classic-$supports"
    $dir = Join-Path $root "measure\$cell"
    if (Test-Path $dir) { Write-Host "[skip] measure $cell (done)"; continue }
    Write-Host "[measure] START $cell $(Get-Date -Format o)"
    & pwsh -NoProfile -File $RunCell -Mode $mode -Fixture base -Generator classic -Supports $supports -Phase measure
    $code = $LASTEXITCODE
    Write-Host "[measure] END $cell exit=$code $(Get-Date -Format o)"
    if ($code -ne 0) {
        Write-Host "[ABORT] measurement failure at $cell"
        exit $code
    }
  }
}
Write-Host '[t93] paired A/B complete'
exit 0
