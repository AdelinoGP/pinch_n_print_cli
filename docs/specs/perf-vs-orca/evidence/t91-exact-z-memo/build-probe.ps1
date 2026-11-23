# t91 probe snapshot builder (attribution only). Builds one arm's developer
# snapshot with the t91 probe applied, stages it under the t91 local-artifacts
# root, and restores the tree.
#
#   baseline  : HEAD's exact_z_query.rs, stashed out of the working tree
#   candidate : the t91 memo, restored from a byte backup afterwards
#
# Never a commit. Both files the probe touches are restored in the `finally`
# block, and the candidate arm's restore is a byte copy of the pre-probe file.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Src = 'crates/slicer-wasm-host/src/exact_z_query.rs'
$Agg = 'crates/slicer-wasm-host/src/support_aggregation.rs'
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t91-exact-z-memo'
$Probe = Join-Path $Root 'probe'
$To = Join-Path $Probe "snapshots\$Arm"
$Backup = Join-Path $Probe 'candidate-exact_z_query.rs.bak'
New-Item -ItemType Directory -Force -Path $Probe | Out-Null

$stashed = $false
try {
    if ($Arm -eq 'baseline') {
        & git stash push -- $Src | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'git stash push failed' }
        $stashed = $true
    }
    else {
        Copy-Item -Force $Src $Backup
    }
    & python (Join-Path $PSScriptRoot 'probe.py') apply --variant $Arm
    if ($LASTEXITCODE -ne 0) { throw 'probe patch failed' }
    & cargo xtask dist --edition developer
    if ($LASTEXITCODE -ne 0) { throw 'ordinary dist failed' }
    if (Test-Path $To) { Remove-Item -Recurse -Force $To }
    Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist\developer') $To
    Write-Host "[t91] probe snapshot $Arm <- target\dist\developer"
}
finally {
    if ($Arm -eq 'baseline') {
        & git checkout -- $Src
        & git checkout -- $Agg
        if ($stashed) {
            & git stash pop
            if ($LASTEXITCODE -ne 0) { throw 'git stash pop failed - recover before continuing' }
        }
    }
    else {
        Copy-Item -Force $Backup $Src
        & git checkout -- $Agg
    }
    Write-Host "[t91] tree restored after probe build ($Arm)"
}
