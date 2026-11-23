# Resolution-fairness pair harness (perf-vs-orca ticket 80)
#
# Runs the matched Orca job twice per pair: the as-run profile (which inherits
# the vendor process default `resolution = 0.01`) and a finest-possible arm at
# the canonical floor `resolution = 0.001`. Everything else is identical, and
# the effective value is verified from the G-code config appendix in every run.
#
# Canonical note: `PrintConfig.cpp`'s post-load clamp rewrites any `resolution`
# below 0.001 to 0.001, so "off" is not expressible in OrcaSlicer. The floor arm
# is therefore the closest available stand-in for PnP's absence of a
# slicing-time simplification pass — 10x finer than the matched value.
#
# Measurement math is the ticket-11 rig's (matched-pair/run_scoreboard.ps1):
# wall = process creation-to-exit, CPU = GetProcessTimes kernel+user.
#
# Usage:
#   pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/run_resolution_pair.ps1' -Fixture benchy -Cells 'classic-off' -Pairs 1 -Warmups 1 -BatchId smoke"

param(
    [Parameter(Mandatory = $true)][ValidateSet('benchy', 'base')][string]$Fixture,
    [string[]]$Cells = @('classic-off', 'classic-on'),
    [int]$Pairs = 3,
    [int]$Warmups = 1,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$Job = @{
    LayerHeight         = '0.2'
    FirstLayerHeight    = '0.2'
    WallLoops           = '2'
    SparseInfillDensity = '20%'
    SparseInfillPattern = 'gyroid'
    SupportType         = 'tree(auto)'
}

$OrcaExe           = 'C:\Program Files\OrcaSlicer\orca-slicer.exe'
$OrcaProfile       = 'C:\Program Files\OrcaSlicer\resources\profiles\BBL'
$OrcaMachine       = Join-Path $OrcaProfile 'machine\Bambu Lab X1 Carbon 0.4 nozzle.json'
$OrcaVendorProcess = Join-Path $OrcaProfile 'process\0.20mm Standard @BBL X1C.json'
$OrcaFilament      = Join-Path $OrcaProfile 'filament\Generic PLA.json'

$FixturePaths = @{
    benchy = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
    base   = Join-Path $RepoRoot 'tmp\base.stl'
}

# arm name -> resolution override (JSON string, per Orca's profile loader) or
# $null for "inherit the vendor default".
$Arms = [ordered]@{
    'default' = $null
    'floor'   = '0.001'
}
$ExpectedEffective = @{ 'default' = '0.01'; 'floor' = '0.001' }

$RunDir      = Join-Path $RepoRoot ('target/t80-resolution/' + $BatchId)
$GcodeDir    = Join-Path $RunDir 'gcode'
$ConfigDir   = Join-Path $RunDir 'orca-configs'
$OrcaOutRoot = Join-Path $RunDir 'orca-out'
$CsvPath     = Join-Path $RunDir 'rows.csv'
foreach ($d in @($RunDir, $GcodeDir, $ConfigDir, $OrcaOutRoot)) {
    New-Item -ItemType Directory -Path $d -Force | Out-Null
}

foreach ($p in @($OrcaExe, $OrcaMachine, $OrcaVendorProcess, $OrcaFilament, $FixturePaths[$Fixture])) {
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
        [string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [int]$PeakSampleMs = 100
    )
    Remove-Item -Force -ErrorAction SilentlyContinue $StderrPath
    $startArgs = @{
        FilePath               = $FilePath
        ArgumentList           = $ArgumentList
        PassThru               = $true
        NoNewWindow            = $true
        RedirectStandardError  = $StderrPath
    }
    if ($StdoutPath) { $startArgs.RedirectStandardOutput = $StdoutPath }
    $proc = Start-Process @startArgs
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
    $nativeTimes = Get-ProcessTimesSeconds -Handle $processHandle
    [pscustomobject]@{
        WallSeconds         = $nativeTimes.WallSeconds
        CpuSeconds          = $nativeTimes.CpuSeconds
        PeakWorkingSetBytes = $maxWs
        ExitCode            = $proc.ExitCode
    }
}

function ConvertTo-CsvField {
    param([string]$Value)
    if ($Value -match '[",\r\n]') { return '"' + ($Value -replace '"', '""') + '"' }
    return $Value
}

function Get-GcodeSectionCounts {
    param([Parameter(Mandatory = $true)][string]$Path)
    $counts = [ordered]@{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^;\s*FEATURE:\s*(.+?)\s*$') {
            $name = $Matches[1].Trim()
            if ($counts.Contains($name)) { $counts[$name]++ } else { $counts[$name] = 1 }
        }
    }
    return $counts
}

