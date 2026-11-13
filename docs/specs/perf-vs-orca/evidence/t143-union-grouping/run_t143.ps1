<#
.SYNOPSIS
    Ticket 143 campaign driver: matched-cell output check and paired A/B for the
    planner union-phase request grouping (EMIT_BATCH_LAYERS buffering depth).

.DESCRIPTION
    Two modes over the {benchy, base} x {classic, arachne} x {supports off, on}
    matrix:

      -Mode output  One run per (cell, arm). Arms: baseline / candidate x
                    ordinary (/ accelerated when staged). Every cell must
                    reproduce the active post-t139 reference set byte for byte.

      -Mode ab      Interleaved paired measurement of baseline-ordinary vs
                    candidate-ordinary. Wall = process creation-to-exit, CPU =
                    kernel+user via the process handle. Pair order alternates by
                    pair parity, following evidence/t123-facet-retirement/run_t123.ps1.

    The candidate lives in a guest, so the two arms carry their own module
    trees:

      baseline-ordinary      target/t143-baseline/pnp_cli-ordinary.exe +
                             target/t143-baseline/modules
      baseline-accelerated   target/t143-baseline/accelerated/pnp_cli.exe +
                             target/t143-baseline/accelerated/modules
      candidate-ordinary     target/release/pnp_cli.exe +
                             target/t143-candidate/modules
      candidate-accelerated  target/t143-candidate/accelerated/pnp_cli.exe +
                             target/t143-candidate/accelerated/modules

    The host binary is identical between the ordinary arms (no host source
    changed); only the planner guest differs. `-Mode manifest` records the
    selected arms' exe and planner-module hashes.

    Raw G-code and stderr land under target/t143/<batch>/ (gitignored). The CSV
    is crash-safe (one appended row per run).

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode manifest
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode output -BatchId output
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode ab -Pairs 3 -BatchId ab
#>
[CmdletBinding()]
param(
    [ValidateSet('output', 'ab', 'manifest')]
    [string]$Mode = 'output',
    [string[]]$Cells = @('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on'),
    [string[]]$Arms = @('baseline-ordinary', 'candidate-ordinary'),
    [int]$Pairs = 3,
    [int]$PairFrom = 1,
    [int]$PairTo = 0,
    [switch]$Append,
    [switch]$OnlyMissing,
    [switch]$NullControl,
    [ValidateSet('baseline-ordinary', 'candidate-ordinary')]
    [string]$NullArm = 'baseline-ordinary',
    [int]$Threads = 12,
    [string]$BatchId = 't143',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$AllCells = @(
    'benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
    'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on'
)
$Cells = @($Cells | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' })
foreach ($cell in $Cells) {
    if ($AllCells -notcontains $cell) {
        throw "unknown cell '$cell'"
    }
}
$AllArms = @('baseline-ordinary', 'baseline-accelerated', 'candidate-ordinary', 'candidate-accelerated')
$Arms = @($Arms | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' })
foreach ($arm in $Arms) {
    if ($AllArms -notcontains $arm) {
        throw "unknown arm '$arm'"
    }
}

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$OutDir = Join-Path $RepoRoot ('target\t143\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'rows.csv'

$ArmSpecs = @{
    'baseline-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t143-baseline\pnp_cli-ordinary.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t143-baseline\modules'
    }
    'baseline-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t143-baseline\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t143-baseline\accelerated\modules'
    }
    'candidate-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t143-candidate\modules'
    }
    'candidate-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t143-candidate\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t143-candidate\accelerated\modules'
    }
}

foreach ($arm in @($Arms)) {
    if (-not (Test-Path -LiteralPath $ArmSpecs[$arm].Exe)) {
        throw "arm '$arm' binary missing: $($ArmSpecs[$arm].Exe)"
    }
    if (-not (Test-Path -LiteralPath $ArmSpecs[$arm].ModuleDir)) {
        throw "arm '$arm' module dir missing: $($ArmSpecs[$arm].ModuleDir)"
    }
}

if ($Mode -eq 'manifest') {
    $manifest = [ordered]@{}
    foreach ($arm in @($Arms)) {
        $spec = $ArmSpecs[$arm]
        $planner = Join-Path $spec.ModuleDir 'tree-support-planner\tree-support-planner.wasm'
        $manifest[$arm] = [pscustomobject]@{
            exe_sha256 = (Get-FileHash -LiteralPath $spec.Exe -Algorithm SHA256).Hash.ToLower()
            planner_wasm_sha256 = (Get-FileHash -LiteralPath $planner -Algorithm SHA256).Hash.ToLower()
            exe = $spec.Exe
            module_dir = $spec.ModuleDir
        }
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $RepoRoot 'target\t143') | Out-Null
    $manifestPath = Join-Path $RepoRoot 'target\t143\manifest.json'
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    foreach ($arm in $manifest.Keys) {
        Write-Host ("[t143] {0} exe={1} planner={2}" -f $arm, $manifest[$arm].exe_sha256.Substring(0, 12), $manifest[$arm].planner_wasm_sha256.Substring(0, 12))
    }
    exit 0
}

