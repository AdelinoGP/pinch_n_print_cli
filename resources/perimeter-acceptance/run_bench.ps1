<#
.SYNOPSIS
    External benchmark harness for a pnp slicing binary. Measures wall time,
    CPU time, and peak working set by polling the process, and appends one CSV
    row per run. Self-contained; uses only pwsh builtins.

.NOTES
    The pnp slice CLI uses --model (not --input) for the input model path.
    This harness maps the -InputModel parameter to the --model flag.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][string]$InputModel,
    [Parameter(Mandatory = $true)][string]$Config,
    [Parameter(Mandatory = $true)][string]$ModuleDir,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][int]$Threads,
    [switch]$Warmup,
    [Parameter(Mandatory = $true)][string]$Label,
    [int]$Runs = 1,
    [Parameter(Mandatory = $true)][string]$ResultsPath,
    [switch]$Instrumented,
    [int]$PeakSampleMs = 100,
    [ValidateSet('classic', 'arachne')][string]$ExpectedGenerator,
    [switch]$Profile
)

$ErrorActionPreference = 'Stop'

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

function ConvertTo-CsvField {
    param([string]$Value)
    if ($Value -match '[",\r\n]') {
        return '"' + ($Value -replace '"', '""') + '"'
    }
    return $Value
}

# --- Ensure parent directories exist -------------------------------------
$resultsDir = Split-Path -Parent $ResultsPath
if ($resultsDir -and -not (Test-Path $resultsDir)) {
    New-Item -ItemType Directory -Path $resultsDir -Force | Out-Null
}
$outDir = Split-Path -Parent $OutputPath
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

# --- CSV header (create only if file does not exist) ---------------------
$legacyHeader = 'label,model,threads,run_index,warmup,wall_seconds,cpu_seconds,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,exit_code,timestamp_utc'
$header = 'label,model,threads,run_index,warmup,expected_generator,validated_generator,config_path,exe_path,module_dir,stderr_path,output_path,wall_seconds,cpu_seconds,cpu_wall_ratio,peak_workingset_bytes,gcode_bytes,sha256_of_gcode,exit_code,completion_status,fatal_error_count,non_fatal_error_count,degraded,timestamp_utc'
if (-not (Test-Path $ResultsPath)) {
    Set-Content -Path $ResultsPath -Value $(if ($ExpectedGenerator) { $header } else { $legacyHeader }) -Encoding utf8
} else {
    $actualHeader = (Get-Content -LiteralPath $ResultsPath -TotalCount 1)
    $wantedHeader = if ($ExpectedGenerator) { $header } else { $legacyHeader }
    if ($actualHeader -ne $wantedHeader) { throw "results CSV header mismatch; expected '$wantedHeader'" }
}

$jsonlPath = Join-Path $resultsDir "$Label.jsonl"
$validatorPath = Join-Path $PSScriptRoot 'validate_measurement.ps1'
if ($ExpectedGenerator) {
    . $validatorPath
}

for ($run = 1; $run -le $Runs; $run++) {
    $runOutputPath = if ($Runs -gt 1) { "$OutputPath.run$run.gcode" } else { $OutputPath }
    $runJsonlPath = if ($Runs -gt 1) { "$jsonlPath.run$run" } else { $jsonlPath }
    Remove-Item -Force -ErrorAction SilentlyContinue $runOutputPath, $runJsonlPath
    # --- Build argument list (quote all paths) ---------------------------
    $argList = @(
        'slice',
        '--model', ('"' + $InputModel + '"'),
        '--output', ('"' + $runOutputPath + '"'),
        '--config', ('"' + $Config + '"'),
        '--module-dir', ('"' + $ModuleDir + '"')
    )
    if ($Instrumented -or $Profile) { $argList += '--instrument-stderr' }
    if ($Profile) { $argList += '--profile' }

    # Explicitly set RAYON_NUM_THREADS for the child process.
    $envMap = @{ RAYON_NUM_THREADS = "$Threads" }

    # Both modes redirect stderr: the default JSONL progress-event stream
    # would otherwise write to the console during timing, adding I/O noise.
    # Uninstrumented runs still get a real wall/CPU/peak-WS CSV row.
    $proc = Start-Process -FilePath $ExePath -ArgumentList $argList -PassThru -NoNewWindow -Environment $envMap -RedirectStandardError $runJsonlPath
    $processHandle = $proc.Handle

    # --- Poll for peak working set and CPU time until the process exits ---
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
    # Wall scope is the child process creation-to-exit interval, excluding
    # harness cleanup and independent of the working-set polling cadence.
    $wall = $nativeTimes.WallSeconds
    $cpu = $nativeTimes.CpuSeconds
    $exitCode = $proc.ExitCode

    $evidence = $null
    if ($ExpectedGenerator) {
        $evidence = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $runOutputPath -StderrPath $runJsonlPath -ExpectedGenerator $ExpectedGenerator -ExitCode $exitCode
    } elseif (-not ($Instrumented -or $Profile)) {
        $evidence = [pscustomobject]@{ ValidatedGenerator = ''; CompletionStatus = ''; FatalErrorCount = ''; NonFatalErrorCount = ''; Degraded = '' }
    }
    if ($Instrumented -or $Profile) {
        continue
    }

    # --- G-code size + sha256 (missing/empty file => 0 bytes, no throw) ----
    $gcodeBytes = 0
    $sha = ''
    if (Test-Path $runOutputPath) {
        $gcodeBytes = (Get-Item $runOutputPath).Length
        $sha = (Get-FileHash -Path $runOutputPath -Algorithm SHA256).Hash.ToLower()
    }

    $warmupVal = if ($Warmup) { 1 } else { 0 }
    $ts = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')

    $line = if ($ExpectedGenerator) { @(
        (ConvertTo-CsvField $Label),
        (ConvertTo-CsvField $InputModel),
        $Threads,
        $run,
        $warmupVal,
        $ExpectedGenerator.ToLowerInvariant(),
        (ConvertTo-CsvField $evidence.ValidatedGenerator),
        (ConvertTo-CsvField $Config),
        (ConvertTo-CsvField $ExePath),
        (ConvertTo-CsvField $ModuleDir),
        (ConvertTo-CsvField $runJsonlPath),
        (ConvertTo-CsvField $runOutputPath),
        ([math]::Round($wall, 4)),
        ([math]::Round($cpu, 4)),
        ([math]::Round(($cpu / $wall), 4)),
        $maxWs,
        $gcodeBytes,
        $sha,
        $exitCode,
        (ConvertTo-CsvField $evidence.CompletionStatus),
        $evidence.FatalErrorCount,
        $evidence.NonFatalErrorCount,
        $evidence.Degraded,
        $ts
    ) -join ',' } else { @(
        (ConvertTo-CsvField $Label), (ConvertTo-CsvField $InputModel), $Threads, $run, $warmupVal,
        ([math]::Round($wall, 4)), ([math]::Round($cpu, 4)), $maxWs, $gcodeBytes, $sha, $exitCode, $ts
    ) -join ',' }

    Add-Content -Path $ResultsPath -Value $line -Encoding utf8
}
