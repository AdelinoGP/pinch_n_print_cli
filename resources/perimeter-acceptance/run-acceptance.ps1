<#
.SYNOPSIS
    Run the controlled exact-perimeter acceptance schedule.

.DESCRIPTION
    Dry-run mode only exercises the six-cell schedule and validator.  The
    corpus modes validate every artifact before invoking the benchmark and
    never build or commit automatically.

    The exactness gate uses structural equivalence, not byte-strict SHA-256
    equality (user decision 2026-09-11).  Classic G-code emission has real,
    pre-existing nondeterminism that has been proven not to move timings;
    tolerances are calibrated from measured same-exe noise: lines +/-0.1%,
    TYPE markers +/-1%, and E totals +/-0.5%.  The gate keeps strict generator
    markers and records per-metric deltas for audit.  Arachne remains
    byte-deterministic and passes trivially.

    run_bench.ps1 in this directory is a tracked verbatim copy of
    tmp/alloc-bench/run_bench.ps1.  Keep that file byte-identical.
#>
[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$Campaign,
    [string]$Workload,
    [ValidateSet('classic', 'arachne')][string]$ExpectedGenerator,
    [string]$CorpusRoot = 'tmp/rtree_query_corpus/',
    [int]$Threads = 12,
    [string]$ExePath,
    [string]$ModuleDir,
    [string]$BaselineExePath,
    [string]$CandidateExePath,
    [string]$BaselineModuleDir,
    [string]$CandidateModuleDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$SamplesPerCell = 4
$WarmupPerCell = 1
$WorkspaceRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$BenchPath = Join-Path $PSScriptRoot 'run_bench.ps1'
$SummaryPath = Join-Path $WorkspaceRoot 'target\perimeter-acceptance\summary.json'
$InvariantCulture = [Globalization.CultureInfo]::InvariantCulture

function Get-WorkspacePath {
    param([Parameter(Mandatory = $true)][string]$Path)
    if ([IO.Path]::IsPathRooted($Path)) { return [IO.Path]::GetFullPath($Path) }
    return [IO.Path]::GetFullPath((Join-Path $WorkspaceRoot $Path))
}

function Get-ConfiguredPath {
    param([string]$Value, [string]$EnvironmentName, [Parameter(Mandatory = $true)][string]$DefaultPath)
    if (-not [string]::IsNullOrWhiteSpace($Value)) { return Get-WorkspacePath $Value }
    $environmentValue = [Environment]::GetEnvironmentVariable($EnvironmentName)
    if (-not [string]::IsNullOrWhiteSpace($environmentValue)) { return Get-WorkspacePath $environmentValue }
    return Get-WorkspacePath $DefaultPath
}

function Get-CellSpecifications {
    return @(
        [PSCustomObject]@{ workload = 'supports-off-benchy'; generator = 'classic' },
        [PSCustomObject]@{ workload = 'supports-off-benchy'; generator = 'arachne' },
        [PSCustomObject]@{ workload = 'tree-support-benchy'; generator = 'classic' },
        [PSCustomObject]@{ workload = 'tree-support-benchy'; generator = 'arachne' },
        [PSCustomObject]@{ workload = 'tree-support-base'; generator = 'classic' },
        [PSCustomObject]@{ workload = 'tree-support-base'; generator = 'arachne' }
    )
}

function Resolve-CorpusFile {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string[]]$Candidates,
        [Parameter(Mandatory = $true)][string]$Canonical
    )
    foreach ($candidate in $Candidates) {
        $path = Join-Path $Root $candidate
        if (Test-Path -LiteralPath $path -PathType Leaf) { return [IO.Path]::GetFullPath($path) }
    }
    return [IO.Path]::GetFullPath((Join-Path $Root $Canonical))
}

function Get-CellArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$CellWorkload,
        [Parameter(Mandatory = $true)][string]$CellGenerator,
        [Parameter(Mandatory = $true)][string]$ResolvedCorpusRoot,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineExe,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateExe,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineModuleDir,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateModuleDir
    )
    $workloadRoot = Join-Path $ResolvedCorpusRoot $CellWorkload
    $modelPath = Resolve-CorpusFile $workloadRoot @('model.stl', ($CellWorkload + '.stl')) 'model.stl'
    $configPath = Resolve-CorpusFile $workloadRoot @(
        ($CellGenerator + '.json'), ('config-' + $CellGenerator + '.json'),
        ('config/' + $CellGenerator + '.json'), ($CellGenerator + '.ini'),
        ('config-' + $CellGenerator + '.ini'), ('config/' + $CellGenerator + '.ini')
    ) ($CellGenerator + '.json')
    $referencePath = Resolve-CorpusFile $workloadRoot @(
        ('reference-' + $CellGenerator + '.gcode'), ('reference/' + $CellGenerator + '.gcode')
    ) ('reference-' + $CellGenerator + '.gcode')

    # Corpus paths are deliberately checked first.  This makes an absent local
    # corpus fail before any executable, module, build, or timing operation.
    $missing = [System.Collections.Generic.List[string]]::new()
    if (-not (Test-Path -LiteralPath $modelPath -PathType Leaf)) { [void]$missing.Add($modelPath) }
    if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) { [void]$missing.Add($configPath) }
    if (-not (Test-Path -LiteralPath $referencePath -PathType Leaf)) { [void]$missing.Add($referencePath) }
    if (-not (Test-Path -LiteralPath $ResolvedBaselineExe -PathType Leaf)) { [void]$missing.Add($ResolvedBaselineExe) }
    if (-not (Test-Path -LiteralPath $ResolvedCandidateExe -PathType Leaf)) { [void]$missing.Add($ResolvedCandidateExe) }
    if (-not (Test-Path -LiteralPath $ResolvedBaselineModuleDir -PathType Container)) { [void]$missing.Add($ResolvedBaselineModuleDir) }
    if (-not (Test-Path -LiteralPath $ResolvedCandidateModuleDir -PathType Container)) { [void]$missing.Add($ResolvedCandidateModuleDir) }
    if (-not (Test-Path -LiteralPath $BenchPath -PathType Leaf)) { [void]$missing.Add($BenchPath) }
    $configSha256 = if (Test-Path -LiteralPath $configPath -PathType Leaf) { Get-Sha256 $configPath } else { '' }
    return [PSCustomObject]@{
        workload = $CellWorkload
        generator = $CellGenerator
        model_path = $modelPath
        config_path = $configPath
        config_provenance = [PSCustomObject]@{ path = $configPath; config_sha256 = $configSha256 }
        reference_path = $referencePath
        missing = @($missing.ToArray())
    }
}