function Get-AppendixMap {
    param([Parameter(Mandatory = $true)][string]$Path)
    $map = @{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^\s*;\s*([a-z0-9_]+)\s*=\s*(.*?)\s*$') { $map[$Matches[1]] = $Matches[2] }
    }
    return $map
}

function New-OrcaConfig {
    param([string]$Generator, [string]$SupportFlag, [string]$Arm)
    $path = Join-Path $ConfigDir ('derived-' + $Arm + '-' + $Generator + '-support' + $SupportFlag + '.json')
    if (Test-Path -LiteralPath $path) { return $path }
    $j = Get-Content -LiteralPath $OrcaVendorProcess -Raw | ConvertFrom-Json
    $sets = [ordered]@{
        name                    = 't80-resolution-' + $Arm + '-' + $Generator + '-support' + $SupportFlag
        wall_generator          = $Generator
        wall_loops              = $Job.WallLoops
        sparse_infill_density   = $Job.SparseInfillDensity
        sparse_infill_pattern   = $Job.SparseInfillPattern
        enable_support          = $SupportFlag
        support_type            = $Job.SupportType
        layer_height            = $Job.LayerHeight
        initial_layer_print_height = $Job.FirstLayerHeight
    }
    if ($null -ne $Arms[$Arm]) { $sets['resolution'] = $Arms[$Arm] }
    foreach ($k in $sets.Keys) {
        $j | Add-Member -NotePropertyName $k -NotePropertyValue ([string]$sets[$k]) -Force
    }
    $j | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $path -Encoding utf8
    return $path
}

function Assert-EqualValue {
    param([string]$What, [string]$Actual, [string]$Expected, [string]$RunId)
    if ($Actual -ne $Expected) {
        throw "measurement rejected ($RunId): $What is '$Actual', expected '$Expected'"
    }
}

$rows = [System.Collections.Generic.List[string]]::new()
$rows.Add('run_id,batch_id,cell,fixture,generator,supports,arm,run_index,warmup,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,feature_counts_json,resolution_effective,config_path,output_path,exit_code,timestamp_utc')

