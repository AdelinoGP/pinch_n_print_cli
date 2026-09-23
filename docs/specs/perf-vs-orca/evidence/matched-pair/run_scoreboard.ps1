<#
.SYNOPSIS
    Matched-pair scoreboard rig (perf-vs-orca ticket 11, "Matched-pair rig and
    first scoreboard").

.DESCRIPTION
    Three-tool interleaved measurement schedule over the {benchy, base} x
    {classic, arachne} x {supports off, on} matrix: OrcaSlicer (vendor profile
    triple + generated string-typed override profile), PNP ordinary
    (target/release + modules/core-modules) and PNP accelerated
    (target/dist-accelerated/developer snapshot).

    Measurement core extends the alloc-bench/run_bench.ps1 lineage: process
    creation-to-exit wall clock and kernel+user CPU via GetProcessTimes, peak
    working set by polling, one CSV row per run. Reuse contract: keep this
    file's measurement math identical to resources/perimeter-acceptance/run_bench.ps1.

    Evidence policy (the map's fairness contract): labels prove nothing.
    Every PNP run is validated by
    resources/perimeter-acceptance/validate_measurement.ps1 (config
    wall_generator, gcode marker, slice_complete counters, claim-holder
    diagnostic) plus a gyroid dispatch check. Every Orca run is validated
    against its resolved-config dump (the `; key = value` gcode appendix).
    A validation failure aborts the batch.

    Rows append to <RunDir>/scoreboard-rows.csv (crash-safe); per-run section
    counts and appendix excerpts go to <RunDir>/stats/<run_id>.json. Raw
    gcode/stderr stay under <RunDir> (gitignored target/) per -KeepGcode.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 `
        -Fixtures calicat -Runs 1 -Warmups 0 -BatchId smoke
#>
[CmdletBinding()]
param(
    [ValidateSet('calicat', 'benchy', 'base')]
    [string[]]$Fixtures = @('benchy', 'base'),
    [ValidateSet('classic-off', 'arachne-off', 'classic-on', 'arachne-on')]
    [string[]]$Cells = @('classic-off', 'arachne-off', 'classic-on', 'arachne-on'),
    [ValidateSet('orca', 'pnp-ordinary', 'pnp-accelerated')]
    [string[]]$Tools = @('orca', 'pnp-ordinary', 'pnp-accelerated'),
    [int]$Runs = 3,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [ValidateSet('All', 'Last', 'None')][string]$KeepGcode = 'Last',
    [switch]$SkipFreshness,
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

# --- Matched job (must stay in lock-step with configs/ and the Orca overrides) --
$Job = @{
    LayerHeight       = '0.2'
    FirstLayerHeight  = '0.2'
    WallLoops         = '2'
    SparseInfillDensity = '20%'
    SparseInfillPattern = 'gyroid'
    SupportType       = 'tree(auto)'
}

# --- Paths -------------------------------------------------------------------
$ConfigDir   = Join-Path $PSScriptRoot 'configs'
$Validator   = Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1'
$OrcaExe     = 'C:\Program Files\OrcaSlicer\orca-slicer.exe'
$OrcaProfile = 'C:\Program Files\OrcaSlicer\resources\profiles\BBL'
$OrcaMachine = Join-Path $OrcaProfile 'machine\Bambu Lab X1 Carbon 0.4 nozzle.json'
$OrcaVendorProcess = Join-Path $OrcaProfile 'process\0.20mm Standard @BBL X1C.json'
$OrcaFilament = Join-Path $OrcaProfile 'filament\Generic PLA.json'

$FixturePaths = @{
    calicat = Join-Path $RepoRoot 'resources\calicat.stl'
    benchy  = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
    base    = Join-Path $RepoRoot 'tmp\base.stl'
}

$ToolSpecs = @{
    'orca' = @{
        Exe = $OrcaExe
        ModuleDir = $null
    }
    'pnp-ordinary' = @{
        Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
    }
    'pnp-accelerated' = @{
        Exe = Join-Path $RepoRoot 'target\dist-accelerated\developer\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\dist-accelerated\developer\modules'
    }
}

$RunDir   = Join-Path $RepoRoot ('target\matched-pair\' + $BatchId)
$GcodeDir = Join-Path $RunDir 'gcode'
$StatsDir = Join-Path $RunDir 'stats'
$OrcaOutRoot = Join-Path $RunDir 'orca-out'
$CsvPath  = Join-Path $RunDir 'scoreboard-rows.csv'
foreach ($d in @($RunDir, $GcodeDir, $StatsDir, $OrcaOutRoot)) {
    New-Item -ItemType Directory -Path $d -Force | Out-Null
}

. $Validator

# --- Measurement core (run_bench.ps1 lineage) --------------------------------
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
        CpuSeconds = ([ProcessTimes]::T($kernel) + [ProcessTimes]::T($user)) / 10000000.0
    }
}

function Invoke-MeasuredProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [hashtable]$Environment,
        [string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [int]$PeakSampleMs = 100
    )
    Remove-Item -Force -ErrorAction SilentlyContinue $StderrPath
    $startArgs = @{
        FilePath = $FilePath
        ArgumentList = $ArgumentList
        PassThru = $true
        NoNewWindow = $true
        RedirectStandardError = $StderrPath
    }
    if ($StdoutPath) { $startArgs.RedirectStandardOutput = $StdoutPath }
    if ($Environment) { $startArgs.Environment = $Environment }
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
        WallSeconds = $nativeTimes.WallSeconds
        CpuSeconds = $nativeTimes.CpuSeconds
        PeakWorkingSetBytes = $maxWs
        ExitCode = $proc.ExitCode
    }
}

function ConvertTo-CsvField {
    param([string]$Value)
    if ($Value -match '[",\r\n]') {
        return '"' + ($Value -replace '"', '""') + '"'
    }
    return $Value
}

function Get-GcodeSectionCounts {
    param([Parameter(Mandatory = $true)][string]$Path, [Parameter(Mandatory = $true)][string]$MarkerRegex)
    $counts = @{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match $MarkerRegex) {
            $name = $Matches[1].Trim()
            if ($counts.ContainsKey($name)) { $counts[$name]++ } else { $counts[$name] = 1 }
        }
    }
    return $counts
}

function Get-AppendixMap {
    param([Parameter(Mandatory = $true)][string]$Path)
    $map = @{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^\s*;\s*([a-z0-9_]+)\s*=\s*(.*?)\s*$') {
            $map[$Matches[1]] = $Matches[2]
        }
    }
    return $map
}

function Assert-EqualValue {
    param([string]$What, [string]$Actual, [string]$Expected, [string]$RunId)
    if ($Actual -ne $Expected) {
        throw "measurement rejected ($RunId): $What is '$Actual', expected '$Expected'"
    }
}

# --- Evidence validation -----------------------------------------------------
function Test-OrcaEvidence {
    param(
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$ExpectedGenerator,
        [Parameter(Mandatory = $true)][string]$ExpectedSupportFlag,
        [Parameter(Mandatory = $true)][int]$ExitCode,
        [Parameter(Mandatory = $true)][string]$RunId
    )
    if ($ExitCode -ne 0) { throw "measurement rejected ($RunId): Orca exit code was $ExitCode" }
    if (-not (Test-Path -LiteralPath $OutputPath) -or (Get-Item -LiteralPath $OutputPath).Length -eq 0) {
        throw "measurement rejected ($RunId): gcode is missing or empty ($OutputPath)"
    }
    $cfg = Get-AppendixMap -Path $OutputPath
    if ($cfg.Count -lt 50) { throw "measurement rejected ($RunId): gcode config appendix missing or truncated" }
    Assert-EqualValue 'wall_generator' $cfg['wall_generator'] $ExpectedGenerator $RunId
    Assert-EqualValue 'wall_loops' $cfg['wall_loops'] $Job.WallLoops $RunId
    Assert-EqualValue 'sparse_infill_density' $cfg['sparse_infill_density'] $Job.SparseInfillDensity $RunId
    Assert-EqualValue 'sparse_infill_pattern' $cfg['sparse_infill_pattern'] $Job.SparseInfillPattern $RunId
    Assert-EqualValue 'enable_support' $cfg['enable_support'] $ExpectedSupportFlag $RunId
    Assert-EqualValue 'support_type' $cfg['support_type'] $Job.SupportType $RunId
    Assert-EqualValue 'layer_height' $cfg['layer_height'] $Job.LayerHeight $RunId
    return $cfg
}

function Get-PnpAppendixExcerpt {
    param([Parameter(Mandatory = $true)][string]$OutputPath)
    $cfg = Get-AppendixMap -Path $OutputPath
    $excerpt = @{}
    foreach ($k in @('wall_generator', 'sparse_infill_pattern', 'sparse_infill_density', 'support_type', 'enable_support', 'layer_height', 'nozzle_diameter')) {
        if ($cfg.ContainsKey($k)) { $excerpt[$k] = $cfg[$k] }
    }
    return $excerpt
}

function Test-PnpGyroidDispatch {
    param(
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [Parameter(Mandatory = $true)][string]$RunId
    )
    $stderr = Get-Content -LiteralPath $StderrPath -Raw
    if ($stderr -notmatch 'com\.core\.gyroid-infill') {
        throw "measurement rejected ($RunId): no com.core.gyroid-infill dispatch evidence in stderr"
    }
}

# --- Orca config generation (string-typed overrides; numbers are ignored) ----
function New-OrcaDerivedConfig {
    param(
        [Parameter(Mandatory = $true)][string]$Generator,
        [Parameter(Mandatory = $true)][string]$SupportFlag
    )
    $derivedDir = Join-Path $RunDir 'orca-configs'
    New-Item -ItemType Directory -Path $derivedDir -Force | Out-Null
    $path = Join-Path $derivedDir ('derived-' + $Generator + '-support' + $SupportFlag + '.json')
    if (Test-Path -LiteralPath $path) { return $path }
    $j = Get-Content -LiteralPath $OrcaVendorProcess -Raw | ConvertFrom-Json
    $sets = [ordered]@{
        name = 'pnp-matched-' + $Generator + '-support' + $SupportFlag
        wall_generator = $Generator
        wall_loops = $Job.WallLoops
        sparse_infill_density = $Job.SparseInfillDensity
        sparse_infill_pattern = $Job.SparseInfillPattern
        enable_support = $SupportFlag
        support_type = $Job.SupportType
        layer_height = $Job.LayerHeight
        initial_layer_print_height = $Job.FirstLayerHeight
    }
    foreach ($k in $sets.Keys) {
        $j | Add-Member -NotePropertyName $k -NotePropertyValue ([string]$sets[$k]) -Force
    }
    $j | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $path
    return $path
}

# --- Preflight (fail-closed) -------------------------------------------------
function Assert-Preflight {
    foreach ($fixture in $Fixtures) {
        if (-not (Test-Path -LiteralPath $FixturePaths[$fixture])) {
            throw "missing-artifact: $($FixturePaths[$fixture])"
        }
    }
    foreach ($tool in $Tools) {
        $spec = $ToolSpecs[$tool]
        if (-not (Test-Path -LiteralPath $spec.Exe)) { throw "missing-artifact: $($spec.Exe)" }
    }
    if ($Tools -contains 'orca') {
        foreach ($p in @($OrcaMachine, $OrcaVendorProcess, $OrcaFilament)) {
            if (-not (Test-Path -LiteralPath $p)) { throw "missing-artifact: $p" }
        }
    }
    if (-not $SkipFreshness) {
        Push-Location $RepoRoot
        try {
            cargo xtask build-guests --check | Out-Host
            if ($LASTEXITCODE -ne 0) { throw "preflight failed: cargo xtask build-guests --check exit $LASTEXITCODE" }
            if ($Tools -contains 'pnp-accelerated') {
                cargo xtask build-guests --accelerated --check | Out-Host
                if ($LASTEXITCODE -ne 0) { throw "preflight failed: cargo xtask build-guests --accelerated --check exit $LASTEXITCODE" }
            }
        } finally { Pop-Location }
    }
    foreach ($tool in @($Tools | Where-Object { $_ -like 'pnp-*' })) {
        $spec = $ToolSpecs[$tool]
        $diag = & $spec.Exe module diagnose --module-dir $spec.ModuleDir | Out-String
        $parsed = $diag | ConvertFrom-Json
        if (-not $parsed.pass) { throw "preflight failed: module diagnose pass=false for $tool" }
        $bad = @($parsed.modules | Where-Object { $_.provenance -ne 'external' })
        if ($bad.Count -gt 0) {
            throw "preflight failed: $($bad.Count) non-external modules for $tool"
        }
        if ([int]$parsed.modules_loaded -lt 24) {
            throw "preflight failed: modules_loaded=$($parsed.modules_loaded) for $tool (expected >= 24)"
        }
    }
}

# --- CSV ---------------------------------------------------------------------
$CsvHeader = 'run_id,batch_id,cell,fixture,generator,supports,tool,run_index,warmup,threads,expected_generator,validated_generator,config_path,exe_path,module_dir,model_path,output_path,stderr_path,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,type_counts_json,completion_status,fatal_error_count,non_fatal_error_count,degraded,exit_code,timestamp_utc'
if (-not (Test-Path $CsvPath)) {
    Set-Content -Path $CsvPath -Value $CsvHeader -Encoding utf8
} else {
    $actualHeader = (Get-Content -LiteralPath $CsvPath -TotalCount 1)
    if ($actualHeader -ne $CsvHeader) { throw "results CSV header mismatch in $CsvPath" }
}

function Add-ScoreRow {
    param([hashtable]$Row)
    $line = @(
        (ConvertTo-CsvField $Row.run_id), (ConvertTo-CsvField $Row.batch_id), (ConvertTo-CsvField $Row.cell),
        (ConvertTo-CsvField $Row.fixture), $Row.generator, $Row.supports, $Row.tool, $Row.run_index, $Row.warmup,
        $Row.threads, (ConvertTo-CsvField $Row.expected_generator), (ConvertTo-CsvField $Row.validated_generator),
        (ConvertTo-CsvField $Row.config_path), (ConvertTo-CsvField $Row.exe_path), (ConvertTo-CsvField $Row.module_dir),
        (ConvertTo-CsvField $Row.model_path), (ConvertTo-CsvField $Row.output_path), (ConvertTo-CsvField $Row.stderr_path),
        ([math]::Round($Row.wall_seconds, 4)), ([math]::Round($Row.cpu_seconds, 4)), ([math]::Round($Row.cpu_wall_ratio, 4)),
        $Row.peak_workingset_bytes, $Row.gcode_bytes, (ConvertTo-CsvField $Row.sha256_of_gcode),
        (ConvertTo-CsvField $Row.type_counts_json), (ConvertTo-CsvField $Row.completion_status),
        $Row.fatal_error_count, $Row.non_fatal_error_count, $Row.degraded, $Row.exit_code, $Row.timestamp_utc
    ) -join ','
    Add-Content -Path $CsvPath -Value $line -Encoding utf8
}

# --- One run -----------------------------------------------------------------
function Invoke-ScoreRun {
    param(
        [Parameter(Mandatory = $true)][string]$Fixture,
        [Parameter(Mandatory = $true)][string]$Generator,
        [Parameter(Mandatory = $true)][string]$Supports,
        [Parameter(Mandatory = $true)][string]$Tool,
        [Parameter(Mandatory = $true)][int]$RunIndex,
        [bool]$Warmup
    )
    $cell = $Generator + '-' + $Supports
    $tag = if ($Warmup) { 'w' } else { 'm' }
    $runId = "$Fixture-$cell-$Tool-$tag$RunIndex"
    $supportFlag = if ($Supports -eq 'on') { '1' } else { '0' }
    $modelPath = $FixturePaths[$Fixture]
    $stderrPath = Join-Path $RunDir ($runId + '.stderr.jsonl')
    Write-Host ("[scoreboard] " + (Get-Date).ToString('HH:mm:ss') + " START " + $runId)

    $measured = $null
    $gcodePath = $null
    $typeMarkerRegex = $null
    $validatedGenerator = ''
    $completionStatus = ''
    $fatalCount = ''
    $nonFatalCount = ''
    $degraded = ''
    $appendixExcerpt = @{}

    if ($Tool -eq 'orca') {
        $derived = New-OrcaDerivedConfig -Generator $Generator -SupportFlag $supportFlag
        $outDir = Join-Path $OrcaOutRoot $runId
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
        $loadSettings = $OrcaMachine + ';' + $derived
        $argList = @(
            '--slice', '0',
            '--load-settings', ('"' + $loadSettings + '"'),
            '--load-filaments', ('"' + $OrcaFilament + '"'),
            '--outputdir', ('"' + $outDir + '"'),
            ('"' + $modelPath + '"')
        )
        $measured = Invoke-MeasuredProcess -FilePath $OrcaExe -ArgumentList $argList -StdoutPath (Join-Path $outDir 'orca.log') -StderrPath $stderrPath
        $gcodePath = Join-Path $GcodeDir ($runId + '.gcode')
        Move-Item -LiteralPath (Join-Path $outDir 'plate_1.gcode') -Destination $gcodePath -Force
        $cfg = Test-OrcaEvidence -OutputPath $gcodePath -ExpectedGenerator $Generator -ExpectedSupportFlag $supportFlag -ExitCode $measured.ExitCode -RunId $runId
        $validatedGenerator = $Generator
        foreach ($k in @('wall_generator', 'wall_loops', 'sparse_infill_density', 'sparse_infill_pattern', 'enable_support', 'support_type', 'layer_height')) {
            $appendixExcerpt[$k] = $cfg[$k]
        }
        $completionStatus = 'n/a'
        $fatalCount = 'n/a'
        $nonFatalCount = 'n/a'
        $degraded = 'n/a'
        $typeMarkerRegex = '^;\s*FEATURE:\s*(.+?)\s*$'
    } else {
        $spec = $ToolSpecs[$Tool]
        $supportsState = if ($Supports -eq 'on') { 'on' } else { 'off' }
        $configPath = Join-Path $ConfigDir ('pnp-' + $Generator + '-supports-' + $supportsState + '.json')
        $gcodePath = Join-Path $GcodeDir ($runId + '.gcode')
        $argList = @(
            'slice',
            '--model', ('"' + $modelPath + '"'),
            '--output', ('"' + $gcodePath + '"'),
            '--config', ('"' + $configPath + '"'),
            '--module-dir', ('"' + $spec.ModuleDir + '"')
        )
        $envMap = @{ RAYON_NUM_THREADS = "$Threads" }
        $measured = Invoke-MeasuredProcess -FilePath $spec.Exe -ArgumentList $argList -Environment $envMap -StderrPath $stderrPath
        $evidence = Test-MeasurementEvidence -ConfigPath $configPath -OutputPath $gcodePath -StderrPath $stderrPath -ExpectedGenerator $Generator -ExitCode $measured.ExitCode
        Test-PnpGyroidDispatch -StderrPath $stderrPath -RunId $runId
        $validatedGenerator = $evidence.ValidatedGenerator
        $completionStatus = $evidence.CompletionStatus
        $fatalCount = $evidence.FatalErrorCount
        $nonFatalCount = $evidence.NonFatalErrorCount
        $degraded = if ($evidence.Degraded) { 'true' } else { 'false' }
        $appendixExcerpt = Get-PnpAppendixExcerpt -OutputPath $gcodePath
        $typeMarkerRegex = '^;\s*TYPE:\s*(.+?)\s*$'
    }

    $gcodeBytes = (Get-Item -LiteralPath $gcodePath).Length
    $sha = (Get-FileHash -Path $gcodePath -Algorithm SHA256).Hash.ToLower()
    $typeCounts = Get-GcodeSectionCounts -Path $gcodePath -MarkerRegex $typeMarkerRegex
    $stats = [ordered]@{
        run_id = $runId
        tool = $Tool
        fixture = $Fixture
        generator = $Generator
        supports = $Supports
        warmup = $Warmup
        type_counts = $typeCounts
        appendix = $appendixExcerpt
        gcode_bytes = $gcodeBytes
        sha256 = $sha
    }
    $statsPath = Join-Path $StatsDir ($runId + '.json')
    ($stats | ConvertTo-Json -Depth 5) | Set-Content -LiteralPath $statsPath -Encoding utf8

    $wall = $measured.WallSeconds
    $cpu = $measured.CpuSeconds
    Add-ScoreRow @{
        run_id = $runId; batch_id = $BatchId; cell = $cell; fixture = $Fixture
        generator = $Generator; supports = $Supports; tool = $Tool
        run_index = $RunIndex; warmup = $(if ($Warmup) { 1 } else { 0 }); threads = $Threads
        expected_generator = $Generator; validated_generator = $validatedGenerator
        config_path = $(if ($Tool -eq 'orca') { $derived } else { $configPath })
        exe_path = $ToolSpecs[$Tool].Exe; module_dir = $(if ($Tool -eq 'orca') { '' } else { $spec.ModuleDir })
        model_path = $modelPath; output_path = $gcodePath; stderr_path = $stderrPath
        wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = $(if ($wall -gt 0) { $cpu / $wall } else { 0 })
        peak_workingset_bytes = $measured.PeakWorkingSetBytes; gcode_bytes = $gcodeBytes
        sha256_of_gcode = $sha; type_counts_json = ($typeCounts | ConvertTo-Json -Compress)
        completion_status = $completionStatus; fatal_error_count = $fatalCount
        non_fatal_error_count = $nonFatalCount; degraded = $degraded
        exit_code = $measured.ExitCode
        timestamp_utc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    }
    Write-Host ("[scoreboard] " + (Get-Date).ToString('HH:mm:ss') + " DONE  " + $runId + (" wall={0:N1}s cpu={1:N1}s ratio={2:N2}" -f $wall, $cpu, $(if ($wall -gt 0) { $cpu / $wall } else { 0 })))
    return $gcodePath
}

# --- Schedule ----------------------------------------------------------------
Assert-Preflight
Write-Host ("[scoreboard] batch " + $BatchId + " fixtures=[" + ($Fixtures -join ',') + "] cells=[" + ($Cells -join ',') + "] tools=[" + ($Tools -join ',') + "] warmups=" + $Warmups + " runs=" + $Runs)

$lastGcode = @{}
foreach ($fixture in $Fixtures) {
    foreach ($cell in $Cells) {
        $parts = $cell -split '-'
        $generator = $parts[0]
        $supports = $parts[1]
        foreach ($tool in $Tools) { $lastGcode["$fixture/$cell/$tool"] = '' }

        for ($w = 1; $w -le $Warmups; $w++) {
            foreach ($tool in $Tools) {
                $lastGcode["$fixture/$cell/$tool"] = Invoke-ScoreRun -Fixture $fixture -Generator $generator -Supports $supports -Tool $tool -RunIndex $w -Warmup $true
            }
        }
        for ($r = 1; $r -le $Runs; $r++) {
            for ($i = 0; $i -lt $Tools.Count; $i++) {
                $tool = $Tools[($r - 1 + $i) % $Tools.Count]
                $lastGcode["$fixture/$cell/$tool"] = Invoke-ScoreRun -Fixture $fixture -Generator $generator -Supports $supports -Tool $tool -RunIndex $r -Warmup $false
            }
        }

        if ($KeepGcode -eq 'None') {
            foreach ($key in @($lastGcode.Keys | Where-Object { $_ -like "$fixture/$cell/*" })) {
                if ($lastGcode[$key]) { Remove-Item -Force -ErrorAction SilentlyContinue $lastGcode[$key] }
                $lastGcode[$key] = ''
            }
        } elseif ($KeepGcode -eq 'Last') {
            foreach ($key in @($lastGcode.Keys | Where-Object { $_ -like "$fixture/$cell/*" })) {
                $keep = $lastGcode[$key]
                $keyTool = $key.Split('/')[2]
                Get-ChildItem -Path $GcodeDir -Filter "$fixture-$cell-$keyTool-*.gcode" | Where-Object { $_.FullName -ne $keep } | Remove-Item -Force
            }
        }
    }
}

Write-Host ("[scoreboard] batch " + $BatchId + " complete. rows: " + $CsvPath)