function Get-PropertyValue {
    param([Parameter(Mandatory = $true)]$Object, [Parameter(Mandatory = $true)][string[]]$Names)
    foreach ($name in $Names) {
        $property = @($Object.PSObject.Properties | Where-Object { $_.Name -ieq $name }) | Select-Object -First 1
        if ($null -ne $property) { return $property.Value }
    }
    return $null
}

function Convert-ToDouble {
    param($Value)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $null }
    try { return [double]::Parse(([string]$Value).Trim(), [Globalization.NumberStyles]::Float, $InvariantCulture) }
    catch { return $null }
}

function Convert-ToCount {
    param($Value)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return 0 }
    $text = ([string]$Value).Trim()
    if ($text -match '^(?i:true|yes|on|degraded)$') { return 1 }
    if ($text -match '^(?i:false|no|off|ok|none)$') { return 0 }
    $number = Convert-ToDouble $Value
    if ($null -eq $number) { return 0 }
    return [int][Math]::Max(0, [Math]::Round($number))
}

function Get-OutputMarker {
    param([Parameter(Mandatory = $true)][string]$OutputPath)
    if (-not (Test-Path -LiteralPath $OutputPath -PathType Leaf)) { return '' }
    $content = Get-Content -LiteralPath $OutputPath -Raw -ErrorAction SilentlyContinue
    if ($null -eq $content) { return '' }
    $match = [regex]::Match($content, '(?im)(?:generator|perimeter)\s*[:=]\s*(classic|arachne)\b')
    if ($match.Success) { return $match.Groups[1].Value.ToLowerInvariant() }
    return ''
}

function Convert-BenchRows {
    param(
        [Parameter(Mandatory = $true)][string]$ResultsPath,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$CellWorkload,
        [Parameter(Mandatory = $true)][string]$CellGenerator,
        [Parameter(Mandatory = $true)][string]$Variant,
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)]$ConfigProvenance
    )
    if (-not (Test-Path -LiteralPath $ResultsPath -PathType Leaf)) { throw "benchmark did not create results: $ResultsPath" }
    $rawRows = @(Import-Csv -LiteralPath $ResultsPath)
    if ($rawRows.Count -eq 0) { throw "benchmark results are empty: $ResultsPath" }
    $fallbackMarker = Get-OutputMarker $OutputPath
    $rows = [System.Collections.Generic.List[object]]::new()
    foreach ($raw in $rawRows) {
        $cpu = Convert-ToDouble (Get-PropertyValue $raw @('cpu_s', 'cpu_seconds', 'process_cpu_s', 'process_cpu_seconds', 'cpu'))
        $wall = Convert-ToDouble (Get-PropertyValue $raw @('wall_s', 'wall_seconds', 'process_wall_s', 'process_wall_seconds', 'wall'))
        if ($null -eq $cpu -or $null -eq $wall) { throw "benchmark result lacks CPU/wall seconds: $ResultsPath" }
        $markerValue = Get-PropertyValue $raw @('generator_marker', 'generator', 'marker', 'generator_claim', 'generator_name')
        $marker = if ($null -eq $markerValue -or [string]::IsNullOrWhiteSpace([string]$markerValue)) { $fallbackMarker } else { ([string]$markerValue).Trim().ToLowerInvariant() }
        $statusValue = Get-PropertyValue $raw @('status', 'completion_status', 'result')
        $degradedValue = Get-PropertyValue $raw @('degraded', 'degraded_count', 'preexisting_degraded')
        $nonfatalValue = Get-PropertyValue $raw @('nonfatal_errors', 'non_fatal_errors', 'nonfatal', 'non_fatal', 'nonfatal_count')
        $fatalValue = Get-PropertyValue $raw @('fatal_errors', 'fatal_error_count', 'fatal')
        $rows.Add([PSCustomObject]@{
            label = $Label
            workload = $CellWorkload
            generator = $CellGenerator
            variant = $Variant
            cpu_seconds = $cpu
            wall_seconds = $wall
            generator_marker = $marker
            status = if ($null -eq $statusValue) { '' } else { [string]$statusValue }
            degraded = Convert-ToCount $degradedValue
            nonfatal_errors = Convert-ToCount $nonfatalValue
            fatal_errors = Convert-ToCount $fatalValue
            config_provenance = $ConfigProvenance
            output_path = $OutputPath
        })
    }
    return @($rows.ToArray())
}

