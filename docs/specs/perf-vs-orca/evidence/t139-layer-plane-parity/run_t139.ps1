<#
.SYNOPSIS
    Ticket 139 campaign driver: canonical mid-layer sampling output check and
    the old-vs-new paired A/B.

.DESCRIPTION
    Two modes over the {benchy, base} x {classic, arachne} x {supports off, on}
    matrix:

      -Mode output  One run per (cell, arm). Three arms: baseline-ordinary,
                    candidate-ordinary, candidate-accelerated. Produces the
                    16-cell reference set (8 cells x 2 build arms) plus the
                    pre-change ordinary arm that must reproduce the t135
                    Stage-1 set exactly. Arms must be byte-identical within
                    each arm class; candidate ordinary must equal candidate
                    accelerated.

      -Mode ab      Interleaved paired measurement of baseline-ordinary vs
                    candidate-ordinary. Wall = process creation-to-exit, CPU =
                    kernel+user via the process handle. Pair order alternates
                    by pair parity, following
                    evidence/t133-windowed-bridge-offsets/run_t133.ps1.

    The candidate changes the host slice producer AND the guest layer planner,
    so the ordinary arms carry their own host binary and module tree. The
    baseline arm is the t135 Stage-1 candidate snapshot (pre-t139 source); the
    candidate arms are this ticket's build.

    Raw G-code and stderr land under target/t139/<batch>/ (gitignored). The CSV
    is crash-safe (one appended row per run).

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 -Mode manifest
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 -Mode output -BatchId output-final
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 -Mode ab -Pairs 3 -BatchId ab
#>
[CmdletBinding()]
param(
    [ValidateSet('output', 'ab', 'manifest')]
    [string]$Mode = 'output',
    [ValidateSet('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on')]
    [string[]]$Cells = @('benchy-classic-off', 'benchy-classic-on', 'benchy-arachne-off', 'benchy-arachne-on',
        'base-classic-off', 'base-classic-on', 'base-arachne-off', 'base-arachne-on'),
    [ValidateSet('baseline-ordinary', 'candidate-ordinary', 'candidate-accelerated')]
    [string[]]$Arms = @('baseline-ordinary', 'candidate-ordinary', 'candidate-accelerated'),
    [int]$Pairs = 3,
    [int]$PairFrom = 1,
    [int]$PairTo = 0,
    [switch]$Append,
    [switch]$OnlyMissing,
    [int]$Threads = 12,
    [string]$BatchId = 't139',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$OutDir = Join-Path $RepoRoot ('target\t139\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'rows.csv'

$ArmSpecs = @{
    'baseline-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t139-baseline\pnp_cli-ordinary.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t139-baseline\modules'
    }
    'candidate-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\t139-candidate\pnp_cli-ordinary.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t139-candidate\modules'
    }
    'candidate-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\t139-candidate\accelerated\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\t139-candidate\accelerated\modules'
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
    $manifest = @{}
    foreach ($arm in @('baseline-ordinary', 'candidate-ordinary', 'candidate-accelerated')) {
        $spec = $ArmSpecs[$arm]
        $planner = Join-Path $spec.ModuleDir 'layer-planner-default\layer-planner-default.wasm'
        $manifest[$arm] = [pscustomobject]@{
            exe_sha256 = (Get-FileHash -LiteralPath $spec.Exe -Algorithm SHA256).Hash.ToLower()
            layer_planner_wasm_sha256 = (Get-FileHash -LiteralPath $planner -Algorithm SHA256).Hash.ToLower()
            exe = $spec.Exe
            module_dir = $spec.ModuleDir
        }
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $RepoRoot 'target\t139') | Out-Null
    $manifestPath = Join-Path $RepoRoot 'target\t139\manifest.json'
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    foreach ($arm in $manifest.Keys) {
        Write-Host ("[t139] {0} exe={1} layer_planner={2}" -f $arm, $manifest[$arm].exe_sha256.Substring(0, 12), $manifest[$arm].layer_planner_wasm_sha256.Substring(0, 12))
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
    Write-Host "[t139] append mode: $($existing.Count) existing row(s) recognised"
} else {
    Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8
}

Write-Host "[t139] mode=$Mode batch=$BatchId cells=$($Cells -join ',') arms=$($Arms -join ',') threads=$Threads"

if ($Mode -eq 'output') {
    foreach ($cell in $Cells) {
        foreach ($arm in $Arms) {
            $runId = "$cell-$arm"
            Write-Host "[t139] START $runId"
            $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
            $line = @(
                $Mode, 0, $arm, $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
            ) -join ','
            Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
            Write-Host ("[t139] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
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
                if ($OnlyMissing -and $existing.ContainsKey("$pair|$cell|$arm")) {
                    Write-Host "[t139] SKIP  $runId (already recorded)"
                    continue
                }
                Write-Host "[t139] START $runId"
                $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
                $line = @(
                    $Mode, $pair, ($order -join '>'), $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                    $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
                ) -join ','
                Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
                Write-Host ("[t139] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
            }
        }
    }
}

Write-Host "[t139] complete: $CsvPath"
