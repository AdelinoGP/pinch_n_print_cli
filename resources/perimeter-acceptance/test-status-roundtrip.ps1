# Regression for the benchmark CSV -> retained rows -> acceptance summary seam.
# Synthetic timings isolate the gates; this script never runs a slicer and is
# not performance evidence. Load real function definitions without executing
# the runner's campaign entry point.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$WorkspaceRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$scratch = Join-Path $WorkspaceRoot 'target\perimeter-acceptance-status-test'
New-Item -ItemType Directory -Force -Path $scratch | Out-Null
$SummaryPath = Join-Path $scratch 'summary.json'
$InvariantCulture = [Globalization.CultureInfo]::InvariantCulture

function Read-ScriptAst {
    param([string]$Path)
    $tokens = $null
    $errors = $null
    $ast = [Management.Automation.Language.Parser]::ParseFile($Path, [ref]$tokens, [ref]$errors)
    if ($errors.Count -ne 0) { throw "parse failed: $Path`: $errors" }
    return $ast
}

$runnerAst = Read-ScriptAst (Join-Path $PSScriptRoot 'run-acceptance.ps1')
foreach ($function in $runnerAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst]
}, $false)) {
    Invoke-Expression $function.Extent.Text
}
$assignments = $runnerAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$SamplesPerCell'
}, $false)
if ($assignments.Count -ne 1) { throw 'expected one runner sample-count assignment' }
Invoke-Expression $assignments[0].Extent.Text
if ($SamplesPerCell -le 0) { throw 'expected nonempty retained samples' }

# Derive the producer's actual schema, not a hand-maintained list of columns.
$benchAst = Read-ScriptAst (Join-Path $PSScriptRoot 'run_bench.ps1')
$assignments = $benchAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$header'
}, $false)
if ($assignments.Count -ne 1) { throw 'expected one benchmark CSV header assignment' }
Invoke-Expression $assignments[0].Extent.Text
$columns = $header.Split(',')
$output = Join-Path $scratch 'synthetic.gcode'
Set-Content -LiteralPath $output -Value '; wall_generator = classic' -Encoding utf8
$provenance = [PSCustomObject]@{ path = 'synthetic://status-roundtrip/config.json'; config_sha256 = '' }

function Assert-Equal {
    param($Actual, $Expected, [string]$Context)
    if ($Actual -cne $Expected) { throw "$Context`: expected '$Expected', got '$Actual'" }
}

function Read-TestRows {
    param([string]$CaseName, [string]$Variant, [hashtable]$Overrides)
    $time = if ($Variant -eq 'baseline') { 20 } else { 10 }
    $raw = @(for ($i = 0; $i -lt $SamplesPerCell; $i++) {
        $values = [ordered]@{}
        foreach ($column in $columns) { $values[$column] = '' }
        $values['label'] = "synthetic-$Variant-$i"
        $values['expected_generator'] = 'classic'
        $values['validated_generator'] = 'classic'
        $values['completion_status'] = 'degraded'
        $values['degraded'] = 'True'
        $values['non_fatal_error_count'] = 1
        $values['fatal_error_count'] = 0
        $values['exit_code'] = 0
        $values['cpu_seconds'] = $time
        $values['wall_seconds'] = $time
        foreach ($key in $Overrides.Keys) {
            if (-not $values.Contains($key)) { throw "unknown benchmark column: $key" }
            $values[$key] = $Overrides[$key]
        }
        [PSCustomObject]$values
    })
    $csv = Join-Path $scratch "$CaseName-$Variant.csv"
    $raw | Export-Csv -LiteralPath $csv -NoTypeInformation
    $rows = @(Convert-BenchRows $csv $output 'synthetic-status-roundtrip' 'classic' $Variant $Variant $provenance)
    Assert-Equal $rows.Count $SamplesPerCell "$CaseName/$Variant retained count"
    for ($i = 0; $i -lt $rows.Count; $i++) {
        Assert-Equal $rows[$i].status $raw[$i].completion_status "$CaseName/$Variant completion status"
        Assert-Equal $rows[$i].generator_marker $raw[$i].validated_generator "$CaseName/$Variant validated generator"
        Assert-Equal $rows[$i].degraded ([int][bool]::Parse($raw[$i].degraded)) "$CaseName/$Variant degraded flag"
        Assert-Equal $rows[$i].nonfatal_errors $raw[$i].non_fatal_error_count "$CaseName/$Variant non-fatal count"
        Assert-Equal $rows[$i].fatal_errors $raw[$i].fatal_error_count "$CaseName/$Variant fatal count"
    }
    return $rows
}