$Fixtures = @{ benchy = (Join-Path $RepoRoot 'tmp\3dbenchy.stl'); base = (Join-Path $RepoRoot 'tmp\base.stl') }

function Get-CellParts {
    param([string]$Cell)
    $parts = $Cell.Split('-')
    $fixture = $parts[0]
    $generator = $parts[1]
    $supports = $parts[2]
    $config = Join-Path $ConfigDir ("pnp-$generator-supports-$supports.json")
    if (-not (Test-Path -LiteralPath $Fixtures[$fixture])) {
        throw "fixture missing: $($Fixtures[$fixture])"
    }
    if (-not (Test-Path -LiteralPath $config)) {
        throw "config missing: $config"
    }
    return @{ Fixture = $fixture; Model = $Fixtures[$fixture]; Config = $config }
}

function Measure-Slice {
    param([string]$Cell, [string]$Arm, [string]$OutPath)
    $parts = Get-CellParts -Cell $Cell
    $spec = $ArmSpecs[$Arm]
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $spec.Exe
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    foreach ($a in @('slice', '--model', $parts.Model, '--output', $OutPath, '--config', $parts.Config, '--module-dir', $spec.ModuleDir, '--no-default-module-paths')) {
        $psi.ArgumentList.Add($a)
    }
    $psi.Environment['RAYON_NUM_THREADS'] = "$Threads"
    $stderrPath = [System.IO.Path]::ChangeExtension($OutPath, '.stderr.jsonl')
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $errTask = $p.StandardError.ReadToEndAsync()
    $p.StandardOutput.ReadToEnd() | Out-Null
    $p.WaitForExit()
    $sw.Stop()
    $stderr = $errTask.Result
    Set-Content -LiteralPath $stderrPath -Value $stderr -Encoding utf8
    $cpu = $p.TotalProcessorTime.TotalSeconds
    $sha = if (Test-Path -LiteralPath $OutPath) { (Get-FileHash -LiteralPath $OutPath -Algorithm SHA256).Hash.ToLower() } else { '' }
    $bytes = if (Test-Path -LiteralPath $OutPath) { (Get-Item -LiteralPath $OutPath).Length } else { 0 }
    return [pscustomobject]@{
        wall_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 4)
        cpu_seconds = [math]::Round($cpu, 4)
        gcode_bytes = $bytes
        sha256 = $sha
        exit_code = $p.ExitCode
    }
}

$header = 'mode,pair,order,cell,arm,wall_seconds,cpu_seconds,gcode_bytes,sha256,exit_code,timestamp_utc'
$existing = @{}
if ($Append -and (Test-Path -LiteralPath $CsvPath)) {
    foreach ($row in (Import-Csv -LiteralPath $CsvPath)) {
        $existing["$($row.pair)|$($row.cell)|$($row.arm)"] = $true
    }
    Write-Host "[t143] append mode: $($existing.Count) existing row(s) recognised"
} else {
    Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8
}

Write-Host "[t143] mode=$Mode batch=$BatchId cells=$($Cells -join ',') arms=$($Arms -join ',') threads=$Threads"

if ($Mode -eq 'output') {
    foreach ($cell in $Cells) {
        foreach ($arm in $Arms) {
            $runId = "$cell-$arm"
            Write-Host "[t143] START $runId"
            $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
            $line = @(
                $Mode, 0, $arm, $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
            ) -join ','
            Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
            Write-Host ("[t143] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
        }
    }
} else {
    # Null control: the same arm on both sides. It measures the instrument's own
    # paired noise (machine drift, SMT scheduling, CPU accounting) so a flat CPU
    # record can be read against a measured band instead of an assumed one.
    $abArms = if ($NullControl) {
        @($NullArm, $NullArm)
    } else {
        @('baseline-ordinary', 'candidate-ordinary')
    }
    $pairLast = if ($PairTo -gt 0) { $PairTo } else { $Pairs }
    for ($pair = $PairFrom; $pair -le $pairLast; $pair++) {
        foreach ($cell in $Cells) {
            $order = if ($pair % 2 -eq 1) { $abArms } else { @($abArms[1], $abArms[0]) }
            $position = 0
            foreach ($arm in $order) {
                $position += 1
                $runId = if ($NullControl) { "$cell-$arm-p$pair-r$position" } else { "$cell-$arm-p$pair" }
                if ($OnlyMissing -and $existing.ContainsKey("$pair|$cell|$arm")) {
                    Write-Host "[t143] SKIP  $runId (already recorded)"
                    continue
                }
                Write-Host "[t143] START $runId"
                $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
                $line = @(
                    $Mode, $pair, ($order -join '>'), $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                    $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
                ) -join ','
                Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
                Write-Host ("[t143] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
            }
        }
    }
}

Write-Host "[t143] complete: $CsvPath"
