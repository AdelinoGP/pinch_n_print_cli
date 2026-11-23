# t79 screening driver. For each variant in the plan: set the two constants in
# the planner guest source, rebuild the stale guest, rebuild the (unchanged)
# release CLI, then run one or more instrumented slices on the screening cell.
#
# Screening only: these runs choose constants; the standing paired gate runs on
# frozen snapshots afterwards. A run is recorded in `sweep/runs.jsonl` as soon as
# it finishes, so an interrupted sweep keeps its completed rows.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Plan
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Lib = Join-Path $Repo 'modules\core-modules\tree-support-planner\src\lib.rs'
$OutDir = Join-Path $PSScriptRoot 'sweep'
$RawDir = Join-Path $Repo 'target\t79-sweep'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path $RawDir | Out-Null
$Rows = Join-Path $OutDir 'runs.jsonl'
$planData = Get-Content $Plan -Raw | ConvertFrom-Json

function Set-Constants([int]$E, [int]$C) {
    $src = [IO.File]::ReadAllText($Lib)
    $src = $src -replace 'const EMIT_BATCH_LAYERS: usize = \d+;', "const EMIT_BATCH_LAYERS: usize = $E;"
    $src = $src -replace 'const CLIP_BATCH_MAX_REQUESTS: usize = \d+;', "const CLIP_BATCH_MAX_REQUESTS: usize = $C;"
    [IO.File]::WriteAllText($Lib, $src)
}

function Build-Guest {
    & cargo xtask build-guests 2>&1 | Tee-Object -FilePath (Join-Path $RawDir 'build-guests.log') -Append
    if ($LASTEXITCODE -ne 0) { throw "build-guests failed for e=$E c=$C" }
    & cargo build --release -p pnp-cli 2>&1 | Tee-Object -FilePath (Join-Path $RawDir 'build-cli.log') -Append
    if ($LASTEXITCODE -ne 0) { throw "release build failed for e=$E c=$C" }
}

foreach ($variant in $planData.variants) {
    $E = [int]$variant.e
    $C = [int]$variant.c
    $repeats = [int]$variant.repeats
    $idx = [int]$variant.index
    $tag = "v{0:00}-e{1}-c{2}" -f $idx, $E, $C
    Write-Host "[t79-sweep] variant $tag repeats=$repeats $(Get-Date -Format o)"
    Set-Constants -E $E -C $C
    Build-Guest
    for ($r = 1; $r -le $repeats; $r++) {
        $stem = Join-Path $RawDir "$tag-r$r"
        $out = "$stem.gcode"
        $err = "$stem.stderr.jsonl"
        $meta = & (Join-Path $PSScriptRoot 'run-instrumented.ps1') `
            -Model (Join-Path $Repo $planData.model) `
            -Config (Join-Path $Repo $planData.config) `
            -Out $out -Err $err
        $row = $meta | ConvertFrom-Json
        $row | Add-Member -NotePropertyName variant -NotePropertyValue $tag
        $row | Add-Member -NotePropertyName e -NotePropertyValue $E
        $row | Add-Member -NotePropertyName c -NotePropertyValue $C
        $row | Add-Member -NotePropertyName repeat -NotePropertyValue $r
        $row | Add-Member -NotePropertyName stderr -NotePropertyValue "target/t79-sweep/$tag-r$r.stderr.jsonl"
        $row | Add-Member -NotePropertyName timestamp -NotePropertyValue (Get-Date -Format o)
        ($row | ConvertTo-Json -Compress) | Add-Content -Path $Rows -Encoding utf8
        Write-Host ("[t79-sweep] {0} r{1} wall={2}s peak={3}MB sha={4}" -f $tag, $r, $row.wall_seconds, $row.peak_working_set_mb, $row.sha256)
        if ($row.exit_code -ne 0 -or $row.status -ne 'ok' -or $row.degraded) {
            throw "unclean screening run: $tag r$r"
        }
        if ($planData.expected_sha -and $row.sha256 -ne $planData.expected_sha) {
            throw "output drift on screening run: $tag r$r sha=$($row.sha256)"
        }
    }
}
Write-Host "[t79-sweep] done $(Get-Date -Format o)"
