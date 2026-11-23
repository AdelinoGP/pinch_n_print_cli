# t20 snapshot builder: builds the four campaign snapshots (baseline/candidate
# x ordinary/accelerated) from the SAME working tree, differing only in the
# candidate change (stash/pop around the baseline build, t58/t65's proven
# dance). Run ONLY after the T20-PROBE has been removed (probe-free tree).
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t20-consume-only'
$Stage = Join-Path $Root 'snapshots'
$Archive = Join-Path $Repo 'target\t20\archive-dist-accelerated-dev'
$CandidateFiles = @(
    'crates/slicer-sdk/src/lib.rs',
    'crates/slicer-core/src/perimeter_spatial.rs',
    'modules/core-modules/classic-perimeters/src/lib.rs',
    'modules/core-modules/classic-perimeters/classic-perimeters.toml',
    'modules/core-modules/arachne-perimeters/src/lib.rs',
    'modules/core-modules/arachne-perimeters/arachne-perimeters.toml',
    'modules/core-modules/overhang-classifier-default/src/lib.rs'
)

# grep-guard: the tree must be probe-free before any measured build
$probeHits = Select-String -Path ($CandidateFiles | Where-Object { $_ -ne 'crates/slicer-sdk/src/lib.rs' }) `
    -Pattern 'T20-PROBE' -SimpleMatch -ErrorAction SilentlyContinue
if ($probeHits) { throw 'T20-PROBE remnants found in the candidate files' }

if (-not (Test-Path $Archive)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Archive) | Out-Null
    Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist-accelerated\developer') $Archive
    Write-Host "[t20] archived standing accelerated dist -> $Archive"
}

Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

Write-Host '== baseline arm (HEAD) =='
git stash push -m 't20-candidate-build' -- $CandidateFiles
try {
    cargo xtask dist --edition developer
    cargo xtask dist --accelerated --edition developer
    Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist\developer') (Join-Path $Stage 'ordinary-baseline')
    Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist-accelerated\developer') (Join-Path $Stage 'accelerated-baseline')
} finally {
    Remove-Item -Recurse -Force (Join-Path $Repo 'target\dist-accelerated\developer') -ErrorAction SilentlyContinue
    Copy-Item -Recurse -Force $Archive (Join-Path $Repo 'target\dist-accelerated\developer')
    git stash pop
}

Write-Host '== candidate arm =='
cargo xtask dist --edition developer
cargo xtask dist --accelerated --edition developer
Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist\developer') (Join-Path $Stage 'ordinary-candidate')
Copy-Item -Recurse -Force (Join-Path $Repo 'target\dist-accelerated\developer') (Join-Path $Stage 'accelerated-candidate')
Remove-Item -Recurse -Force (Join-Path $Repo 'target\dist-accelerated\developer') -ErrorAction SilentlyContinue
Copy-Item -Recurse -Force $Archive (Join-Path $Repo 'target\dist-accelerated\developer')
$ParkedCandidate = Join-Path $Repo 'target\t20\candidate-dist-ordinary'
if (Test-Path $ParkedCandidate) { Remove-Item -Recurse -Force $ParkedCandidate }
Move-Item (Join-Path $Repo 'target\dist\developer') $ParkedCandidate

Write-Host '== exe hashes =='
foreach ($arm in @('baseline','candidate')) {
    foreach ($mode in @('ordinary','accelerated')) {
        $exe = Join-Path $Stage "$mode-$arm\pnp_cli.exe"
        $h = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
        Write-Host "$mode-$arm exe=$h"
    }
}
Write-Host 'SNAPSHOTS_READY'