$cases = @(
    @{ name = 'nonfatal-count-changed'; candidate = @{ non_fatal_error_count = 2 }; expected = 'DROP' },
    @{ name = 'identical-degraded-counts'; expected = 'KEEP' },
    @{ name = 'identical-clean-status'; baseline = @{ completion_status = 'ok'; degraded = 'False'; non_fatal_error_count = 0 }; candidate = @{ completion_status = 'ok'; degraded = 'False'; non_fatal_error_count = 0 }; expected = 'KEEP' },
    @{ name = 'completion-status-changed'; candidate = @{ completion_status = 'ok' }; expected = 'DROP' },
    @{ name = 'degraded-flag-changed'; candidate = @{ degraded = 'False' }; expected = 'DROP' },
    @{ name = 'candidate-fatal'; candidate = @{ fatal_error_count = 1 }; expected = 'DROP' },
    @{ name = 'baseline-fatal'; baseline = @{ fatal_error_count = 1 }; expected = 'DROP' },
    # G-code fallback still says classic; CSV's independently validated marker
    # must take precedence rather than hiding this disagreement.
    @{ name = 'validated-generator-changed'; candidate = @{ validated_generator = 'arachne' }; expected = 'DROP' },
    @{ name = 'exactness-failed'; exactness = $false; expected = 'DROP' },
    @{ name = 'cpu-ranges-touch'; candidate = @{ cpu_seconds = 20 }; expected = 'inconclusive' },
    @{ name = 'wall-ranges-touch'; candidate = @{ wall_seconds = 20 }; expected = 'inconclusive' },
    @{ name = 'cpu-regresses'; candidate = @{ cpu_seconds = 30 }; expected = 'DROP' },
    @{ name = 'wall-regresses'; candidate = @{ wall_seconds = 30 }; expected = 'DROP' }
)

foreach ($case in $cases) {
    $baselineOverrides = if ($case.ContainsKey('baseline')) { $case.baseline } else { @{} }
    $candidateOverrides = if ($case.ContainsKey('candidate')) { $case.candidate } else { @{} }
    $exactness = if ($case.ContainsKey('exactness')) { $case.exactness } else { $true }
    $baseline = @(Read-TestRows $case.name 'baseline' $baselineOverrides)
    $candidate = @(Read-TestRows $case.name 'candidate' $candidateOverrides)
    $cell = New-CellSummary 'synthetic-status-roundtrip' 'classic' 'classic' $exactness $baseline $candidate '' '' $null $provenance
    Assert-Equal $cell.decision $case.expected "$($case.name) cell decision"
    $summary = [PSCustomObject]@{
        status = Get-OverallDecision @($cell)
        cells = @($cell)
    }
    Write-Summary $summary $false
    $saved = Get-Content -LiteralPath $SummaryPath -Raw | ConvertFrom-Json
    Assert-Equal $saved.status $case.expected "$($case.name) saved decision"
    Assert-Equal $saved.cells.Count 1 "$($case.name) saved cell count"
    foreach ($variant in @('baseline', 'candidate')) {
        $inputRows = if ($variant -eq 'baseline') { $baseline } else { $candidate }
        $savedCell = $saved.cells[0]
        Assert-Equal $savedCell.$variant.rows.Count $SamplesPerCell "$($case.name)/$variant saved row count"
        $nonfatal = if ($variant -eq 'baseline') {
            if ($baselineOverrides.ContainsKey('non_fatal_error_count')) { $baselineOverrides.non_fatal_error_count } else { 1 }
        } else {
            if ($candidateOverrides.ContainsKey('non_fatal_error_count')) { $candidateOverrides.non_fatal_error_count } else { 1 }
        }
        Assert-Equal $savedCell."nonfatal_errors_$variant" ($SamplesPerCell * $nonfatal) "$($case.name)/$variant saved non-fatal total"
        Assert-Equal $savedCell."fatal_errors_$variant" ($SamplesPerCell * $inputRows[0].fatal_errors) "$($case.name)/$variant saved fatal total"
        Assert-Equal $savedCell."degraded_$variant" $inputRows[0].degraded "$($case.name)/$variant saved degraded flag"
        Assert-Equal $savedCell."status_$variant" $inputRows[0].status "$($case.name)/$variant saved status"
        for ($i = 0; $i -lt $SamplesPerCell; $i++) {
            Assert-Equal $savedCell.$variant.rows[$i].nonfatal_errors $nonfatal "$($case.name)/$variant saved row non-fatal count"
            Assert-Equal $savedCell.$variant.rows[$i].fatal_errors $inputRows[$i].fatal_errors "$($case.name)/$variant saved row fatal count"
            Assert-Equal $savedCell.$variant.rows[$i].degraded $inputRows[$i].degraded "$($case.name)/$variant saved row degraded flag"
            Assert-Equal $savedCell.$variant.rows[$i].status $inputRows[$i].status "$($case.name)/$variant saved row status"
            Assert-Equal $savedCell.$variant.rows[$i].generator_marker $inputRows[$i].generator_marker "$($case.name)/$variant saved row generator"
        }
    }
    Write-Output "PASS $($case.name): $($cell.decision)"
}
Write-Output "PASS: $($cases.Count) CSV status round-trip cases; synthetic timings only"