$invokeRun = {
    param([string]$Generator, [string]$Supports, [string]$Arm, [int]$RunIndex, [bool]$Warmup)
    $cell = $Generator + '-' + $Supports
    $tag = if ($Warmup) { 'w' } else { 'm' }
    $runId = "$Fixture-$cell-$Arm-$tag$RunIndex"
    $supportFlag = if ($Supports -eq 'on') { '1' } else { '0' }
    $derived = New-OrcaConfig -Generator $Generator -SupportFlag $supportFlag -Arm $Arm
    $outDir = Join-Path $OrcaOutRoot $runId
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null

    Write-Host ("[t80] " + (Get-Date).ToString('HH:mm:ss') + " START " + $runId)
    $loadSettings = $OrcaMachine + ';' + $derived
    $argList = @(
        '--slice', '0',
        '--load-settings', ('"' + $loadSettings + '"'),
        '--load-filaments', ('"' + $OrcaFilament + '"'),
        '--outputdir', ('"' + $outDir + '"'),
        ('"' + $FixturePaths[$Fixture] + '"')
    )
    $measured = Invoke-MeasuredProcess -FilePath $OrcaExe -ArgumentList $argList `
        -StdoutPath (Join-Path $outDir 'orca.log') -StderrPath (Join-Path $outDir 'orca.stderr.log')

    if ($measured.ExitCode -ne 0) { throw "measurement rejected ($runId): Orca exit code $($measured.ExitCode)" }
    $plate = Join-Path $outDir 'plate_1.gcode'
    if (-not (Test-Path -LiteralPath $plate)) { throw "measurement rejected ($runId): plate_1.gcode missing" }
    $gcodePath = Join-Path $GcodeDir ($runId + '.gcode')
    Move-Item -LiteralPath $plate -Destination $gcodePath -Force

    $cfg = Get-AppendixMap -Path $gcodePath
    if ($cfg.Count -lt 50) { throw "measurement rejected ($runId): appendix missing or truncated" }
    Assert-EqualValue 'wall_generator' $cfg['wall_generator'] $Generator $runId
    Assert-EqualValue 'wall_loops' $cfg['wall_loops'] $Job.WallLoops $runId
    Assert-EqualValue 'sparse_infill_density' $cfg['sparse_infill_density'] $Job.SparseInfillDensity $runId
    Assert-EqualValue 'sparse_infill_pattern' $cfg['sparse_infill_pattern'] $Job.SparseInfillPattern $runId
    Assert-EqualValue 'enable_support' $cfg['enable_support'] $supportFlag $runId
    Assert-EqualValue 'support_type' $cfg['support_type'] $Job.SupportType $runId
    Assert-EqualValue 'layer_height' $cfg['layer_height'] $Job.LayerHeight $runId
    Assert-EqualValue 'resolution' $cfg['resolution'] $ExpectedEffective[$Arm] $runId

    $bytes = (Get-Item -LiteralPath $gcodePath).Length
    $sha = (Get-FileHash -Path $gcodePath -Algorithm SHA256).Hash.ToLower()
    $featureCounts = (Get-GcodeSectionCounts -Path $gcodePath) | ConvertTo-Json -Compress
    $ratio = if ($measured.WallSeconds -gt 0) { $measured.CpuSeconds / $measured.WallSeconds } else { 0 }
    $warm = if ($Warmup) { 1 } else { 0 }
    $row = @(
        $runId, $BatchId, $cell, $Fixture, $Generator, $Supports, $Arm, $RunIndex, $warm,
        ('{0:F3}' -f $measured.WallSeconds), ('{0:F3}' -f $measured.CpuSeconds), ('{0:F4}' -f $ratio),
        $measured.PeakWorkingSetBytes, $bytes, $sha, $featureCounts, $cfg['resolution'],
        $derived, $gcodePath, $measured.ExitCode, (Get-Date).ToUniversalTime().ToString('o')
    ) | ForEach-Object { ConvertTo-CsvField ([string]$_) }
    $rows.Add($row -join ',')
    Write-Host ("[t80] " + (Get-Date).ToString('HH:mm:ss') + " END   " + $runId +
        (" wall={0:F2}s cpu={1:F2}s bytes={2} resolution={3}" -f $measured.WallSeconds, $measured.CpuSeconds, $bytes, $cfg['resolution']))
}

foreach ($cell in $Cells) {
    $parts = $cell.Split('-')
    $Generator = $parts[0]
    $Supports = $parts[1]

    for ($w = 0; $w -lt $Warmups; $w++) {
        foreach ($arm in $Arms.Keys) { & $invokeRun $Generator $Supports $arm 0 $true }
    }
    for ($r = 1; $r -le $Pairs; $r++) {
        $order = if ($r % 2 -eq 1) { @('default', 'floor') } else { @('floor', 'default') }
        foreach ($arm in $order) { & $invokeRun $Generator $Supports $arm $r $false }
    }
}

$rows | Set-Content -LiteralPath $CsvPath -Encoding utf8
Write-Host "[t80] wrote $CsvPath"
