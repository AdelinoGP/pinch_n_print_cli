# t89 snapshot builder. Builds one arm's developer snapshot(s) and copies them
# under the t89 local-artifacts root.
#
#   candidate : the working tree (exact-Z validation runs through an
#               order-preserving indexed parallel map)
#   baseline  : the committed tree (the support-aggregation source diff is
#               stashed)
#
# The candidate is HOST-ONLY: both arms execute the same 24 external guests,
# so unlike t85 there is no guest artifact difference between snapshots. The
# stash is path-limited to the one changed source file and is always popped.
# Never a checkout or a revert.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm,
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated', 'both')][string]$Modes = 'both'
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Lib = 'crates/slicer-wasm-host/src/support_aggregation.rs'
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t89-exact-z-parallel'
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
            Write-Host '[t89] support-aggregation source stashed for the baseline arm'
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
        Write-Host "[t89] snapshot $mode-$Arm <- $from"
    }
}
finally {
    if ($stashed) {
        & git stash pop
        if ($LASTEXITCODE -ne 0) { throw 'git stash pop failed - recover before continuing' }
    }
}
Write-Host "[t89] snapshots built for $Arm ($($wanted -join ','))"