function Invoke-Benchmark {
    param(
        [Parameter(Mandatory = $true)]$Artifacts,
        [Parameter(Mandatory = $true)][string]$Variant,
        [Parameter(Mandatory = $true)][string]$ResolvedExe,
        [Parameter(Mandatory = $true)][string]$ResolvedModuleDir,
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][bool]$Warmup
    )
    $outputPath = Join-Path $RunRoot ($Label + '.gcode')
    $resultsPath = Join-Path $RunRoot ($Label + '.csv')
    $benchLogPath = Join-Path $RunRoot ($Label + '.bench.log')
    # run_bench appends one row per internal run.  Each runner invocation uses
    # -Runs 1, so discard a prior invocation's CSV before starting to ensure
    # Convert-BenchRows retains exactly one row for this sample.
    if (Test-Path -LiteralPath $resultsPath -PathType Leaf) {
        Remove-Item -LiteralPath $resultsPath -Force -ErrorAction Stop
    }
    $arguments = @(
        '-NoProfile', '-File', '.\run_bench.ps1',
        '-ExePath', $ResolvedExe, '-InputModel', $Artifacts.model_path,
        '-Config', $Artifacts.config_path, '-ModuleDir', $ResolvedModuleDir,
        '-OutputPath', $outputPath, '-Threads', [string]$Threads,
        '-Label', $Label, '-Runs', '1', '-ResultsPath', $resultsPath,
        '-PeakSampleMs', '100', '-ExpectedGenerator', $Artifacts.generator
    )
    if ($Warmup) { $arguments += '-Warmup' }
    Push-Location $PSScriptRoot
    try {
        & pwsh @arguments *> $benchLogPath
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
    if ($exitCode -ne 0) { throw "benchmark failed ($exitCode); see $benchLogPath" }
    return Convert-BenchRows $resultsPath $outputPath $Artifacts.workload $Artifacts.generator $Variant $Label $Artifacts.config_provenance
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-GcodeMetrics {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "G-code output does not exist: $Path" }

    $lines = [IO.File]::ReadAllLines($Path)
    $typeMarkerCount = 0
    $eTotal = 0.0
    $generatorMarker = ''
    $ePattern = [regex]::new('E(-?\d+\.\d+)')
    foreach ($line in $lines) {
        if ([regex]::IsMatch($line, '^;TYPE:')) { $typeMarkerCount++ }
        if ([string]::IsNullOrWhiteSpace($generatorMarker)) {
            $generatorMatch = [regex]::Match($line, '^;\s*wall_generator\s*=\s*(\w+)')
            if ($generatorMatch.Success) { $generatorMarker = $generatorMatch.Groups[1].Value.ToLowerInvariant() }
        }
        foreach ($eMatch in $ePattern.Matches($line)) {
            $eTotal += [double]::Parse($eMatch.Groups[1].Value, [Globalization.NumberStyles]::Float, $InvariantCulture)
        }
    }
    return [PSCustomObject]@{
        total_lines = @($lines).Count
        type_marker_count = $typeMarkerCount
        e_total = $eTotal
        generator_marker = $generatorMarker
    }
}

