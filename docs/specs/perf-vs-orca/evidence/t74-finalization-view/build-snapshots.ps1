# t74 snapshot builder: builds the ordinary + accelerated developer snapshots
# for one arm and copies them under the ticket's gitignored evidence root.
# Run with the working tree in the state that arm should represent.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t74-finalization-view'
$Snap = Join-Path $Root 'snapshots'
Push-Location $Repo
try {
    & cargo xtask build-guests --check *> (Join-Path $Repo "target\t74\build-guests-$Arm-ordinary.log")
    if ($LASTEXITCODE -ne 0) { throw "ordinary guest freshness exit $LASTEXITCODE" }
    & cargo xtask build-guests --accelerated --check *> (Join-Path $Repo "target\t74\build-guests-$Arm-accelerated.log")
    if ($LASTEXITCODE -ne 0) { throw "accelerated guest freshness exit $LASTEXITCODE" }

    foreach ($mode in @('ordinary', 'accelerated')) {
        $distArgs = @('xtask', 'dist', '--edition', 'developer')
        if ($mode -eq 'accelerated') { $distArgs += '--accelerated' }
        Write-Host "[t74-build] $Arm/$mode : cargo $($distArgs -join ' ') $(Get-Date -Format o)"
        & cargo @distArgs *> (Join-Path $Repo "target\t74\dist-$Arm-$mode.log")
        if ($LASTEXITCODE -ne 0) { throw "$Arm/$mode dist exit $LASTEXITCODE" }
        $src = if ($mode -eq 'accelerated') {
            Join-Path $Repo 'target\dist-accelerated\developer'
        } else {
            Join-Path $Repo 'target\dist\developer'
        }
        $dest = Join-Path $Snap "$mode-$Arm"
        if (Test-Path $dest) { throw "one-shot guard: snapshot exists: $dest" }
        New-Item -ItemType Directory -Path $dest | Out-Null
        Copy-Item -Recurse -Force (Join-Path $src '*') $dest
        $exe = Join-Path $dest 'pnp_cli.exe'
        if (-not (Test-Path $exe)) { throw "$Arm/$mode snapshot has no pnp_cli.exe" }
        $sha = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLowerInvariant()
        Write-Host "[t74-build] $Arm/$mode staged $(Get-Date -Format o) exe=$sha"
    }
    $commit = (& git rev-parse HEAD).Trim()
    $status = @(& git status --porcelain) -join "`n"
    $modules = @(Get-ChildItem (Join-Path $Snap "ordinary-$Arm\modules") -Recurse -File |
        Sort-Object FullName | ForEach-Object {
            [pscustomobject]@{
                path = [IO.Path]::GetRelativePath($Snap, $_.FullName).Replace('\', '/')
                sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
        })
    [ordered]@{
        arm = $Arm; commit = $commit; built_at = (Get-Date -Format o)
        working_tree_status = $status
        snapshots = [ordered]@{
            "ordinary-$Arm" = [ordered]@{
                exe_sha256 = (Get-FileHash (Join-Path $Snap "ordinary-$Arm\pnp_cli.exe") -Algorithm SHA256).Hash.ToLowerInvariant()
            }
            "accelerated-$Arm" = [ordered]@{
                exe_sha256 = (Get-FileHash (Join-Path $Snap "accelerated-$Arm\pnp_cli.exe") -Algorithm SHA256).Hash.ToLowerInvariant()
            }
        }
        modules = $modules
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $Snap "provenance-$Arm.json") -Encoding utf8
    Write-Host "[t74-build] $Arm snapshots complete"
} finally { Pop-Location }
exit 0
