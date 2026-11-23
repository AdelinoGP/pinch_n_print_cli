# Resolution-port paired campaign harness (perf-vs-orca ticket 81)
#
# Compares the pre-port PnP (baseline arm) with the `resolution` port
# (candidate arm) on the identical matched job. Both arms are frozen snapshots:
# the baseline was built from HEAD before the port, the candidate from the
# working tree after it, each with its own module directory and binary.
#
# Pre-declared acceptance (ticket 81, 2026-10-23):
#   * paired-median wall delta < 0 in every one of the eight gate cells
#   * paired-median CPU delta <= +3.0 % in every cell
#   * stop by rule at the first cell whose wall clause fails; preserve the
#     partial set and do not run the remaining cells
#
# Measurement math is the ticket-11 rig's (matched-pair/run_scoreboard.ps1):
# wall = process creation-to-exit, CPU = GetProcessTimes kernel+user.
#
# Usage:
#   pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/t81-resolution-port/run_port_pair.ps1' -Fixture benchy -Cells 'classic-off,classic-on,arachne-off,arachne-on' -Pairs 6 -Warmups 1 -BatchId benchy-1"

param(
    [Parameter(Mandatory = $true)][ValidateSet('benchy', 'base')][string]$Fixture,
    [string[]]$Cells = @('classic-off', 'classic-on', 'arachne-off', 'arachne-on'),
    [int]$Pairs = 6,
    [int]$Warmups = 1,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$Job = @{
    WallLoops           = '2'
    SparseInfillDensity = '20'
    SparseInfillPattern = 'gyroid'
    LayerHeight         = '0.2'
}

$FixturePaths = @{
    benchy = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
    base   = Join-Path $RepoRoot 'tmp\base.stl'
}

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'

# Arm -> frozen snapshot. `resolution` is the value the G-code appendix must
# disclose: the baseline still aliases it to `gcode_resolution` (0.0125, the
# disclosure lie ticket 80 measured), the candidate renders its own real key
# (0.01, the canonical default).
$Arms = [ordered]@{
    'baseline'  = @{
        Exe        = Join-Path $RepoRoot 'target\t81\baseline\pnp_cli.exe'
        Modules    = Join-Path $RepoRoot 'target\t81\baseline\modules'
        Resolution = '0.0125'
    }
    'candidate' = @{
        Exe        = Join-Path $RepoRoot 'target\t81\candidate\pnp_cli.exe'
        Modules    = Join-Path $RepoRoot 'target\t81\candidate\modules'
        Resolution = '0.01'
    }
}

$RunDir   = Join-Path $RepoRoot ('target/t81-campaign/' + $BatchId)
$GcodeDir = Join-Path $RunDir 'gcode'
$CsvPath  = Join-Path $RunDir 'rows.csv'
foreach ($d in @($RunDir, $GcodeDir)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }

foreach ($p in @($FixturePaths[$Fixture], $Arms['baseline'].Exe, $Arms['candidate'].Exe)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing prerequisite: $p" }
}

# --- Measurement core (copied from matched-pair/run_scoreboard.ps1) ----------
if (-not ('ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProcessTimes {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h, out System.Runtime.InteropServices.ComTypes.FILETIME c, out System.Runtime.InteropServices.ComTypes.FILETIME e, out System.Runtime.InteropServices.ComTypes.FILETIME k, out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime << 32) + (uint)f.dwLowDateTime;
}
'@
}

function Get-ProcessTimesSeconds {
    param([IntPtr]$Handle)
    $creation = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $exit = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $kernel = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $user = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [ProcessTimes]::GetProcessTimes($Handle, [ref]$creation, [ref]$exit, [ref]$kernel, [ref]$user)) {
        throw "GetProcessTimes failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    [pscustomobject]@{
        WallSeconds = ([ProcessTimes]::T($exit) - [ProcessTimes]::T($creation)) / 10000000.0
        CpuSeconds  = ([ProcessTimes]::T($kernel) + [ProcessTimes]::T($user)) / 10000000.0
    }
}