function Invoke-Exactness {
    param(
        [Parameter(Mandatory = $true)]$Artifacts,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineExe,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateExe,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineModuleDir,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateModuleDir,
        [Parameter(Mandatory = $true)][string]$RunRoot
    )
    # Excluded warmup invocations provide output for the exactness comparison;
    # their timing rows are never retained in the acceptance summary.
    $baselineRows = @(Invoke-Benchmark $Artifacts 'baseline-exactness' $ResolvedBaselineExe $ResolvedBaselineModuleDir $RunRoot 'exactness-baseline' $true)
    $candidateRows = @(Invoke-Benchmark $Artifacts 'candidate-exactness' $ResolvedCandidateExe $ResolvedCandidateModuleDir $RunRoot 'exactness-candidate' $true)
    $referenceMetrics = Get-GcodeMetrics $Artifacts.reference_path
    $baselineMetrics = Get-GcodeMetrics $baselineRows[0].output_path
    $candidateMetrics = Get-GcodeMetrics $candidateRows[0].output_path
    $expectedGenerator = ([string]$Artifacts.generator).ToLowerInvariant()
    $epsilon = 1e-12
    $lineTolerance = 0.001
    $typeTolerance = 0.01
    $eTolerance = 0.005

    $lineDelta = @{}
    $typeDelta = @{}
    $eDelta = @{}
    foreach ($variant in @(
        [PSCustomObject]@{ name = 'baseline'; metrics = $baselineMetrics },
        [PSCustomObject]@{ name = 'candidate'; metrics = $candidateMetrics }
    )) {
        $runMetrics = $variant.metrics
        $lineDenominator = [double]$referenceMetrics.total_lines
        $lineDelta[$variant.name] = if ($lineDenominator -eq 0) {
            if ([double]$runMetrics.total_lines -eq 0) { 0.0 } else { 1.0 }
        }
        else {
            [Math]::Abs(([double]$runMetrics.total_lines - $lineDenominator) / $lineDenominator)
        }
        $typeDenominator = [Math]::Max([double]$referenceMetrics.type_marker_count, 1.0)
        $typeDelta[$variant.name] = [Math]::Abs(([double]$runMetrics.type_marker_count - [double]$referenceMetrics.type_marker_count) / $typeDenominator)
        $eDenominator = [Math]::Max([Math]::Abs([double]$referenceMetrics.e_total), $epsilon)
        $eDelta[$variant.name] = [Math]::Abs(([double]$runMetrics.e_total - [double]$referenceMetrics.e_total) / $eDenominator)
    }

    $baselineMarker = [string]$baselineMetrics.generator_marker
    $candidateMarker = [string]$candidateMetrics.generator_marker
    $referenceMarker = [string]$referenceMetrics.generator_marker
    $markersMatch =
        -not [string]::IsNullOrWhiteSpace($baselineMarker) -and $baselineMarker -eq $expectedGenerator -and
        -not [string]::IsNullOrWhiteSpace($candidateMarker) -and $candidateMarker -eq $expectedGenerator -and
        -not [string]::IsNullOrWhiteSpace($referenceMarker) -and $referenceMarker -eq $expectedGenerator
    $structurallyEquivalent =
        $lineDelta.baseline -le $lineTolerance -and $typeDelta.baseline -le $typeTolerance -and $eDelta.baseline -le $eTolerance -and
        $lineDelta.candidate -le $lineTolerance -and $typeDelta.candidate -le $typeTolerance -and $eDelta.candidate -le $eTolerance
    return [PSCustomObject]@{
        passed = [bool]($markersMatch -and $structurallyEquivalent)
        baseline_marker = $baselineMarker
        candidate_marker = $candidateMarker
        reference_marker = $referenceMarker
        deltas = [PSCustomObject]@{
            baseline = [PSCustomObject]@{
                lines_pct = [Math]::Round($lineDelta.baseline * 100.0, 4)
                types_pct = [Math]::Round($typeDelta.baseline * 100.0, 4)
                e_pct = [Math]::Round($eDelta.baseline * 100.0, 4)
            }
            candidate = [PSCustomObject]@{
                lines_pct = [Math]::Round($lineDelta.candidate * 100.0, 4)
                types_pct = [Math]::Round($typeDelta.candidate * 100.0, 4)
                e_pct = [Math]::Round($eDelta.candidate * 100.0, 4)
            }
        }
    }
}

function Get-Median {
    param([double[]]$Values)
    $items = @($Values | Sort-Object)
    if ($items.Count -eq 0) { return $null }
    $middle = [int]($items.Count / 2)
    if (($items.Count % 2) -eq 1) { return [double]$items[$middle] }
    return ([double]$items[$middle - 1] + [double]$items[$middle]) / 2.0
}

function Get-SumProperty {
    param([object[]]$Rows, [Parameter(Mandatory = $true)][string]$PropertyName)
    $sum = 0
    foreach ($row in @($Rows)) { $sum += [int]$row.$PropertyName }
    return $sum
}

function Get-MaxProperty {
    param([object[]]$Rows, [Parameter(Mandatory = $true)][string]$PropertyName)
    $maximum = 0
    foreach ($row in @($Rows)) { $maximum = [Math]::Max($maximum, [int]$row.$PropertyName) }
    return $maximum
}

function Test-Markers {
    param([object[]]$Rows, [Parameter(Mandatory = $true)][string]$Expected)
    $markers = @($Rows | ForEach-Object { [string]$_.generator_marker })
    if ($markers.Count -eq 0) { return $false }
    foreach ($marker in $markers) {
        if ([string]::IsNullOrWhiteSpace($marker) -or $marker.ToLowerInvariant() -ne $Expected) { return $false }
    }
    return $true
}

function Test-RangeSeparated {
    param([object[]]$BaselineRows, [object[]]$CandidateRows, [Parameter(Mandatory = $true)][string]$PropertyName)
    if (@($BaselineRows).Count -ne $SamplesPerCell -or @($CandidateRows).Count -ne $SamplesPerCell) { return $false }
    $baseline = @($BaselineRows | ForEach-Object { [double]$_.${PropertyName} })
    $candidate = @($CandidateRows | ForEach-Object { [double]$_.${PropertyName} })
    return (($candidate | Measure-Object -Maximum).Maximum -lt ($baseline | Measure-Object -Minimum).Minimum)
}

function Test-RangesOverlap {
    param([object[]]$BaselineRows, [object[]]$CandidateRows, [Parameter(Mandatory = $true)][string]$PropertyName)
    if (@($BaselineRows).Count -eq 0 -or @($CandidateRows).Count -eq 0) { return $false }
    $baseline = @($BaselineRows | ForEach-Object { [double]$_.${PropertyName} })
    $candidate = @($CandidateRows | ForEach-Object { [double]$_.${PropertyName} })
    $baselineMin = ($baseline | Measure-Object -Minimum).Minimum
    $baselineMax = ($baseline | Measure-Object -Maximum).Maximum
    $candidateMin = ($candidate | Measure-Object -Minimum).Minimum
    $candidateMax = ($candidate | Measure-Object -Maximum).Maximum
    return -not ($candidateMax -lt $baselineMin -or $baselineMax -lt $candidateMin)
}

