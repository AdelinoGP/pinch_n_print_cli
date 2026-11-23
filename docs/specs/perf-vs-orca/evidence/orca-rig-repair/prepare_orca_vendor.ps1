# Materialize the OrcaSlicer vendor preset tree the scoreboard rig loads.
#
# Why this exists (perf-vs-orca ticket 98): OrcaSlicer 2.5 resolves `--load-settings`
# through the vendor preset bundle, not through bare JSON file paths. A preset whose
# JSON says `"from": "system"` is resolved by walking up from the file to the vendor
# directory and reading the sibling `<Vendor>.json` manifest; that path is what
# flattens the preset's `inherits`/`include` chain against the real vendor values.
# The shipped 2.5 install carries only the binary cache `resources/profiles/BBL.opc`
# and no JSON tree, so a file-path run fails with
#   "can not resolve preset <path>: Preset was not found in the loaded bundle".
#
# This script builds that JSON tree at a pinned upstream tag and refuses to hand it
# to the rig unless its declared profile version equals the version stamped in the
# installed `.opc` cache. That is the provenance gate: the rig must measure the same
# vendor content the installed slicer would use, not a different upstream revision.
#
# Output: <VendorRoot>\BBL.json, <VendorRoot>\BBL\**, <VendorRoot>\OrcaFilamentLibrary.json,
#         <VendorRoot>\OrcaFilamentLibrary\**  (default VendorRoot = target\orca-vendor,
#         gitignored: the tree is regenerable, ~16 MB, and upstream-licensed).
#
# Re-run when the installed OrcaSlicer's vendor version changes; the script fails
# closed on a version mismatch and names the tag it would need.
#
# Requires: git and network access to github.com. The rig itself does not need either.

[CmdletBinding()]
param(
    [string]$Tag = 'v2.5.0-beta',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))))),
    [string]$VendorRoot = '',
    [string]$OrcaProfileDir = 'C:\Program Files\OrcaSlicer\resources\profiles',
    [switch]$KeepSource
)

$ErrorActionPreference = 'Stop'

if (-not $VendorRoot) { $VendorRoot = Join-Path $RepoRoot 'target\orca-vendor' }
$SourceDir = Join-Path $RepoRoot 'target\orca-vendor-src'

function ConvertTo-VersionParts {
    param([Parameter(Mandatory = $true)][string]$Version)
    # "02.08.00.16" (profile JSON) and "2.8.0.16" (cache stamp) are the same version.
    return @($Version.Trim() -split '\.' | ForEach-Object { [int]$_ })
}

function Read-InstalledVendorVersion {
    param([Parameter(Mandatory = $true)][string]$CachePath)
    if (-not (Test-Path -LiteralPath $CachePath)) {
        throw "missing-artifact: $CachePath (is OrcaSlicer installed at the expected path?)"
    }
    # The cache header carries the vendor name and version as length-prefixed
    # strings next to the magic "ZCRO"; the version is the first dotted quad.
    $bytes = [System.IO.File]::ReadAllBytes($CachePath)[0..127]
    $head = -join ($bytes | ForEach-Object { [char]$_ })
    $m = [regex]::Match($head, '(\d+\.\d+\.\d+\.\d+)')
    if (-not $m.Success) { throw "could not read a vendor version stamp from $CachePath" }
    return $m.Groups[1].Value
}

# --- 1. Installed vendor version (the provenance target) ----------------------
$cachePath = Join-Path $OrcaProfileDir 'BBL.opc'
$installedVersion = Read-InstalledVendorVersion -CachePath $cachePath
Write-Host "[prepare-orca-vendor] installed BBL.opc stamp : $installedVersion"

# --- 2. Pinned upstream tree --------------------------------------------------
$manifestInTree = Join-Path $SourceDir 'resources\profiles\BBL.json'
$needFetch = $true
if (Test-Path -LiteralPath $manifestInTree) {
    Push-Location $SourceDir
    try {
        $currentTag = (& git describe --tags --exact-match 2>$null)
        if ($LASTEXITCODE -eq 0 -and $currentTag -eq $Tag) { $needFetch = $false }
    } finally { Pop-Location }
}

