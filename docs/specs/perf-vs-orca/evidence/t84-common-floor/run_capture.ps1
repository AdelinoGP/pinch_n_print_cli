# Common-floor attribution capture harness (perf-vs-orca ticket 84).
#
# One cell, one arm, one run per capture. Records process wall/CPU, output SHA,
# TYPE counts, completion status, and the instrumented/profile stream.
# -Probe sets PNP_T84_PROBE=1 (the probe patch is inert without it).
# -Profile adds --profile --profile-verbose; its wall is inflated and is never
# a wall claim (docs/17_agent_debugging.md).
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t84-common-floor/run_capture.ps1 `
#       -Cell base-classic-off -Label head -BatchId t84-head
#   pwsh ... -Cell base-classic-off -Label probe -Probe -Profile -BatchId t84-probe

[CmdletBinding()]
param(
    [ValidateSet('base-classic-off', 'base-classic-on')][string]$Cell = 'base-classic-off',
    [string]$Label = 'head',
    [switch]$Probe,
    [switch]$Profile,
    [int]$Threads = 12,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RawRoot = 'target\t84-common-floor',
    [switch]$SkipFreshness,
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$FixturePaths = @{ benchy = (Join-Path $RepoRoot 'tmp\3dbenchy.stl'); base = (Join-Path $RepoRoot 'tmp\base.stl') }
$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
$Modules = Join-Path $RepoRoot 'modules\core-modules'

$parts = $Cell.Split('-')          # <fixture>-<generator>-<supports>
$fixture = $parts[0]
$generator = $parts[1]
$supports = $parts[2]
$config = Join-Path $ConfigDir "pnp-$generator-supports-$supports.json"

if (-not [IO.Path]::IsPathRooted($RawRoot)) { $RawRoot = Join-Path $RepoRoot $RawRoot }
$RunDir = Join-Path $RawRoot $BatchId
foreach ($d in @($RunDir, (Join-Path $RunDir 'gcode'), (Join-Path $RunDir 'stderr'), (Join-Path $RunDir 'stdout'))) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}
$Csv = Join-Path $RunDir 'rows.csv'

foreach ($p in @($FixturePaths[$fixture], $Exe, $Modules, $config)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing prerequisite: $p" }
}

. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
Remove-Item Env:PNP_T84_PROBE -ErrorAction SilentlyContinue

if (-not ('T84ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T84ProcessTimes {
  [DllImport("kernel32.dll",SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
}
function Get-Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

$inputsPath = Join-Path $RunDir 'inputs.json'
if (-not (Test-Path $inputsPath)) {
    Push-Location $RepoRoot
    try {
        if (-not $SkipFreshness) {
            & cargo xtask build-guests --check *> (Join-Path $RunDir 'freshness.log')
            if ($LASTEXITCODE -ne 0) { throw "guest freshness exit $LASTEXITCODE (see freshness.log)" }
        }
        & $Exe module diagnose --module-dir $Modules --no-default-module-paths --no-integrated-modules `
            1> (Join-Path $RunDir 'diagnose.json') 2> (Join-Path $RunDir 'diagnose.stderr.log')
        if ($LASTEXITCODE -ne 0) { throw "module diagnose exit $LASTEXITCODE" }
        $diagnose = Get-Content (Join-Path $RunDir 'diagnose.json') -Raw | ConvertFrom-Json
        if (-not $diagnose.pass -or @($diagnose.modules).Count -eq 0 -or
            @($diagnose.modules | Where-Object provenance -ne 'external').Count -ne 0) {
            throw 'external-module provenance failed'
        }
        $inputs = [ordered]@{
            captured_commit = (& git rev-parse HEAD).Trim()
            exe_sha256      = (Get-Hash $Exe)
            threads         = $Threads
            guest_freshness_exit = 0
            models          = [ordered]@{ benchy = (Get-Hash $FixturePaths['benchy']); base = (Get-Hash $FixturePaths['base']) }
            provenance      = $diagnose.modules
            modules         = @(Get-ChildItem $Modules -Recurse -File |
                Where-Object Extension -in @('.wasm', '.toml') | Sort-Object FullName | ForEach-Object {
                    [pscustomobject]@{
                        path   = [IO.Path]::GetRelativePath($RepoRoot, $_.FullName).Replace('\', '/')
                        sha256 = (Get-Hash $_.FullName)
                    }
                })
        }
        $inputs | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $inputsPath -Encoding utf8
    } finally { Pop-Location }
}