function Get-CellDecision {
    param([Parameter(Mandatory = $true)]$Cell)
    if (-not $Cell.exactness_passed -or -not $Cell.generator_marker_match) { return 'DROP' }
    # Empty/empty is an identical status; empty/set and differing non-empty
    # statuses are generator disagreements and can never be retained.
    if ($Cell.status_baseline -ne $Cell.status_candidate) { return 'DROP' }
    if ($Cell.degraded_baseline -ne $Cell.degraded_candidate -or $Cell.nonfatal_errors_baseline -ne $Cell.nonfatal_errors_candidate) { return 'DROP' }
    if ($Cell.fatal_errors_baseline -gt 0 -or $Cell.fatal_errors_candidate -gt 0) { return 'DROP' }
    if ($Cell.cpu_separated -and $Cell.wall_separated) { return 'KEEP' }
    if ((Test-RangesOverlap $Cell.baseline.rows $Cell.candidate.rows 'cpu_seconds') -or
        (Test-RangesOverlap $Cell.baseline.rows $Cell.candidate.rows 'wall_seconds')) { return 'inconclusive' }
    return 'DROP'
}

function New-CellSummary {
    param(
        [Parameter(Mandatory = $true)][string]$CellWorkload,
        [Parameter(Mandatory = $true)][string]$CellGenerator,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][bool]$ExactnessPassed,
        [Parameter(Mandatory = $true)][object[]]$BaselineRows,
        [Parameter(Mandatory = $true)][object[]]$CandidateRows,
        [string]$BaselineMarker,
        [string]$CandidateMarker,
        [AllowNull()][object]$ExactnessDeltas,
        [Parameter(Mandatory = $true)]$ConfigProvenance
    )
    $baseline = @($BaselineRows)
    $candidate = @($CandidateRows)
    if ([string]::IsNullOrWhiteSpace($BaselineMarker)) { $BaselineMarker = if ($baseline.Count -gt 0) { [string]$baseline[0].generator_marker } else { '' } }
    if ([string]::IsNullOrWhiteSpace($CandidateMarker)) { $CandidateMarker = if ($candidate.Count -gt 0) { [string]$candidate[0].generator_marker } else { '' } }
    $statusBaseline = if ($baseline.Count -gt 0) { [string]$baseline[0].status } else { '' }
    $statusCandidate = if ($candidate.Count -gt 0) { [string]$candidate[0].status } else { '' }
    $status = if (-not [string]::IsNullOrWhiteSpace($statusCandidate)) { $statusCandidate } else { $statusBaseline }
    $cpuSeparated = Test-RangeSeparated $baseline $candidate 'cpu_seconds'
    $wallSeparated = Test-RangeSeparated $baseline $candidate 'wall_seconds'
    $baselineCpu = Get-Median @($baseline | ForEach-Object { [double]$_.cpu_seconds })
    $candidateCpu = Get-Median @($candidate | ForEach-Object { [double]$_.cpu_seconds })
    $baselineWall = Get-Median @($baseline | ForEach-Object { [double]$_.wall_seconds })
    $candidateWall = Get-Median @($candidate | ForEach-Object { [double]$_.wall_seconds })
    $cpuRatio = if ($null -ne $baselineCpu -and $baselineCpu -ne 0 -and $null -ne $candidateCpu) { $candidateCpu / $baselineCpu } else { $null }
    $wallRatio = if ($null -ne $baselineWall -and $baselineWall -ne 0 -and $null -ne $candidateWall) { $candidateWall / $baselineWall } else { $null }
    $cell = [PSCustomObject]@{
        workload = $CellWorkload
        generator = $CellGenerator
        expected_generator = $Expected
        exactness_passed = $ExactnessPassed
        exactness_deltas = $ExactnessDeltas
        cpu_separated = [bool]$cpuSeparated
        wall_separated = [bool]$wallSeparated
        baseline = [PSCustomObject]@{
            cpu_samples = @($baseline | ForEach-Object { $_.cpu_seconds })
            wall_samples = @($baseline | ForEach-Object { $_.wall_seconds })
            rows = $baseline
        }
        candidate = [PSCustomObject]@{
            cpu_samples = @($candidate | ForEach-Object { $_.cpu_seconds })
            wall_samples = @($candidate | ForEach-Object { $_.wall_seconds })
            rows = $candidate
        }
        status = $status
        status_baseline = $statusBaseline
        status_candidate = $statusCandidate
        config_provenance = $ConfigProvenance
        cpu_ratio = $cpuRatio
        wall_ratio = $wallRatio
        generator_marker = [PSCustomObject]@{ baseline = $BaselineMarker; candidate = $CandidateMarker }
        generator_marker_match = (Test-Markers $baseline $Expected) -and (Test-Markers $candidate $Expected) -and ($BaselineMarker -eq $CandidateMarker)
        degraded_baseline = Get-MaxProperty $baseline 'degraded'
        degraded_candidate = Get-MaxProperty $candidate 'degraded'
        nonfatal_errors_baseline = Get-SumProperty $baseline 'nonfatal_errors'
        nonfatal_errors_candidate = Get-SumProperty $candidate 'nonfatal_errors'
        fatal_errors_baseline = Get-SumProperty $baseline 'fatal_errors'
        fatal_errors_candidate = Get-SumProperty $candidate 'fatal_errors'
        decision = ''
    }
    $cell.decision = Get-CellDecision $cell
    return $cell
}

