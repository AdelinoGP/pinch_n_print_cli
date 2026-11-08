<#
.SYNOPSIS
    Ticket 116 campaign driver: matched-cell output check and paired A/B for the
    canonical flat-bridge classification candidate.

.DESCRIPTION
    Two modes over the {benchy, base} x {classic, arachne} x {supports off, on}
    matrix:

      -Mode output  One run per (cell, arm). Four arms: baseline / candidate x
                    ordinary / accelerated. Produces the 16-cell reference set
                    (8 cells x 2 arms per build) and the hashes compared against
                    evidence/t115-post-merge/summary.json output_audit.

      -Mode ab      Interleaved paired measurement of baseline-ordinary vs
                    candidate-ordinary. Wall = process creation-to-exit, CPU =
                    kernel+user via the process handle. The measurement math and
                    the per-pair order alternation follow
                    evidence/t115-post-merge/run_merge_ab.ps1 (which follows
                    evidence/matched-pair/run_scoreboard.ps1), so the rows are
                    directly comparable with the t115 A/B.

    Arms:
      baseline-ordinary     target/t116-baseline/pnp_cli-ordinary.exe +
                            in-tree modules/core-modules (HEAD 456bd349 build)
      baseline-accelerated  target/t116-baseline/accelerated/pnp_cli.exe +
                            target/t116-baseline/accelerated/modules
      candidate-ordinary    target/t116-candidate/pnp_cli-ordinary.exe +
                            in-tree modules/core-modules
      candidate-accelerated target/t116-candidate/accelerated/pnp_cli.exe +
                            target/t116-candidate/accelerated/modules

    Raw G-code and stderr land under target/t116/<batch>/ (gitignored). The CSV
    is crash-safe (one appended row per run).

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/run_t116.ps1 -Mode output -BatchId output
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/run_t116.ps1 -Mode ab -Pairs 3 -BatchId ab
#>
[CmdletBinding()]
param(
    [ValidateSet('output', 'ab')]
    [string]$Mode = 'output',
    [ValidateSet('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on')]
    [string[]]$Cells = @('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on'),
    [ValidateSet('baseline-ordinary', 'baseline-accelerated', 'candidate-ordinary', 'candidate-accelerated')]
    [string[]]$Arms = @('baseline-ordinary', 'baseline-accelerated', 'candidate-ordinary', 'candidate-accelerated'),
    [int]$Pairs = 3,
    [int]$PairFrom = 1,
    [int]$PairTo = 0,
    [switch]$Append,
    [switch]$OnlyMissing,
    [int]$Threads = 12,
    [string]$BatchId = 't116',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$OutDir = Join-Path $RepoRoot ('target\t116\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'rows.csv'

$ArmSpecs = @{
    'baseline-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t116-baseline\pnp_cli-ordinary.exe'
        ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
    }
    'baseline-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t116-baseline\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t116-baseline\accelerated\modules'
    }
    'candidate-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t116-candidate\pnp_cli-ordinary.exe'
        ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
    }
    'candidate-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t116-candidate\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t116-candidate\accelerated\modules'
    }
}

foreach ($arm in @($Arms)) {
    if (-not (Test-Path -LiteralPath $ArmSpecs[$arm].Exe)) {
        throw "arm '$arm' binary missing: $($ArmSpecs[$arm].Exe)"
    }
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
    Write-Host "[t116] append mode: $($existing.Count) existing row(s) recognised"
} else {
    Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8
}

Write-Host "[t116] mode=$Mode batch=$BatchId cells=$($Cells -join ',') arms=$($Arms -join ',') threads=$Threads"

if ($Mode -eq 'output') {
    foreach ($cell in $Cells) {
        foreach ($arm in $Arms) {
            $runId = "$cell-$arm"
            Write-Host "[t116] START $runId"
            $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
            $line = @(
                $Mode, 0, $arm, $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
            ) -join ','
            Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
            Write-Host ("[t116] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
        }
    }
} else {
    $abArms = @('baseline-ordinary', 'candidate-ordinary')
    foreach ($arm in $abArms) {
        if (-not (Test-Path -LiteralPath $ArmSpecs[$arm].Exe)) {
            throw "ab arm '$arm' binary missing: $($ArmSpecs[$arm].Exe)"
        }
    }
    $pairLast = if ($PairTo -gt 0) { $PairTo } else { $Pairs }
    for ($pair = $PairFrom; $pair -le $pairLast; $pair++) {
        foreach ($cell in $Cells) {
            $order = if ($pair % 2 -eq 1) { $abArms } else { @($abArms[1], $abArms[0]) }
            foreach ($arm in $order) {
                $runId = "$cell-$arm-p$pair"
                if ($OnlyMissing -and $existing.ContainsKey("$pair|$cell|$arm")) {
                    Write-Host "[t116] SKIP  $runId (already recorded)"
                    continue
                }
                Write-Host "[t116] START $runId"
                $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
                $line = @(
                    $Mode, $pair, ($order -join '>'), $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                    $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
                ) -join ','
                Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
                Write-Host ("[t116] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
            }
        }
    }
}

Write-Host "[t116] complete: $CsvPath"
