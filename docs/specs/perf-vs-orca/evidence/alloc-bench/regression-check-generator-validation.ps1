<# Regression checks for generator evidence validation; no slicer is launched. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'validate_measurement.ps1')

$root = Join-Path $PSScriptRoot 'generator-validation-fixtures'
New-Item -ItemType Directory -Path $root -Force | Out-Null
function Write-Case {
    param([string]$Name, [string]$Generator, [string]$Holder, [bool]$IncludeStderr = $true, [bool]$IncludeMarker = $true)
    $config = Join-Path $root "$Name.json"
    $gcode = Join-Path $root "$Name.gcode"
    $stderr = Join-Path $root "$Name.stderr"
    @{ wall_generator = $Generator } | ConvertTo-Json | Set-Content $config
    if ($IncludeMarker) { "; wall_generator = $Generator`nG1 X1 Y1" | Set-Content $gcode } else { 'G1 X1 Y1' | Set-Content $gcode }
    if ($IncludeStderr) {
        @(
            "Info: module x: claim 'perimeter-generator' already held by 'com.core.$Holder-perimeters'"
            '{"schema_version":"1.5.0","event":"slice_complete","status":"ok","degraded":false,"fatal_error_count":0,"non_fatal_error_count":0}'
        ) | Set-Content $stderr
    } else { Remove-Item -Force -ErrorAction SilentlyContinue $stderr }
    return @{ ConfigPath = $config; OutputPath = $gcode; StderrPath = $stderr }
}
function Assert-Accepts($Case, [string]$Expected) {
    Test-MeasurementEvidence @Case -ExpectedGenerator $Expected -ExitCode 0
}
function Assert-Rejects($Case, [string]$Expected, [int]$ExitCode = 0) {
    try { Test-MeasurementEvidence @Case -ExpectedGenerator $Expected -ExitCode $ExitCode; throw "case unexpectedly accepted: $Expected" } catch { if ($_.Exception.Message -like 'case unexpectedly accepted*') { throw } }
}

# Regression: an Arachne-labelled run with a Classic config must not count.
Assert-Rejects (Write-Case 'arachne-label-classic-config' 'classic' 'classic') 'arachne'
Assert-Accepts (Write-Case 'valid-classic' 'classic' 'classic') 'classic'
Assert-Accepts (Write-Case 'valid-arachne' 'arachne' 'arachne') 'arachne'
Assert-Rejects (Write-Case 'missing-claim-evidence' 'classic' 'classic' $false) 'classic'
Assert-Rejects (Write-Case 'contradictory-claim-evidence' 'classic' 'arachne') 'classic'
Assert-Rejects (Write-Case 'missing-gcode-marker' 'classic' 'classic' $true $false) 'classic'
$malformed = Write-Case 'malformed-json' 'classic' 'classic'; Add-Content $malformed.StderrPath '{not-json'; Assert-Rejects $malformed 'classic'
$nonzero = Write-Case 'nonzero-exit' 'classic' 'classic'; Assert-Rejects $nonzero 'classic' 1
$missingConfig = Write-Case 'missing-config-key' 'classic' 'classic'; '{}' | Set-Content $missingConfig.ConfigPath; Assert-Rejects $missingConfig 'classic'
$duplicateMarker = Write-Case 'duplicate-marker' 'classic' 'classic'; Add-Content $duplicateMarker.OutputPath '; wall_generator = Classic'; Assert-Rejects $duplicateMarker 'classic'
$missingCompletion = Write-Case 'missing-completion' 'classic' 'classic'; "claim 'perimeter-generator' already held by 'com.core.classic-perimeters'" | Set-Content $missingCompletion.StderrPath; Assert-Rejects $missingCompletion 'classic'
Write-Output 'generator evidence regression checks passed'
