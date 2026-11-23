# t85 snapshot builder. Builds one arm's developer snapshot(s) and copies them
# under the t85 local-artifacts root.
#
#   candidate : the working tree (seam planner batched in one host call)
#   baseline  : the committed tree (the planner source diff is stashed)
#
# The stash is path-limited to the seam planner source and is always popped.
# Never a checkout or a revert.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm,
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated', 'both')][string]$Modes = 'both'
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Lib = 'modules/core-modules/seam-planner-default/src/lib.rs'
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t85-seam-inset-batching'
$Snap = Join-Path $Root 'snapshots'
New-Item -ItemType Directory -Force -Path $Snap | Out-Null

$wanted = if ($Modes -eq 'both') { @('ordinary', 'accelerated') } else { @($Modes) }
$stashed = $false
try {
    if ($Arm -eq 'baseline') {
        $dirty = (& git status --porcelain -- $Lib)
        if ($dirty) {
            & git stash push -- $Lib | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'git stash push failed' }
            $stashed = $true
            Write-Host '[t85] planner source stashed for the baseline arm'
        }
    }
    if ($wanted -contains 'ordinary') {
        & cargo xtask dist --edition developer
        if ($LASTEXITCODE -ne 0) { throw 'ordinary dist failed' }
    }
    if ($wanted -contains 'accelerated') {
        & cargo xtask dist --accelerated --edition developer
        if ($LASTEXITCODE -ne 0) { throw 'accelerated dist failed' }
    }
    foreach ($mode in $wanted) {
        $from = if ($mode -eq 'ordinary') { 'target\dist\developer' } else { 'target\dist-accelerated\developer' }
        $to = Join-Path $Snap "$mode-$Arm"
        if (Test-Path $to) { Remove-Item -Recurse -Force $to }
        Copy-Item -Recurse -Force (Join-Path $Repo $from) $to
        Write-Host "[t85] snapshot $mode-$Arm <- $from"
    }
}
finally {
    if ($stashed) {
        & git stash pop
        if ($LASTEXITCODE -ne 0) { throw 'git stash pop failed - recover before continuing' }
    }
}
Write-Host "[t85] snapshots built for $Arm ($($wanted -join ','))"
