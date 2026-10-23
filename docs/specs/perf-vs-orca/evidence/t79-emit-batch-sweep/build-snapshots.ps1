# t79 snapshot builder. Builds one arm's complete ordinary + accelerated
# developer snapshots and copies them under the t79 local-artifacts root.
#
#   baseline  : the committed `4 / 256` tree (the planner source diff is stashed)
#   candidate : the chosen constant pair in the working tree
#
# The stash is path-limited to the planner source and is always popped. Never a
# checkout or a revert.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm,
    [Parameter(Mandatory)][int]$E,
    [Parameter(Mandatory)][int]$C
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Lib = 'modules/core-modules/tree-support-planner/src/lib.rs'
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t79-emit-batch-sweep'
$Snap = Join-Path $Root 'snapshots'
New-Item -ItemType Directory -Force -Path $Snap | Out-Null
$Src = [IO.File]::ReadAllText((Join-Path $Repo $Lib))
$Src = $Src -replace 'const EMIT_BATCH_LAYERS: usize = \d+;', "const EMIT_BATCH_LAYERS: usize = $E;"
$Src = $Src -replace 'const CLIP_BATCH_MAX_REQUESTS: usize = \d+;', "const CLIP_BATCH_MAX_REQUESTS: usize = $C;"
[IO.File]::WriteAllText((Join-Path $Repo $Lib), $Src)

$stashed = $false
try {
    if ($Arm -eq 'baseline') {
        $dirty = (& git status --porcelain -- $Lib)
        if ($dirty) {
            & git stash push -- $Lib | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'git stash push failed' }
            $stashed = $true
        }
    }
    & cargo xtask dist --edition developer
    if ($LASTEXITCODE -ne 0) { throw 'ordinary dist failed' }
    & cargo xtask dist --accelerated --edition developer
    if ($LASTEXITCODE -ne 0) { throw 'accelerated dist failed' }
    foreach ($mode in @('ordinary', 'accelerated')) {
        $from = if ($mode -eq 'ordinary') { 'target\dist\developer' } else { 'target\dist-accelerated\developer' }
        $to = Join-Path $Snap "$mode-$Arm"
        if (Test-Path $to) { Remove-Item -Recurse -Force $to }
        Copy-Item -Recurse -Force (Join-Path $Repo $from) $to
    }
}
finally {
    if ($stashed) {
        & git stash pop
        if ($LASTEXITCODE -ne 0) { throw 'git stash pop failed - recover before continuing' }
    }
}
Write-Host "[t79] snapshots built for $Arm (e=$E c=$C)"