function Get-OverallDecision {
    param([object[]]$Cells)
    $decisions = @($Cells | ForEach-Object { [string]$_.decision })
    if ($decisions -contains 'DROP') { return 'DROP' }
    if ($decisions -contains 'inconclusive') { return 'inconclusive' }
    if ($decisions.Count -gt 0 -and @($decisions | Where-Object { $_ -eq 'KEEP' }).Count -eq $decisions.Count) { return 'KEEP' }
    return 'DROP'
}

function Get-SummaryConfigProvenance {
    param([Parameter(Mandatory = $true)][object[]]$Cells)
    return @($Cells | ForEach-Object { $_.config_provenance })
}

function New-SyntheticConfigProvenance {
    param(
        [Parameter(Mandatory = $true)][string]$CellWorkload,
        [Parameter(Mandatory = $true)][string]$CellGenerator
    )
    # DryRun has no config bytes to hash; retain an explicit synthetic path and
    # an empty hash rather than presenting a fabricated file hash as evidence.
    return [PSCustomObject]@{
        path = "synthetic://$CellWorkload/$CellGenerator/config.json"
        config_sha256 = ''
    }
}

function New-SyntheticRow {
    param(
        [string]$CellWorkload, [string]$CellGenerator, [string]$Variant,
        [int]$Index, [double]$Cpu, [double]$Wall, [Parameter(Mandatory = $true)]$ConfigProvenance
    )
    return [PSCustomObject]@{
        label = "synthetic-$Variant-$Index"; workload = $CellWorkload; generator = $CellGenerator; variant = $Variant
        cpu_seconds = $Cpu; wall_seconds = $Wall; generator_marker = $CellGenerator; status = 'ok'
        degraded = 0; nonfatal_errors = 0; fatal_errors = 0; config_provenance = $ConfigProvenance
        output_path = "synthetic://$CellWorkload/$CellGenerator/$Variant/$Index"
    }
}

function New-DryRunSummary {
    $cells = [System.Collections.Generic.List[object]]::new()
    $index = 0
    foreach ($spec in Get-CellSpecifications) {
        $configProvenance = New-SyntheticConfigProvenance $spec.workload $spec.generator
        $baselineCpu = @((20.0 + $index), (21.0 + $index), (22.0 + $index), (23.0 + $index))
        $baselineWall = @((10.0 + $index), (11.0 + $index), (12.0 + $index), (13.0 + $index))
        if ($index -eq 0) {
            $candidateCpu = @(22.5, 23.5, 24.5, 25.5); $candidateWall = @(12.5, 13.5, 14.5, 15.5)
        }
        elseif ($index -eq 1) {
            $candidateCpu = @(1.0, 2.0, 3.0, 4.0); $candidateWall = @(1.0, 2.0, 3.0, 4.0)
        }
        else {
            $candidateCpu = @((10.0 + $index), (11.0 + $index), (12.0 + $index), (13.0 + $index))
            $candidateWall = @((5.0 + $index), (6.0 + $index), (7.0 + $index), (8.0 + $index))
        }
        $baselineRows = [System.Collections.Generic.List[object]]::new()
        $candidateRows = [System.Collections.Generic.List[object]]::new()
        for ($sample = 0; $sample -lt $SamplesPerCell; $sample++) {
            $baselineRows.Add((New-SyntheticRow $spec.workload $spec.generator 'baseline' ($sample + 1) $baselineCpu[$sample] $baselineWall[$sample] $configProvenance))
            $candidateRows.Add((New-SyntheticRow $spec.workload $spec.generator 'candidate' ($sample + 1) $candidateCpu[$sample] $candidateWall[$sample] $configProvenance))
        }
        if ($index -eq 2) {
            foreach ($row in $baselineRows) { $row.degraded = 1; $row.nonfatal_errors = 2 }
            foreach ($row in $candidateRows) { $row.degraded = 1; $row.nonfatal_errors = 2 }
        }
        $cells.Add((New-CellSummary $spec.workload $spec.generator $spec.generator ($index -ne 1) @($baselineRows.ToArray()) @($candidateRows.ToArray()) $spec.generator $spec.generator $null $configProvenance))
        $index++
    }
    return [PSCustomObject]@{
        status = Get-OverallDecision @($cells.ToArray())
        automatic_commit = $false
        threads = $Threads
        samples_per_cell = $SamplesPerCell
        warmup_per_cell = $WarmupPerCell
        config_provenance = Get-SummaryConfigProvenance @($cells.ToArray())
        cells = @($cells.ToArray())
    }
}

function Write-Summary {
    param([Parameter(Mandatory = $true)]$Summary, [Parameter(Mandatory = $true)][bool]$Print)
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $SummaryPath) | Out-Null
    $json = $Summary | ConvertTo-Json -Depth 10
    Set-Content -LiteralPath $SummaryPath -Value $json -Encoding utf8
    if ($Print) { Write-Output $json }
}

