<#
.SYNOPSIS
    Validates the non-timing evidence required for a generator measurement.

    The function is dot-sourced by run_bench.ps1 and can also be invoked as a
    standalone check. Evidence formats are the formats emitted by the CLI:
    the gcode header has a wall_generator marker and stderr has the scheduler's
    same-claim winner diagnostic.
#>
function Test-MeasurementEvidence {
    param(
        [Parameter(Mandatory = $true)][string]$ConfigPath,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [Parameter(Mandatory = $true)][ValidateSet('classic', 'arachne')][string]$ExpectedGenerator,
        [Parameter(Mandatory = $true)][int]$ExitCode
    )

    $expected = $ExpectedGenerator.ToLowerInvariant()
    if ($ExitCode -ne 0) { throw "measurement rejected: CLI exit code was $ExitCode" }
    if (-not (Test-Path -LiteralPath $OutputPath) -or (Get-Item -LiteralPath $OutputPath).Length -eq 0) {
        throw "measurement rejected: gcode is missing or empty ($OutputPath)"
    }
    if (-not (Test-Path -LiteralPath $StderrPath)) { throw "measurement rejected: stderr evidence is missing ($StderrPath)" }

    try { $config = Get-Content -LiteralPath $ConfigPath -Raw | ConvertFrom-Json } catch { throw "measurement rejected: invalid config JSON: $ConfigPath" }
    $configProperty = $config.PSObject.Properties | Where-Object Name -eq 'wall_generator'
    if ($null -eq $configProperty -or [string]::IsNullOrWhiteSpace([string]$config.wall_generator)) {
        throw 'measurement rejected: config must explicitly set wall_generator'
    }
    $configured = ([string]$config.wall_generator).ToLowerInvariant()
    if ($configured -notin @('classic', 'arachne') -or $configured -ne $expected) {
        throw "measurement rejected: config wall_generator '$configured' does not match expected '$expected'"
    }

    $gcode = Get-Content -LiteralPath $OutputPath -Raw
    $markers = [regex]::Matches($gcode, '(?im)^\s*;\s*wall_generator\s*=\s*(Classic|Arachne)\s*$')
    if ($markers.Count -ne 1) { throw "measurement rejected: expected exactly one gcode wall_generator marker, found $($markers.Count)" }
    $marked = $markers[0].Groups[1].Value.ToLowerInvariant()
    if ($marked -ne $expected) { throw "measurement rejected: gcode marker '$marked' does not match expected '$expected'" }

    $stderr = Get-Content -LiteralPath $StderrPath -Raw
    $completion = @()
    foreach ($line in (Get-Content -LiteralPath $StderrPath)) {
        if ($line.TrimStart().StartsWith('{')) {
            try { $event = $line | ConvertFrom-Json } catch { throw 'measurement rejected: malformed JSON-looking stderr event' }
            if ($event.event -eq 'slice_complete') { $completion += $event }
        }
    }
    if ($completion.Count -ne 1) { throw "measurement rejected: expected exactly one slice_complete event, found $($completion.Count)" }
    $complete = $completion[0]
    $requiredCompletionProperties = @('status', 'fatal_error_count', 'non_fatal_error_count', 'degraded')
    foreach ($property in $requiredCompletionProperties) {
        if ($null -eq $complete.PSObject.Properties[$property]) { throw "measurement rejected: slice_complete is missing '$property'" }
    }
    $fatal = [int]$complete.fatal_error_count
    $nonFatal = [int]$complete.non_fatal_error_count
    $degraded = [bool]$complete.degraded
    if ($complete.status -notin @('ok', 'degraded') -or $fatal -ne 0) {
        throw "measurement rejected: completion status=$($complete.status), fatal=$fatal"
    }
    $claims = [regex]::Matches($stderr, "claim 'perimeter-generator' already held by 'com\.core\.(classic|arachne)-perimeters'")
    if ($claims.Count -ne 1) { throw "measurement rejected: expected exactly one claim-holder diagnostic, found $($claims.Count)" }
    $holder = $claims[0].Groups[1].Value.ToLowerInvariant()
    if ($holder -ne $expected) { throw "measurement rejected: stderr claim holder '$holder' does not match expected '$expected'" }
    [pscustomobject]@{
        ValidatedGenerator = $expected
        CompletionStatus = [string]$complete.status
        FatalErrorCount = $fatal
        NonFatalErrorCount = $nonFatal
        Degraded = $degraded
    }
}