if ($needFetch) {
    if (Test-Path -LiteralPath $SourceDir) { Remove-Item -LiteralPath $SourceDir -Recurse -Force }
    Write-Host "[prepare-orca-vendor] fetching $Tag (blobless sparse checkout)"
    & git clone --filter=blob:none --sparse --depth 1 --branch $Tag https://github.com/OrcaSlicer/OrcaSlicer.git $SourceDir
    if ($LASTEXITCODE -ne 0) { throw "git clone failed for $Tag" }
    Push-Location $SourceDir
    try {
        & git sparse-checkout set --skip-checks resources/profiles/BBL resources/profiles/BBL.json resources/profiles/OrcaFilamentLibrary resources/profiles/OrcaFilamentLibrary.json
        if ($LASTEXITCODE -ne 0) { throw "git sparse-checkout failed" }
    } finally { Pop-Location }
}

$manifest = Get-Content -LiteralPath $manifestInTree -Raw | ConvertFrom-Json
$treeVersion = [string]$manifest.version
Write-Host "[prepare-orca-vendor] upstream $Tag BBL.json   : $treeVersion"

# --- 3. Provenance gate -------------------------------------------------------
$a = ConvertTo-VersionParts $installedVersion
$b = ConvertTo-VersionParts $treeVersion
$same = ($a.Count -eq $b.Count) -and -not (Compare-Object $a $b)
if (-not $same) {
    throw ("version-mismatch: installed BBL.opc is {0} but tag {1}'s BBL.json is {2}. " +
           "Pick the upstream tag whose resources/profiles/BBL.json declares {0} (pass -Tag).") -f `
        $installedVersion, $Tag, $treeVersion
}

# --- 4. Stage the vendor root -------------------------------------------------
$profiles = Join-Path $SourceDir 'resources\profiles'
if (Test-Path -LiteralPath $VendorRoot) { Remove-Item -LiteralPath $VendorRoot -Recurse -Force }
New-Item -ItemType Directory -Path $VendorRoot -Force | Out-Null
foreach ($item in @('BBL.json', 'BBL', 'OrcaFilamentLibrary.json', 'OrcaFilamentLibrary')) {
    Copy-Item -LiteralPath (Join-Path $profiles $item) -Destination $VendorRoot -Recurse -Force
}

# The shipped cache goes beside the manifest on purpose. With the JSON tree alone,
# every Orca run parses all ~2.4k vendor presets before slicing: measured on the
# benchy classic-off cell, 10.08 s wall against 2.06 s with the cache present, with
# process CPU essentially flat (8.28 s vs 8.31 s) — i.e. ~8 s of added *serial*
# work that no archived Orca number ever paid. The cache is gated on the version
# asserted above, so it can only serve the tree this script just staged.
Copy-Item -LiteralPath $cachePath -Destination (Join-Path $VendorRoot 'BBL.opc') -Force

# --- 5. Assert the presets the rig names are present --------------------------
$required = @(
    'BBL\machine\Bambu Lab X1 Carbon 0.4 nozzle.json',
    'BBL\process\0.20mm Standard @BBL X1C.json',
    'BBL\filament\Generic PLA.json',
    'OrcaFilamentLibrary.json'
)
foreach ($rel in $required) {
    $p = Join-Path $VendorRoot $rel
    if (-not (Test-Path -LiteralPath $p)) { throw "missing-artifact after staging: $p" }
}

$machineCount = (Get-ChildItem -LiteralPath (Join-Path $VendorRoot 'BBL\machine') -Filter *.json).Count
$processCount = (Get-ChildItem -LiteralPath (Join-Path $VendorRoot 'BBL\process') -Filter *.json).Count
$filamentCount = (Get-ChildItem -LiteralPath (Join-Path $VendorRoot 'BBL\filament') -Filter *.json).Count
Write-Host ("[prepare-orca-vendor] staged {0} (machine {1} / process {2} / filament {3} json)" -f `
        $VendorRoot, $machineCount, $processCount, $filamentCount)

if (-not $KeepSource) {
    Write-Host "[prepare-orca-vendor] source clone kept at $SourceDir (re-run reuses it; delete to force a refetch)"
}

Write-Host "[prepare-orca-vendor] OK: vendor tree version $treeVersion matches the installed cache"
exit 0
