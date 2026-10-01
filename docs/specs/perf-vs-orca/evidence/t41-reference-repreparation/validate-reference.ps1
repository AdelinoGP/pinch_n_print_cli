param(
    [Parameter(Mandatory)][string]$ConfigPath,
    [Parameter(Mandatory)][string]$OutputPath,
    [Parameter(Mandatory)][string]$StderrPath,
    [Parameter(Mandatory)][ValidateSet('classic', 'arachne')][string]$ExpectedGenerator,
    [Parameter(Mandatory)][int]$ExitCode
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../../../../resources/perimeter-acceptance/validate_measurement.ps1"
$result = Test-MeasurementEvidence @PSBoundParameters
if ($result.CompletionStatus -ne 'ok' -or $result.Degraded -or
    $result.FatalErrorCount -ne 0 -or $result.NonFatalErrorCount -ne 0) {
    throw "reference preparation rejected: expected clean completion, got $($result | ConvertTo-Json -Compress)"
}
$result | ConvertTo-Json -Compress
