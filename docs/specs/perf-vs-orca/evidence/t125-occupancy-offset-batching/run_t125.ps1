<#
.SYNOPSIS
    Ticket 125 output identity and paired A/B for planner occupancy batching.

.DESCRIPTION
    Output mode runs baseline/candidate x ordinary/accelerated over all eight
    matched cells. AB mode interleaves baseline-ordinary and candidate-ordinary,
    alternating pair order. Every run validates the matched generator and the
    slice_complete diagnostic contract and records wall, process CPU and SHA-256.

    The four arm snapshots contain their own module directories, so the baseline
    planner WASM cannot be overwritten by a later candidate guest build.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/run_t125.ps1 -Mode output -BatchId output
.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/run_t125.ps1 -Mode ab -Pairs 3 -BatchId ab
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
    [string]$BatchId = 't125',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'
$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Validator = Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1'
. $Validator

$OutDir = Join-Path $RepoRoot ('target\t125\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'rows.csv'

$ArmSpecs = @{
    'baseline-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t125-baseline\ordinary\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t125-baseline\ordinary\modules'
    }
    'baseline-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t125-baseline\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t125-baseline\accelerated\modules'
    }
    'candidate-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t125-candidate\ordinary\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t125-candidate\ordinary\modules'
    }
    'candidate-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t125-candidate\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t125-candidate\accelerated\modules'
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
    if (-not (Test-Path -LiteralPath $Fixtures[$fixture])) { throw "fixture missing: $($Fixtures[$fixture])" }
    if (-not (Test-Path -LiteralPath $config)) { throw "config missing: $config" }
    return @{ Fixture = $fixture; Model = $Fixtures[$fixture]; Config = $config; Generator = $generator }
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
    $evidence = Test-MeasurementEvidence -ConfigPath $parts.Config -OutputPath $OutPath -StderrPath $stderrPath -ExpectedGenerator $parts.Generator -ExitCode $p.ExitCode
    $sha = (Get-FileHash -LiteralPath $OutPath -Algorithm SHA256).Hash.ToLower()
    return [pscustomobject]@{
        wall_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 4)
        cpu_seconds = [math]::Round($p.TotalProcessorTime.TotalSeconds, 4)
        gcode_bytes = (Get-Item -LiteralPath $OutPath).Length
        sha256 = $sha
        exit_code = $p.ExitCode
        completion_status = $evidence.CompletionStatus
        non_fatal_error_count = $evidence.NonFatalErrorCount
        degraded = if ($evidence.Degraded) { 'true' } else { 'false' }
    }
}

foreach ($arm in @($Arms)) {
    if (-not (Test-Path -LiteralPath $ArmSpecs[$arm].Exe)) { throw "arm '$arm' binary missing: $($ArmSpecs[$arm].Exe)" }
    $diag = & $ArmSpecs[$arm].Exe module diagnose --module-dir $ArmSpecs[$arm].ModuleDir | Out-String
    $parsed = $diag | ConvertFrom-Json
    if (-not $parsed.pass -or [int]$parsed.modules_loaded -lt 24) {
        throw "module diagnose failed for ${arm}: pass=$($parsed.pass) modules=$($parsed.modules_loaded)"
    }
}
Push-Location $RepoRoot
try {
    cargo xtask build-guests --check | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "ordinary guest freshness exit $LASTEXITCODE" }
    cargo xtask build-guests --accelerated --check | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "accelerated guest freshness exit $LASTEXITCODE" }
} finally { Pop-Location }

$header = 'mode,pair,order,cell,arm,wall_seconds,cpu_seconds,gcode_bytes,sha256,exit_code,completion_status,non_fatal_error_count,degraded,timestamp_utc'
$existing = @{}
if ($Append -and (Test-Path -LiteralPath $CsvPath)) {
    foreach ($row in (Import-Csv -LiteralPath $CsvPath)) { $existing["$($row.pair)|$($row.cell)|$($row.arm)"] = $true }
    Write-Host "[t125] append mode: $($existing.Count) existing row(s) recognised"
} else {
    Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8
}

Write-Host "[t125] mode=$Mode batch=$BatchId cells=$($Cells -join ',') arms=$($Arms -join ',') threads=$Threads"
if ($Mode -eq 'output') {
    foreach ($cell in $Cells) {
        foreach ($arm in $Arms) {
            $runId = "$cell-$arm"
            if ($OnlyMissing -and $existing.ContainsKey("0|$cell|$arm")) { Write-Host "[t125] SKIP $runId"; continue }
            Write-Host "[t125] START $runId"
            $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
            $line = @($Mode, 0, $arm, $cell, $arm, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes,
                $row.sha256, $row.exit_code, $row.completion_status, $row.non_fatal_error_count, $row.degraded,
                (Get-Date).ToUniversalTime().ToString('o')) -join ','
            Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
            Write-Host ("[t125] DONE {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5} degraded={6} nonfatal={7}" -f
                $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, 12),
                $row.exit_code, $row.degraded, $row.non_fatal_error_count)
        }
    }
} else {
    $abArms = @('baseline-ordinary', 'candidate-ordinary')
    $pairLast = if ($PairTo -gt 0) { $PairTo } else { $Pairs }
    for ($pair = $PairFrom; $pair -le $pairLast; $pair++) {
        foreach ($cell in $Cells) {
            $order = if ($pair % 2 -eq 1) { $abArms } else { @($abArms[1], $abArms[0]) }
            foreach ($arm in $order) {
                $runId = "$cell-$arm-p$pair"
                if ($OnlyMissing -and $existing.ContainsKey("$pair|$cell|$arm")) { Write-Host "[t125] SKIP $runId"; continue }
                Write-Host "[t125] START $runId"
                $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
                $line = @($Mode, $pair, ($order -join '>'), $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                    $row.gcode_bytes, $row.sha256, $row.exit_code, $row.completion_status,
                    $row.non_fatal_error_count, $row.degraded, (Get-Date).ToUniversalTime().ToString('o')) -join ','
                Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
                Write-Host ("[t125] DONE {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5} degraded={6} nonfatal={7}" -f
                    $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, 12),
                    $row.exit_code, $row.degraded, $row.non_fatal_error_count)
            }
        }
    }
}
Write-Host "[t125] complete: $CsvPath"
