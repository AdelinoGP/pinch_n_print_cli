# t65 qualification driver: proof run per cell (baseline + candidate, output
# identity), sequential over the full 16-cell matrix. One-shot per cell
# (run-cell.ps1 throws if the cell dir exists), so this driver is resumable
# by re-running: completed cells are skipped by the one-shot guard.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunCell = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t65-linker-batch\run-cell.ps1'
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($fixture in @('benchy','base')) {
    foreach ($generator in @('classic','arachne')) {
      foreach ($supports in @('off','on')) {
        $cell = "$mode-$fixture-$generator-$supports"
        $dir = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t65-linker-batch\proof' $cell
        $dir = Join-Path $Repo (".local-artifacts\perimeter-reference-preparation\t65-linker-batch\proof\$cell")
        if (Test-Path $dir) { Write-Host "[skip] proof $cell (done)"; continue }
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
Write-Host '[t65] qualification proofs complete'
exit 0