function Invoke-MeasuredProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [int]$PeakSampleMs = 200
    )
    Remove-Item -Force -ErrorAction SilentlyContinue $StderrPath
    $proc = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -PassThru `
        -NoNewWindow -RedirectStandardOutput $StdoutPath -RedirectStandardError $StderrPath
    $processHandle = $proc.Handle
    $maxWs = 0
    while ($true) {
        $p = $null
        try { $p = Get-Process -Id $proc.Id -ErrorAction Stop } catch { $p = $null }
        if ($null -eq $p) { break }
        if ($p.PeakWorkingSet64 -gt $maxWs) { $maxWs = $p.PeakWorkingSet64 }
        Start-Sleep -Milliseconds $PeakSampleMs
    }
    $proc.WaitForExit()
    $proc.Refresh()
    $native = Get-ProcessTimesSeconds -Handle $processHandle
    [pscustomobject]@{
        WallSeconds         = $native.WallSeconds
        CpuSeconds          = $native.CpuSeconds
        PeakWorkingSetBytes = $maxWs
        ExitCode            = $proc.ExitCode
    }
}

function Get-AppendixMap {
    param([Parameter(Mandatory = $true)][string]$Path)
    $map = @{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^\s*;\s*([a-z0-9_]+)\s*=\s*(.*?)\s*$') { $map[$Matches[1]] = $Matches[2] }
    }
    return $map
}

function Get-GcodeSectionCounts {
    param([Parameter(Mandatory = $true)][string]$Path)
    $counts = [ordered]@{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^;\s*TYPE:\s*(.+?)\s*$') {
            $name = $Matches[1].Trim()
            if ($counts.Contains($name)) { $counts[$name]++ } else { $counts[$name] = 1 }
        }
    }
    return $counts
}

function Assert-EqualValue {
    param([string]$What, [string]$Actual, [string]$Expected, [string]$RunId)
    if ($Actual -ne $Expected) {
        throw "measurement rejected ($RunId): $What is '$Actual', expected '$Expected'"
    }
}

$rows = [System.Collections.Generic.List[string]]::new()
$header = 'run_id,batch_id,fixture,generator,supports,arm,run_index,warmup,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,type_counts_json,resolution_disclosed,config_path,output_path,exit_code,timestamp_utc'
$rows.Add($header)
if (-not (Test-Path -LiteralPath $CsvPath)) {
    Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8
}

function Invoke-Run {
    param([string]$Generator, [string]$Supports, [string]$Arm, [int]$RunIndex, [bool]$Warmup)
    $armSpec = $Arms[$Arm]
    $cell = $Generator + '-' + $Supports
    $tag = if ($Warmup) { 'w' } else { 'm' }
    $runId = "$Fixture-$cell-$Arm-$tag$RunIndex"
    $config = Join-Path $ConfigDir ("pnp-" + $Generator + "-supports-" + $Supports + ".json")
    $outPath = Join-Path $GcodeDir ($runId + '.gcode')
    $stdout = Join-Path $RunDir ($runId + '.stdout.log')
    $stderr = Join-Path $RunDir ($runId + '.stderr.log')

    Write-Host ("[t81] " + (Get-Date).ToString('HH:mm:ss') + " START " + $runId)
    $measured = Invoke-MeasuredProcess -FilePath $armSpec.Exe -ArgumentList @(
        'slice', '--model', ('"' + $FixturePaths[$Fixture] + '"'), '--config', ('"' + $config + '"'),
        '--module-dir', ('"' + $armSpec.Modules + '"'), '--output', ('"' + $outPath + '"')
    ) -StdoutPath $stdout -StderrPath $stderr

    if ($measured.ExitCode -ne 0) { throw "measurement rejected ($runId): exit code $($measured.ExitCode)" }
    if (-not (Test-Path -LiteralPath $outPath)) { throw "measurement rejected ($runId): output missing" }

    $cfg = Get-AppendixMap -Path $outPath
    if ($cfg.Count -lt 50) { throw "measurement rejected ($runId): appendix missing or truncated" }
    Assert-EqualValue 'wall_generator' $cfg['wall_generator'] $Generator $runId
    Assert-EqualValue 'wall_loops' $cfg['wall_loops'] $Job.WallLoops $runId
    Assert-EqualValue 'sparse_infill_density' $cfg['sparse_infill_density'] $Job.SparseInfillDensity $runId
    Assert-EqualValue 'layer_height' $cfg['layer_height'] $Job.LayerHeight $runId
    Assert-EqualValue 'enable_support' $cfg['enable_support'] $(if ($Supports -eq 'on') { '1' } else { '0' }) $runId
    # Numeric comparison: the baseline arm still aliases `resolution` to
    # `gcode_resolution` and renders the f32->f64 widening artifact, so the
    # disclosed string differs in form as well as value. The value is the claim.
    $disclosed = 0.0
    if (-not [double]::TryParse($cfg['resolution'], [ref]$disclosed)) {
        throw "measurement rejected ($runId): resolution '$($cfg['resolution'])' is not numeric"
    }
    $expected = [double]::Parse($armSpec.Resolution, [System.Globalization.CultureInfo]::InvariantCulture)
    if ([math]::Abs($disclosed - $expected) -gt 1e-6) {
        throw "measurement rejected ($runId): resolution is '$($cfg['resolution'])', expected $expected"
    }

    $bytes = (Get-Item -LiteralPath $outPath).Length
    $sha = (Get-FileHash -Path $outPath -Algorithm SHA256).Hash.ToLower()
    $typeCounts = (Get-GcodeSectionCounts -Path $outPath) | ConvertTo-Json -Compress
    $supportFlag = if ($Supports -eq 'on') { 'on' } else { 'off' }
    $ratio = if ($measured.WallSeconds -gt 0) { $measured.CpuSeconds / $measured.WallSeconds } else { 0 }
    $row = @(
        $runId, $BatchId, $Fixture, $Generator, $supportFlag, $Arm, $RunIndex, [int]$Warmup,
        ('{0:F3}' -f $measured.WallSeconds), ('{0:F3}' -f $measured.CpuSeconds), ('{0:F4}' -f $ratio),
        $measured.PeakWorkingSetBytes, $bytes, $sha, $typeCounts, $cfg['resolution'],
        $config, $outPath, $measured.ExitCode, (Get-Date).ToUniversalTime().ToString('o')
    ) | ForEach-Object {
        $v = [string]$_
        if ($v -match '[",\r\n]') { '"' + ($v -replace '"', '""') + '"' } else { $v }
    }
    # Append immediately: an interrupted batch (a host restart, a killed
    # session) must not lose the pairs it already paid for.
    $line = $row -join ','
    $rows.Add($line)
    Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
    Write-Host ("[t81] " + (Get-Date).ToString('HH:mm:ss') + " END   " + $runId +
        (" wall={0:F2}s cpu={1:F2}s bytes={2}" -f $measured.WallSeconds, $measured.CpuSeconds, $bytes))
    [pscustomobject]@{
        Cell = $cell; Arm = $Arm; RunIndex = $RunIndex; Warmup = $Warmup
        Wall = $measured.WallSeconds; Cpu = $measured.CpuSeconds; Bytes = $bytes
    }
}

function Get-Median {
    param([double[]]$Values)
    $sorted = $Values | Sort-Object
    $n = $sorted.Count
    if ($n -eq 0) { return [double]::NaN }
    if ($n % 2 -eq 1) { return $sorted[[int](($n - 1) / 2)] }
    return ($sorted[$n / 2 - 1] + $sorted[$n / 2]) / 2.0
}

$summary = [System.Collections.Generic.List[object]]::new()
$stoppedByRule = $false

foreach ($cell in $Cells) {
    $parts = $cell.Split('-')
    $Generator = $parts[0]
    $Supports = $parts[1]

    for ($w = 0; $w -lt $Warmups; $w++) {
        foreach ($arm in $Arms.Keys) {
            Invoke-Run -Generator $Generator -Supports $Supports -Arm $arm -RunIndex 0 -Warmup $true | Out-Null
        }
    }

    $pairedWall = @()
    $pairedCpu = @()
    $wallDeltas = @()
    $cpuDeltas = @()
    for ($i = 1; $i -le $Pairs; $i++) {
        # Alternate which arm goes first so a monotone drift cancels instead of
        # loading one arm.
        $order = if ($i % 2 -eq 1) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
        $results = @{}
        foreach ($arm in $order) {
            $r = Invoke-Run -Generator $Generator -Supports $Supports -Arm $arm -RunIndex $i -Warmup $false
            $results[$arm] = $r
        }
        $b = $results['baseline']
        $c = $results['candidate']
        $wallDeltas += (($c.Wall - $b.Wall) / $b.Wall * 100.0)
        $cpuDeltas += (($c.Cpu - $b.Cpu) / $b.Cpu * 100.0)
        Write-Host ("[t81] pair " + $i + " " + $cell + " wall%={0:F2} cpu%={1:F2}" -f $wallDeltas[-1], $cpuDeltas[-1])
    }

    $wallMedian = Get-Median -Values $wallDeltas
    $cpuMedian = Get-Median -Values $cpuDeltas
    $entry = [pscustomobject]@{
        cell = $cell
        pairs = $Pairs
        wall_delta_pct_median = [math]::Round($wallMedian, 4)
        cpu_delta_pct_median = [math]::Round($cpuMedian, 4)
        wall_deltas = ($wallDeltas | ForEach-Object { [math]::Round($_, 4) })
        cpu_deltas = ($cpuDeltas | ForEach-Object { [math]::Round($_, 4) })
        wall_clause = ($wallMedian -lt 0)
        cpu_clause = ($cpuMedian -le 3.0)
    }
    $summary.Add($entry)
    Write-Host ("[t81] CELL " + $cell + (" wall_median={0:F3}% cpu_median={1:F3}% wall_clause={2} cpu_clause={3}" -f $wallMedian, $cpuMedian, $entry.wall_clause, $entry.cpu_clause))

    if (-not $entry.wall_clause) {
        $stoppedByRule = $true
        Write-Host "[t81] STOP BY RULE: $cell failed the wall clause; remaining cells not run."
        break
    }
}

$rows | Set-Content -LiteralPath $CsvPath -Encoding utf8
$summary | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $RunDir 'summary.json') -Encoding utf8
$summary | Format-Table -AutoSize | Out-String | Set-Content -LiteralPath (Join-Path $RunDir 'summary.txt') -Encoding utf8
Write-Host "[t81] rows -> $CsvPath"
Write-Host "[t81] stopped_by_rule=$stoppedByRule"
if ($stoppedByRule) { exit 2 }
exit 0
