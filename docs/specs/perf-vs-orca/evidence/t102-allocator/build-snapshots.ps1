# t102 snapshot builder. Builds one arm's developer snapshot(s) and copies them
# under the t102 local-artifacts root.
#
#   candidate : the working tree (mimalloc as the global allocator)
#   baseline  : the committed tree (the candidate's build inputs are stashed)
#
# The stash is path-limited to the candidate's build inputs and is always
# popped. Never a checkout or a revert. The candidate is host-only, so both arms
# share byte-identical guest modules; only `pnp_cli.exe` differs.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('baseline', 'candidate')][string]$Arm,
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated', 'both')][string]$Modes = 'both'
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Sources = @(
    'crates/pnp-cli/Cargo.toml'
    'crates/pnp-cli/src/main.rs'
    'Cargo.lock'
)
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t102-allocator'
$Snap = Join-Path $Root 'snapshots'
New-Item -ItemType Directory -Force -Path $Snap | Out-Null

$wanted = if ($Modes -eq 'both') { @('ordinary', 'accelerated') } else { @($Modes) }
$stashed = $false
try {
    if ($Arm -eq 'baseline') {
        $dirty = (& git status --porcelain -- $Sources)
        if ($dirty) {
            & git stash push -- $Sources | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'git stash push failed' }
            $stashed = $true
            Write-Host '[t102] candidate build inputs stashed for the baseline arm'
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
        Write-Host "[t102] snapshot $mode-$Arm <- $from"

        # Provenance: the arm's commit, its dirty-candidate source set, the host
        # binary's SHA, and the module tree's per-file SHAs. Both arms must ship
        # byte-identical module trees (the candidate is host-only), so the
        # committed provenance pins that claim.
        $provDir = Join-Path $Root 'provenance'
        New-Item -ItemType Directory -Force -Path $provDir | Out-Null
        $modules = @(
            Get-ChildItem -Path (Join-Path $to 'modules') -Recurse -File -Filter '*.wasm' |
                Sort-Object Name |
                ForEach-Object {
                    [ordered]@{
                        name = $_.Name
                        bytes = $_.Length
                        sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
                    }
                }
        )
        $provenance = [ordered]@{
            arm = $Arm
            mode = $mode
            captured_commit = (& git rev-parse HEAD).Trim()
            candidate_sources_dirty = @(& git status --porcelain -- $Sources | ForEach-Object { $_.Trim() })
            exe_sha256 = (Get-FileHash (Join-Path $to 'pnp_cli.exe') -Algorithm SHA256).Hash.ToLower()
            module_count = $modules.Count
            modules = $modules
        }
        $provenance | ConvertTo-Json -Depth 5 |
            Set-Content -Path (Join-Path $provDir "$mode-$Arm.json") -Encoding utf8
    }
}
finally {
    if ($stashed) {
        & git stash pop
        if ($LASTEXITCODE -ne 0) { throw 'git stash pop failed - recover before continuing' }
    }
}
Write-Host "[t102] snapshots built for $Arm ($($wanted -join ','))"
