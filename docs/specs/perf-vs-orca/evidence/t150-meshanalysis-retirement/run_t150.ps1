<#
.SYNOPSIS
    perf-vs-orca ticket 150 campaign driver: retire the unread MeshAnalysis
    bridge product.

.DESCRIPTION
    The candidate deletes `find_bridge_clusters` /
    `compute_bridge_metrics` from `classify_object`; `bridge_regions` stays in
    the IR as an empty vector (schema unchanged). Two modes:

      -Mode output  One run per (cell, arm). 8 cells x {baseline, candidate}.
                    The candidate must reproduce the t148 active reference set
                    byte for byte; the baseline arm is the control that the
                    reference set is reproducible on today's box.

      -Mode ab      Interleaved paired measurement of baseline vs candidate.
                    Wall = process creation-to-exit, CPU = kernel+user via the
                    process handle, matching evidence/t133's Measure-Slice.

    Arms:
      baseline   target/t150/bin/pnp_cli-baseline.exe   (pre-change tree, HEAD)
      candidate  target/t150/bin/pnp_cli-candidate.exe  (this ticket's change)

    Both binaries were built with `cargo build --release -p pnp-cli` on the same
    workspace, so only the ticket's source change differs. Guest artifacts are
    shared and were rebuilt once before the campaign
    (`cargo xtask build-guests`); `--check` exits 0.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 -Mode output
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 -Mode ab -Pairs 3
#>
[CmdletBinding()]
param(
    [ValidateSet('output', 'ab', 'null')]
    [string]$Mode = 'output',
    [ValidateSet('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on')]
    [string[]]$Cells = @('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on'),
    [ValidateSet('baseline', 'candidate')]
    [string[]]$Arms = @('baseline', 'candidate'),
    [int]$Pairs = 3,
    [int]$PairFrom = 1,
    [int]$PairTo = 0,
    [switch]$Append,
    [int]$Threads = 12,
    [switch]$NullControl,
    [string]$BatchId = 't150',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$OutDir = Join-Path $RepoRoot ('target\t150\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'rows.csv'

$Exe = @{
    baseline  = Join-Path $RepoRoot 'target\t150\bin\pnp_cli-baseline.exe'
    candidate = Join-Path $RepoRoot 'target\t150\bin\pnp_cli-candidate.exe'
}
$ModuleDir = Join-Path $RepoRoot 'modules\core-modules'

$CsvHeader = 'cell,arm,mode,pair,sequence,wall_seconds,cpu_seconds,gcode_bytes,sha256,exit_code,degraded,exact_z_rejections,slice_elapsed_ms,timestamp_utc'

function Get-CellParts {
    param([string]$Cell)
    $fx = $Cell.Split('-')[0]
    $gen = $Cell.Split('-')[1]
    $sup = $Cell.Split('-')[2]
    $model = if ($fx -eq 'benchy') { Join-Path $RepoRoot 'tmp\3dbenchy.stl' } else { Join-Path $RepoRoot 'tmp\base.stl' }
    $config = Join-Path $ConfigDir ("pnp-$gen-supports-$sup.json")
    return [pscustomobject]@{ Fixture = $fx; Generator = $gen; Supports = $sup; Model = $model; Config = $config }
}

function Measure-Slice {
    param([string]$Cell, [string]$Arm, [string]$OutPath)
    $parts = Get-CellParts -Cell $Cell
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Exe[$Arm]
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    foreach ($a in @('slice', '--model', $parts.Model, '--output', $OutPath, '--config', $parts.Config, '--module-dir', $ModuleDir)) {
        $psi.ArgumentList.Add($a)
    }
    $psi.Environment['RAYON_NUM_THREADS'] = "$Threads"
    $stderrPath = [System.IO.Path]::ChangeExtension($OutPath, '.stderr.log')
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
    $degraded = if ($stderr -match '"degraded":true') { 'true' } else { 'false' }
    $rejections = ([regex]::Matches($stderr, 'exact-Z occupancy')).Count
    $sliceMs = ''
    $m = [regex]::Matches($stderr, '"elapsed_ms":(\d+)')
    if ($m.Count -gt 0) { $sliceMs = $m[$m.Count - 1].Groups[1].Value }
    return [pscustomobject]@{
        wall = [math]::Round($sw.Elapsed.TotalSeconds, 4)
        cpu = [math]::Round($cpu, 4)
        bytes = $bytes
        sha = $sha
        exit_code = $p.ExitCode
        degraded = $degraded
        rejections = $rejections
        slice_ms = $sliceMs
    }
}

function Add-Row {
    param([string]$Cell, [string]$Arm, [string]$Mode, [string]$Pair, [string]$Sequence, $Measured)
    $row = '{0},{1},{2},{3},{4},{5},{6},{7},{8},{9},{10},{11},{12},{13}' -f `
        $Cell, $Arm, $Mode, $Pair, $Sequence,
        $Measured.wall, $Measured.cpu, $Measured.bytes, $Measured.sha, $Measured.exit_code,
        $Measured.degraded, $Measured.rejections, $Measured.slice_ms,
        (Get-Date).ToUniversalTime().ToString('o')
    Add-Content -LiteralPath $CsvPath -Value $row -Encoding utf8
    Write-Host ("[{0}] {1} pair={2} seq={3} {4} wall={5:N2}s cpu={6:N2}s sha={7}" -f `
        $Mode, $Cell, $Pair, $Sequence, $Arm, $Measured.wall, $Measured.cpu, $Measured.sha.Substring(0, [Math]::Min(12, $Measured.sha.Length)))
}

if (-not $Append) {
    Set-Content -LiteralPath $CsvPath -Value $CsvHeader -Encoding utf8
}

$LastPair = if ($PairTo -eq 0) { $Pairs } else { $PairTo }

if ($Mode -eq 'output') {
    foreach ($cell in $Cells) {
        foreach ($arm in $Arms) {
            $out = Join-Path $GcodeDir ("$cell-$arm.gcode")
            $m = Measure-Slice -Cell $cell -Arm $arm -OutPath $out
            Add-Row -Cell $cell -Arm $arm -Mode 'output' -Pair '0' -Sequence '1' -Measured $m
        }
    }
} elseif ($Mode -eq 'ab') {
    foreach ($cell in $Cells) {
        # One discarded warmup pages in the fixture and the module tree.
        Measure-Slice -Cell $cell -Arm 'baseline' -OutPath (Join-Path $GcodeDir "warmup-$cell.gcode") | Out-Null
        for ($pair = $PairFrom; $pair -le $LastPair; $pair++) {
            if ($pair % 2 -eq 1) {
                $first = 'baseline'; $second = 'candidate'
            } else {
                $first = 'candidate'; $second = 'baseline'
            }
            $m1 = Measure-Slice -Cell $cell -Arm $first -OutPath (Join-Path $GcodeDir "$cell-$first-$pair-1.gcode")
            Add-Row -Cell $cell -Arm $first -Mode 'ab' -Pair "$pair" -Sequence '1' -Measured $m1
            $m2 = Measure-Slice -Cell $cell -Arm $second -OutPath (Join-Path $GcodeDir "$cell-$second-$pair-2.gcode")
            Add-Row -Cell $cell -Arm $second -Mode 'ab' -Pair "$pair" -Sequence '2' -Measured $m2
        }
    }
}

if ($Mode -eq 'null' -or ($Mode -eq 'ab' -and $NullControl)) {
    $cell = 'base-classic-off'
    for ($pair = $PairFrom; $pair -le $LastPair; $pair++) {
        $m1 = Measure-Slice -Cell $cell -Arm 'candidate' -OutPath (Join-Path $GcodeDir "null-$cell-candidate-$pair-1.gcode")
        Add-Row -Cell $cell -Arm 'candidate' -Mode 'null' -Pair "$pair" -Sequence '1' -Measured $m1
        $m2 = Measure-Slice -Cell $cell -Arm 'candidate' -OutPath (Join-Path $GcodeDir "null-$cell-candidate-$pair-2.gcode")
        Add-Row -Cell $cell -Arm 'candidate' -Mode 'null' -Pair "$pair" -Sequence '2' -Measured $m2
    }
}
Write-Host "DONE $CsvPath"
