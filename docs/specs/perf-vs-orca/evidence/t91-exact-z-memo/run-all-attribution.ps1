# t91 attribution driver: one instrumented capture per arm per measured cell.
# Attribution only — an instrumented wall is never a wall claim
# (docs/17_agent_debugging.md). The captures split the support-geometry stage
# into planner module walls plus the post-planner gap that carries the exact-Z
# validation loop.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$RunOne = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t91-exact-z-memo\run-instrumented.ps1'
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
Write-Host '[t91] attribution captures complete'
exit 0
