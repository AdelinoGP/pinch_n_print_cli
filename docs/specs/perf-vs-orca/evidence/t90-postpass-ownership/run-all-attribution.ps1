# t90 attribution driver: one instrumented capture per arm per measured cell.
# Attribution only — an instrumented wall is never a wall claim
# (docs/17_agent_debugging.md). The captures locate the whole-print clone, the
# working copy's drop and the postpass teardown residual on each arm.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunOne = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t90-postpass-ownership\run-instrumented.ps1'
$tag = 'attribution'
foreach ($mode in @('ordinary','accelerated')) {
  foreach ($generator in @('classic','arachne')) {
    foreach ($arm in @('baseline','candidate')) {
      Write-Host "[attr] START $mode-base-$generator-on $arm $(Get-Date -Format o)"
      & pwsh -NoProfile -File $RunOne -Arm $arm -Mode $mode -Fixture base -Generator $generator -Supports on -Tag $tag
      if ($LASTEXITCODE -ne 0) { Write-Host "[ABORT] attribution failure $mode $generator $arm"; exit $LASTEXITCODE }
    }
  }
}
Write-Host '[t90] attribution captures complete'
exit 0