function New-NotRunCell {
    param(
        [Parameter(Mandatory = $true)]$Spec,
        [Parameter(Mandatory = $true)]$ConfigProvenance
    )
    return [PSCustomObject]@{
        workload = $Spec.workload; generator = $Spec.generator; expected_generator = $Spec.generator
        exactness_passed = $false; cpu_separated = $false; wall_separated = $false
        exactness_deltas = $null
        baseline = [PSCustomObject]@{ cpu_samples = @(); wall_samples = @(); rows = @() }
        candidate = [PSCustomObject]@{ cpu_samples = @(); wall_samples = @(); rows = @() }
        status = ''; status_baseline = ''; status_candidate = ''
        config_provenance = $ConfigProvenance
        cpu_ratio = $null; wall_ratio = $null
        generator_marker = [PSCustomObject]@{ baseline = ''; candidate = '' }; generator_marker_match = $false
        degraded_baseline = 0; degraded_candidate = 0; nonfatal_errors_baseline = 0; nonfatal_errors_candidate = 0
         fatal_errors_baseline = 0; fatal_errors_candidate = 0; decision = 'not-run'
    }
}

function Invoke-Cell {
    param(
        [Parameter(Mandatory = $true)]$Spec, [Parameter(Mandatory = $true)]$Artifacts,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineExe,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateExe,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineModuleDir,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateModuleDir
    )
    $runDirectory = Join-Path $WorkspaceRoot ('target\perimeter-acceptance\runs\' + $Spec.workload + '-' + $Spec.generator)
    New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null
    $exactness = Invoke-Exactness $Artifacts $ResolvedBaselineExe $ResolvedCandidateExe $ResolvedBaselineModuleDir $ResolvedCandidateModuleDir $runDirectory
    if (-not $exactness.passed) {
        return New-CellSummary $Spec.workload $Spec.generator $Spec.generator $false @() @() $exactness.baseline_marker $exactness.candidate_marker $exactness.deltas $Artifacts.config_provenance
    }
    [void](Invoke-Benchmark $Artifacts 'baseline' $ResolvedBaselineExe $ResolvedBaselineModuleDir $runDirectory 'warmup-baseline' $true)
    [void](Invoke-Benchmark $Artifacts 'candidate' $ResolvedCandidateExe $ResolvedCandidateModuleDir $runDirectory 'warmup-candidate' $true)
    $baselineRows = [System.Collections.Generic.List[object]]::new()
    $candidateRows = [System.Collections.Generic.List[object]]::new()
    $counts = @{ baseline = 0; candidate = 0 }
    # A is baseline and B is candidate: ABBA followed by BAAB.
    foreach ($variant in @('baseline', 'candidate', 'candidate', 'baseline', 'candidate', 'baseline', 'baseline', 'candidate')) {
        $counts[$variant]++
        $label = 'measured-{0}-{1}' -f $variant, $counts[$variant]
        if ($variant -eq 'baseline') {
            foreach ($row in @(Invoke-Benchmark $Artifacts 'baseline' $ResolvedBaselineExe $ResolvedBaselineModuleDir $runDirectory $label $false)) { $baselineRows.Add($row) }
        }
        else {
            foreach ($row in @(Invoke-Benchmark $Artifacts 'candidate' $ResolvedCandidateExe $ResolvedCandidateModuleDir $runDirectory $label $false)) { $candidateRows.Add($row) }
        }
    }
    return New-CellSummary $Spec.workload $Spec.generator $Spec.generator $true @($baselineRows.ToArray()) @($candidateRows.ToArray()) $exactness.baseline_marker $exactness.candidate_marker $exactness.deltas $Artifacts.config_provenance
}

function Invoke-Campaign {
    param(
        [Parameter(Mandatory = $true)][string]$ResolvedCorpusRoot,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineExe,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateExe,
        [Parameter(Mandatory = $true)][string]$ResolvedBaselineModuleDir,
        [Parameter(Mandatory = $true)][string]$ResolvedCandidateModuleDir
    )
    $specifications = @(Get-CellSpecifications)
    $artifactsByCell = @{}
    foreach ($spec in $specifications) {
        $artifacts = Get-CellArtifacts $spec.workload $spec.generator $ResolvedCorpusRoot $ResolvedBaselineExe $ResolvedCandidateExe $ResolvedBaselineModuleDir $ResolvedCandidateModuleDir
        if (@($artifacts.missing).Count -gt 0) {
            Write-Output ('missing-artifact: {0}' -f $artifacts.missing[0]); exit 1
        }
        $artifactsByCell[('{0}/{1}' -f $spec.workload, $spec.generator)] = $artifacts
    }
    $cells = [System.Collections.Generic.List[object]]::new()
    for ($index = 0; $index -lt $specifications.Count; $index++) {
        $spec = $specifications[$index]
        $childStdout = Join-Path $WorkspaceRoot ('target\perimeter-acceptance\campaign-' + $index + '.stdout.log')
        $childStderr = Join-Path $WorkspaceRoot ('target\perimeter-acceptance\campaign-' + $index + '.stderr.log')
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $childStdout) | Out-Null
        Remove-Item -LiteralPath $SummaryPath -Force -ErrorAction SilentlyContinue
        $childArguments = @(
            '-NoProfile', '-File', $PSCommandPath, '-Workload', $spec.workload,
            '-ExpectedGenerator', $spec.generator, '-CorpusRoot', $ResolvedCorpusRoot,
            '-Threads', [string]$Threads, '-BaselineExePath', $ResolvedBaselineExe,
            '-CandidateExePath', $ResolvedCandidateExe, '-BaselineModuleDir', $ResolvedBaselineModuleDir,
            '-CandidateModuleDir', $ResolvedCandidateModuleDir
        )
        & pwsh @childArguments 1> $childStdout 2> $childStderr
        $childExitCode = $LASTEXITCODE
        if (-not (Test-Path -LiteralPath $SummaryPath -PathType Leaf)) {
            [Console]::Error.WriteLine(('acceptance-error: no summary for {0}/{1}' -f $spec.workload, $spec.generator))
            for ($remaining = $index; $remaining -lt $specifications.Count; $remaining++) {
                $remainingSpec = $specifications[$remaining]
                $remainingArtifacts = $artifactsByCell[('{0}/{1}' -f $remainingSpec.workload, $remainingSpec.generator)]
                $cells.Add((New-NotRunCell $remainingSpec $remainingArtifacts.config_provenance))
            }
            $failedSummary = [PSCustomObject]@{
                status = 'DROP'; automatic_commit = $false; threads = $Threads
                samples_per_cell = $SamplesPerCell; warmup_per_cell = $WarmupPerCell
                config_provenance = Get-SummaryConfigProvenance @($cells.ToArray())
                cells = @($cells.ToArray())
            }
            Write-Summary $failedSummary $true; exit 1
        }
        $childSummary = Get-Content -LiteralPath $SummaryPath -Raw | ConvertFrom-Json
        $cell = @($childSummary.cells)[0]
        $cells.Add($cell)
        if ($childExitCode -ne 0 -or $cell.decision -ne 'KEEP') {
            for ($remaining = $index + 1; $remaining -lt $specifications.Count; $remaining++) {
                $remainingSpec = $specifications[$remaining]
                $remainingArtifacts = $artifactsByCell[('{0}/{1}' -f $remainingSpec.workload, $remainingSpec.generator)]
                $cells.Add((New-NotRunCell $remainingSpec $remainingArtifacts.config_provenance))
            }
            break
        }
    }
    $summary = [PSCustomObject]@{
        status = Get-OverallDecision @($cells.ToArray()); automatic_commit = $false; threads = $Threads
        samples_per_cell = $SamplesPerCell; warmup_per_cell = $WarmupPerCell
        config_provenance = Get-SummaryConfigProvenance @($cells.ToArray())
        cells = @($cells.ToArray())
    }
    Write-Summary $summary $true
}

