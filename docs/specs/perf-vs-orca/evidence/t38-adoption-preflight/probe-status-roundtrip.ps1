# Diagnostic only: exercise the real CSV reader and decision functions without
# invoking a slicer or changing the acceptance runner. Synthetic times are not
# performance evidence. A nonzero exit reproduces a fail-open status gate.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..\..\..'))
$runner = Join-Path $workspace 'resources\perimeter-acceptance\run-acceptance.ps1'
$bench = Join-Path $workspace 'resources\perimeter-acceptance\run_bench.ps1'
$scratch = Join-Path $workspace 'target\t38-status-roundtrip'
New-Item -ItemType Directory -Force -Path $scratch | Out-Null
$tokens = $null
$errors = $null
$runnerAst = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
if ($errors.Count -ne 0) { throw 'runner parse failed' }
foreach ($function in $runnerAst.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] }, $false)) {
    Invoke-Expression $function.Extent.Text
}
$samplesAssignment = $runnerAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$SamplesPerCell'
}, $false)
Invoke-Expression $samplesAssignment[0].Extent.Text
$InvariantCulture = [Globalization.CultureInfo]::InvariantCulture
$benchAst = [Management.Automation.Language.Parser]::ParseFile($bench, [ref]$tokens, [ref]$errors)
if ($errors.Count -ne 0) { throw 'benchmark parse failed' }
$headerAssignment = $benchAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$header'
}, $false)
Invoke-Expression $headerAssignment[0].Extent.Text
$columns = $header.Split(',')
$output = Join-Path $scratch 'synthetic.gcode'
Set-Content -LiteralPath $output -Value '; wall_generator = classic' -Encoding utf8
$provenance = [PSCustomObject]@{ path = 'synthetic://status-roundtrip'; config_sha256 = '' }
$variants = @{}
foreach ($variant in @('baseline', 'candidate')) {
    $nonfatal = if ($variant -eq 'baseline') { 1 } else { 2 }
    $syntheticTime = if ($variant -eq 'baseline') { 20 } else { 10 }
    $rows = @(for ($i = 0; $i -lt $SamplesPerCell; $i++) {
        $values = [ordered]@{}
        foreach ($column in $columns) { $values[$column] = '' }
        $values['label'] = "synthetic-$variant-$i"
        $values['validated_generator'] = 'classic'
        $values['completion_status'] = 'degraded'
        $values['degraded'] = 'True'
        $values['non_fatal_error_count'] = $nonfatal
        $values['fatal_error_count'] = 0
        $values['cpu_seconds'] = $syntheticTime
        $values['wall_seconds'] = $syntheticTime
        [PSCustomObject]$values
    })
    $csv = Join-Path $scratch "$variant.csv"
    $rows | Export-Csv -LiteralPath $csv -NoTypeInformation
    $variants[$variant] = @(Convert-BenchRows $csv $output 'synthetic-status-roundtrip' 'classic' $variant $variant $provenance)
}
$cell = New-CellSummary 'synthetic-status-roundtrip' 'classic' 'classic' $true $variants.baseline $variants.candidate 'classic' 'classic' $null $provenance
$result = [PSCustomObject]@{
    diagnostic_only = $true
    timing_evidence = $false
    input_nonfatal_per_row = [PSCustomObject]@{ baseline = 1; candidate = 2 }
    parsed_nonfatal_per_row = [PSCustomObject]@{ baseline = $variants.baseline[0].nonfatal_errors; candidate = $variants.candidate[0].nonfatal_errors }
    reported_nonfatal_totals = [PSCustomObject]@{ baseline = $cell.nonfatal_errors_baseline; candidate = $cell.nonfatal_errors_candidate }
    expected_decision = 'DROP'
    actual_decision = $cell.decision
}
$result | ConvertTo-Json -Depth 5
if ($cell.decision -ne 'DROP') { exit 1 }
exit 0
