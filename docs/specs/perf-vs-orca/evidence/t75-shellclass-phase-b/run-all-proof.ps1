# t75 qualification driver: proof run per cell (baseline + candidate, output
# identity), sequential over the full 16-cell matrix. One-shot per cell
# (run-cell.ps1 throws if the cell dir exists), so this driver is resumable
# by re-running: completed cells are skipped when rows.csv exists.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t75-shellclass-phase-b\run-cell.ps1'
$root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t75-shellclassification-phase-b'
Write-Host "[t75] driver root=$root repo=$Repo"
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($fixture in @('benchy','base')) {
    foreach ($generator in @('classic','arachne')) {
      foreach ($supports in @('off','on')) {
        $cell = "$mode-$fixture-$generator-$supports"
        $dir = Join-Path $root "proof\$cell"
        if (Test-Path (Join-Path $dir 'rows.csv')) { Write-Host "[skip] proof $cell (done)"; continue }
        Write-Host "[proof] START $cell $(Get-Date -Format o)"
        & pwsh -NoProfile -File $RunCell -Mode $mode -Fixture $fixture -Generator $generator -Supports $supports -Phase proof
        $code = $LASTEXITCODE
        Write-Host "[proof] END $cell exit=$code $(Get-Date -Format o)"
        if ($code -ne 0) {
            Write-Host "[ABORT] qualification failure at $cell"
            exit $code
        }
      }
    }
  }
}
Write-Host '[t75] qualification proofs complete'
exit 0