try {
    $hasWorkload = -not [string]::IsNullOrWhiteSpace($Workload)
    $hasGenerator = -not [string]::IsNullOrWhiteSpace($ExpectedGenerator)
    if ($DryRun -and ($Campaign -or $hasWorkload -or $hasGenerator)) { throw '-DryRun cannot be combined with campaign or cell arguments' }
    if ($Campaign -and ($hasWorkload -or $hasGenerator)) { throw '-Campaign cannot be combined with cell arguments' }
    if (-not $DryRun -and -not $Campaign -and (-not $hasWorkload -or -not $hasGenerator)) { throw 'single-cell mode requires -Workload and -ExpectedGenerator' }
    if ($Threads -le 0) { throw '-Threads must be positive' }
    if ($DryRun) { Write-Summary (New-DryRunSummary) $true; exit 0 }

    $resolvedCorpusRoot = Get-WorkspacePath $CorpusRoot
    $resolvedExe = Get-ConfiguredPath $ExePath 'PNP_PERIMETER_EXE' 'target/release/pnp_cli.exe'
    $resolvedModuleDir = Get-ConfiguredPath $ModuleDir 'PNP_PERIMETER_MODULE_DIR' 'target/release/modules'
    $resolvedBaselineExe = Get-ConfiguredPath $BaselineExePath 'PNP_PERIMETER_BASELINE_EXE' $resolvedExe
    $resolvedCandidateExe = Get-ConfiguredPath $CandidateExePath 'PNP_PERIMETER_CANDIDATE_EXE' $resolvedExe
    $resolvedBaselineModuleDir = Get-ConfiguredPath $BaselineModuleDir 'PNP_PERIMETER_BASELINE_MODULE_DIR' $resolvedModuleDir
    $resolvedCandidateModuleDir = Get-ConfiguredPath $CandidateModuleDir 'PNP_PERIMETER_CANDIDATE_MODULE_DIR' $resolvedModuleDir

    if ($Campaign) {
        Invoke-Campaign $resolvedCorpusRoot $resolvedBaselineExe $resolvedCandidateExe $resolvedBaselineModuleDir $resolvedCandidateModuleDir
        exit 0
    }
    $spec = [PSCustomObject]@{ workload = $Workload; generator = $ExpectedGenerator }
    $artifacts = Get-CellArtifacts $spec.workload $spec.generator $resolvedCorpusRoot $resolvedBaselineExe $resolvedCandidateExe $resolvedBaselineModuleDir $resolvedCandidateModuleDir
    if (@($artifacts.missing).Count -gt 0) { Write-Output ('missing-artifact: {0}' -f $artifacts.missing[0]); exit 1 }
    $cell = Invoke-Cell $spec $artifacts $resolvedBaselineExe $resolvedCandidateExe $resolvedBaselineModuleDir $resolvedCandidateModuleDir
    $summary = [PSCustomObject]@{
        status = $cell.decision; automatic_commit = $false; threads = $Threads
        samples_per_cell = $SamplesPerCell; warmup_per_cell = $WarmupPerCell
        config_provenance = @($cell.config_provenance); cells = @($cell)
    }
    Write-Summary $summary $true
    exit 0
}
catch {
    [Console]::Error.WriteLine(('acceptance-error: {0}' -f $_.Exception.Message))
    exit 1
}
