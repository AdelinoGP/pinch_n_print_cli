# t58 end-of-take production restoration.
# Restores the standing accelerated snapshot from its pre-campaign byte archive
# and verifies both mode-specific guest freshness gates. Run AFTER the measure
# campaign and reducer have produced their records.
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Archive = Join-Path $Repo 'target\t58\standing-dist-accelerated-archive'
$Standing = Join-Path $Repo 'target\dist-accelerated\developer'
if (-not (Test-Path $Archive)) { throw "standing archive missing: $Archive" }
$standingExe = Get-FileHash (Join-Path $Standing 'pnp_cli.exe') -ErrorAction SilentlyContinue
$archiveExe = Get-FileHash (Join-Path $Archive 'pnp_cli.exe')
if ($standingExe -and $standingExe.Hash -eq $archiveExe.Hash) {
    Write-Host '[t58-restore] standing snapshot already matches the archive'
} else {
    Remove-Item -Recurse -Force $Standing
    Copy-Item -Recurse -Force $Archive $Standing
    Write-Host '[t58-restore] restored standing snapshot from archive'
    $now = Get-FileHash (Join-Path $Standing 'pnp_cli.exe')
    if ($now.Hash -ne $archiveExe.Hash) { throw 'restore mismatch after copy' }
}
Write-Host '[t58-restore] freshness gates:'
cmd /c "cargo xtask build-guests --check >nul 2>&1"
$ord = $LASTEXITCODE
cmd /c "cargo xtask build-guests --accelerated --check >nul 2>&1"
$acc = $LASTEXITCODE
Write-Host ("[t58-restore] ordinary check exit={0}; accelerated check exit={1}" -f $ord, $acc)
if ($ord -ne 0 -or $acc -ne 0) { throw 'freshness gate failed after restore' }
exit 0