$runId = "$Cell-$Label"
$out = Join-Path $RunDir "gcode\$runId.gcode"
$err = Join-Path $RunDir "stderr\$runId.jsonl"
$stdout = Join-Path $RunDir "stdout\$runId.log"
Remove-Item -Force -ErrorAction SilentlyContinue $out, $err, $stdout

$arguments = @('slice', '--model', ('"' + $FixturePaths[$fixture] + '"'),
    '--config', ('"' + $config + '"'), '--output', ('"' + $out + '"'),
    '--module-dir', ('"' + $Modules + '"'), '--no-default-module-paths',
    '--no-integrated-modules', '--instrument-stderr')
if ($Profile) { $arguments += @('--profile', '--profile-verbose') }

$envOverrides = @{ RAYON_NUM_THREADS = "$Threads" }
if ($Probe) { $envOverrides['PNP_T84_PROBE'] = '1' }

$p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
    -WorkingDirectory $RepoRoot -Environment $envOverrides `
    -RedirectStandardError $err -RedirectStandardOutput $stdout
$handle = $p.Handle
if (-not $p.WaitForExit(3600000)) { $p.Kill(); throw "run timeout: $runId" }
$p.Refresh()

$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T84ProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T84ProcessTimes]::T($e) - [T84ProcessTimes]::T($c)) / 10000000.0
$cpu = ([T84ProcessTimes]::T($k) + [T84ProcessTimes]::T($u)) / 10000000.0

$valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err `
    -ExpectedGenerator $generator -ExitCode $p.ExitCode

$types = [ordered]@{}
foreach ($line in [IO.File]::ReadLines($out)) {
    if ($line.StartsWith(';TYPE:')) {
        $key = $line.Substring(6)
        if (-not $types.Contains($key)) { $types[$key] = 0 }
        $types[$key]++
    }
}
if (-not $types.Contains('Outer wall') -or -not $types.Contains('Sparse infill')) {
    throw "vacuous output roles: $runId"
}

[pscustomobject][ordered]@{
    run_id          = $runId
    fixture         = $fixture
    cell            = "$generator-supports-$supports"
    label           = $Label
    probe           = [bool]$Probe
    profile         = [bool]$Profile
    threads         = $Threads
    exit_code       = $p.ExitCode
    wall_seconds    = [math]::Round($wall, 3)
    cpu_seconds     = [math]::Round($cpu, 3)
    cpu_wall_ratio  = [math]::Round(($cpu / $wall), 4)
    bytes           = (Get-Item $out).Length
    sha256          = (Get-Hash $out)
    type_counts     = ($types | ConvertTo-Json -Compress)
    status          = $valid.CompletionStatus
    degraded        = $valid.Degraded
    fatal           = $valid.FatalErrorCount
    non_fatal       = $valid.NonFatalErrorCount
    config          = [IO.Path]::GetRelativePath($RepoRoot, $config).Replace('\', '/')
    config_sha256   = (Get-Hash $config)
    stderr_sha256   = (Get-Hash $err)
    exe_sha256      = (Get-Hash $Exe)
} | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8

Write-Host ('[t84] {0}: wall={1:N1}s cpu={2:N1}s ratio={3:N2} bytes={4} sha={5}' -f `
    $runId, $wall, $cpu, ($cpu / $wall), (Get-Item $out).Length, (Get-Hash $out))
$p.Dispose()
