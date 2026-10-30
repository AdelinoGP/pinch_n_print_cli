# t95 Phase-1 screening — build the probe snapshot (attribution only).
#
# The probe is host-only (`slicer-wasm-host` + `pnp-cli`, both compiled into the
# host binary), so the snapshot is the standing `target/dist/developer` module
# tree plus a probe-built `pnp_cli.exe`. No guest is rebuilt and no guest
# artifact is touched: `cargo build -p pnp-cli --release` does not build guests.
#
# The probe sources must already be applied to the tree (`probe.patch`).
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t95-exact-z-batch'
$To = Join-Path $Root 'probe\snapshots\probe'
& cargo build -p pnp-cli --release
if ($LASTEXITCODE -ne 0) { throw 'probe host build failed' }
$Standing = Join-Path $Repo 'target\dist\developer'
if (-not (Test-Path (Join-Path $Standing 'modules'))) { throw 'standing developer modules absent' }
New-Item -ItemType Directory -Force -Path $To | Out-Null
Copy-Item -Force (Join-Path $Repo 'target\release\pnp_cli.exe') (Join-Path $To 'pnp_cli.exe')
if (Test-Path (Join-Path $To 'modules')) { Remove-Item -Recurse -Force (Join-Path $To 'modules') }
Copy-Item -Recurse -Force (Join-Path $Standing 'modules') (Join-Path $To 'modules')
$exe = (Get-FileHash (Join-Path $To 'pnp_cli.exe') -Algorithm SHA256).Hash.ToLower()
$moduleCount = (Get-ChildItem (Join-Path $To 'modules') -Filter *.wasm -Recurse).Count
Write-Host "[t95] probe snapshot exe=$exe modules=$moduleCount"
