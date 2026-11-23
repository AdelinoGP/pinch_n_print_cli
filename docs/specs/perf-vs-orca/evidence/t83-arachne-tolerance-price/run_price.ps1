# Arachne wall-simplification tolerance pricing harness (perf-vs-orca ticket 83).
#
# Two arms on ONE binary and ONE module tree, differing only in config:
#   default   = the committed matched Arachne config (both tolerance keys unset
#               -> code fallback 0.05 mm / 0.005 mm)
#   canonical = the same job plus wall_maximum_resolution = 0.5 mm and
#               wall_maximum_deviation = 0.025 mm (Orca's resolved values)
#
# Pre-declared gate: docs/specs/perf-vs-orca/issues/83-*.md, "Pre-declared
# pricing gate". Instrumented runs are attribution only; they are never a wall
# claim.
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t83-arachne-tolerance-price/run_price.ps1 `
#       -Fixtures benchy,base -Repeats 2 -BatchId t83-price-1

[CmdletBinding()]
param(
    [ValidateSet('benchy', 'base')][string[]]$Fixtures = @('benchy', 'base'),
    [ValidateSet('off', 'on')][string[]]$Supports = @('off', 'on'),
    [int]$Repeats = 2,
    [int]$Threads = 12,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RawRoot = 'target\t83-price',
    [switch]$SkipFreshness,
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$FixturePaths = @{
    benchy = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
    base   = Join-Path $RepoRoot 'tmp\base.stl'
}
$DefaultConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$CanonicalConfigDir = Join-Path $PSScriptRoot 'configs'
$Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
$Modules = Join-Path $RepoRoot 'modules\core-modules'

if (-not [IO.Path]::IsPathRooted($RawRoot)) { $RawRoot = Join-Path $RepoRoot $RawRoot }
$RunDir = Join-Path $RawRoot $BatchId
foreach ($d in @($RunDir, (Join-Path $RunDir 'gcode'), (Join-Path $RunDir 'stderr'))) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}
$Csv = Join-Path $RunDir 'rows.csv'

$Arms = [ordered]@{
    'default'   = @{ ConfigDir = $DefaultConfigDir; Suffix = '' }
    'canonical' = @{ ConfigDir = $CanonicalConfigDir; Suffix = '-canonical' }
}

foreach ($p in @($FixturePaths['benchy'], $FixturePaths['base'], $Exe, $Modules)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing prerequisite: $p" }
}

. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue

if (-not ('T83ProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T83ProcessTimes {
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
        $commit = (& git rev-parse HEAD).Trim()
        $inputs = [ordered]@{
            captured_commit = $commit
            exe_sha256      = (Get-Hash $Exe)
            threads         = $Threads
            guest_freshness_exit = 0
            models          = [ordered]@{
                benchy = (Get-Hash $FixturePaths['benchy'])
                base   = (Get-Hash $FixturePaths['base'])
            }
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
$inputs = Get-Content -LiteralPath $inputsPath -Raw | ConvertFrom-Json

$runIndex = 0
foreach ($fixture in $Fixtures) {
    foreach ($supportsState in $Supports) {
        $cell = "arachne-supports-$supportsState"
        $cellDir = Join-Path $RunDir "$fixture-$cell"
        if (-not (Test-Path $cellDir)) { New-Item -ItemType Directory -Path $cellDir -Force | Out-Null }

        # Alternating pairs: repeat 0 runs default->canonical, repeat 1 runs
        # canonical->default, so order and drift cancel across the pair set.
        foreach ($repeat in 0..($Repeats - 1)) {
            $order = if ($repeat % 2 -eq 0) { @('default', 'canonical') } else { @('canonical', 'default') }
            foreach ($arm in $order) {
                $spec = $Arms[$arm]
                $config = Join-Path $spec.ConfigDir "pnp-$cell$($spec.Suffix).json"
                if (-not (Test-Path $config)) { throw "missing config: $config" }
                $runId = '{0}-{1}-{2}-r{3}' -f $fixture, $cell, $arm, ($repeat + 1)
                $out = Join-Path $cellDir "$runId.gcode"
                $err = Join-Path $cellDir "$runId.stderr.jsonl"
                $stdout = Join-Path $cellDir "$runId.stdout.log"
                Remove-Item -Force -ErrorAction SilentlyContinue $out, $err, $stdout

                $arguments = @('slice', '--model', ('"' + $FixturePaths[$fixture] + '"'),
                    '--config', ('"' + $config + '"'), '--output', ('"' + $out + '"'),
                    '--module-dir', ('"' + $Modules + '"'), '--no-default-module-paths',
                    '--no-integrated-modules', '--instrument-stderr',
                    '--profile', '--profile-verbose')

                $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
                    -WorkingDirectory $RepoRoot -Environment @{ RAYON_NUM_THREADS = "$Threads" } `
                    -RedirectStandardError $err -RedirectStandardOutput $stdout
                $handle = $p.Handle
                if (-not $p.WaitForExit(3600000)) { $p.Kill(); throw "run timeout: $runId" }
                $p.Refresh()

                $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
                $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
                $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
                $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
                if (-not [T83ProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) {
                    throw 'GetProcessTimes failed'
                }
                $wall = ([T83ProcessTimes]::T($e) - [T83ProcessTimes]::T($c)) / 10000000.0
                $cpu = ([T83ProcessTimes]::T($k) + [T83ProcessTimes]::T($u)) / 10000000.0

                $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err `
                    -ExpectedGenerator 'arachne' -ExitCode $p.ExitCode

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
                    cell            = $cell
                    arm             = $arm
                    repeat          = ($repeat + 1)
                    order_index     = $runIndex
                    threads         = $Threads
                    exit_code       = $p.ExitCode
                    wall_seconds    = $wall
                    cpu_seconds     = $cpu
                    cpu_wall_ratio  = ($cpu / $wall)
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

                Write-Host ('[t83] {0}: wall={1:N1}s cpu={2:N1}s ratio={3:N2} bytes={4}' -f `
                    $runId, $wall, $cpu, ($cpu / $wall), (Get-Item $out).Length)
                $p.Dispose()
                $runIndex++
            }
        }
    }
    if ((Get-Hash $Exe) -ne $inputs.exe_sha256) { throw 'binary changed during batch' }
    foreach ($module in $inputs.modules) {
        if ((Get-Hash (Join-Path $RepoRoot $module.path)) -ne $module.sha256) {
            throw "module changed during batch: $($module.path)"
        }
    }
}
Write-Host "[t83] batch complete: $Csv"
