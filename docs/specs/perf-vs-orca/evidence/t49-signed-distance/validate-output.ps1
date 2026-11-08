# Read-only check through the standing measurement-evidence validator; no slices.
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Raw = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t49-signed-distance'
$Config = Join-Path $PSScriptRoot '..\t44-clip-universe-hoist\benchy-arachne.json'
$Results = @()
foreach ($Mode in @('ordinary', 'accelerated')) {
    foreach ($Phase in @('baseline', 'probe', 'baseline-profile')) {
        $Dir = Join-Path $Raw "$Mode\$Phase"
        $Evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath (Join-Path $Dir 'output.gcode') `
            -StderrPath (Join-Path $Dir 'stderr.jsonl') -ExpectedGenerator arachne -ExitCode 0
        if ($Evidence.CompletionStatus -ne 'ok' -or $Evidence.Degraded -or
            $Evidence.FatalErrorCount -ne 0 -or $Evidence.NonFatalErrorCount -ne 0) {
            throw "unclean output: $Mode $Phase"
        }
        $Results += [pscustomobject]@{ mode = $Mode; phase = $Phase; evidence = $Evidence }
    }
}
$Results | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 (Join-Path $PSScriptRoot 'output-validation.json')
Write-Host 'PASS: every captured output passes the standing config/generator/completion validator.'
