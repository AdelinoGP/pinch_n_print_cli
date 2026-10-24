# Uninstrumented wall A/B for the two borderline Arachne cells (ticket 83).
#
# Same two config arms as run_price.ps1 (default vs canonical tolerances), one
# binary and one module tree, but no `--profile` / `--instrument-stderr`, so
# wall and CPU are clean process measurements. Alternating order per pair.
#
# Usage:
#   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t83-arachne-tolerance-price/run_wall_pairs.ps1 `
#       -Cells 'benchy-arachne-supports-on,base-arachne-supports-off' -Pairs 6 -BatchId wall-1

[CmdletBinding()]
param(
    [string[]]$Cells = @('benchy-arachne-supports-on', 'base-arachne-supports-off'),
    [int]$Pairs = 6,
    [int]$Warmups = 1,
    [int]$Threads = 12,
    [string]$BatchId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$RawRoot = 'target\t83-price',
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
foreach ($d in @($RunDir, (Join-Path $RunDir 'gcode'))) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}
$Csv = Join-Path $RunDir 'rows.csv'

foreach ($p in @($FixturePaths['benchy'], $FixturePaths['base'], $Exe, $Modules)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing prerequisite: $p" }
}
. (Join-Path $RepoRoot 'resources\perimeter-acceptance\validate_measurement.ps1')
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue

if (-not ('T83WallProcessTimes' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T83WallProcessTimes {
  [DllImport("kernel32.dll",SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
  public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
}

function Get-Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

$arms = @{
    'default'   = @{ ConfigDir = $DefaultConfigDir;   Suffix = '' }
    'canonical' = @{ ConfigDir = $CanonicalConfigDir; Suffix = '-canonical' }
}

$runIndex = 0
foreach ($cellSpec in $Cells) {
    $fixture, $cell = $cellSpec -split '-', 2
    foreach ($arm in @('default', 'canonical')) {
        $config = Join-Path $arms[$arm].ConfigDir "pnp-$cell$($arms[$arm].Suffix).json"
        if (-not (Test-Path $config)) { throw "missing config: $config" }
        $warm = Join-Path $RunDir "$cellSpec-$arm-warmup.gcode"
        if ($Warmups -gt 0) {
            $p = Start-Process -FilePath $Exe -ArgumentList @('slice', '--model', ('"' + $FixturePaths[$fixture] + '"'),
                '--config', ('"' + $config + '"'), '--output', ('"' + $warm + '"'),
                '--module-dir', ('"' + $Modules + '"'), '--no-default-module-paths', '--no-integrated-modules') `
                -PassThru -NoNewWindow -WorkingDirectory $RepoRoot -Environment @{ RAYON_NUM_THREADS = "$Threads" } `
                -RedirectStandardOutput (Join-Path $RunDir "$cellSpec-$arm-warmup.stdout.log") `
                -RedirectStandardError (Join-Path $RunDir "$cellSpec-$arm-warmup.stderr.log")
            if (-not $p.WaitForExit(3600000)) { $p.Kill(); throw "warmup timeout: $cellSpec/$arm" }
            if ($p.ExitCode -ne 0) { throw "warmup failed: $cellSpec/$arm" }
            $p.Dispose()
        }
    }
    foreach ($pair in 1..$Pairs) {
        $order = if ($pair % 2 -eq 1) { @('default', 'canonical') } else { @('canonical', 'default') }
        foreach ($arm in $order) {
            $config = Join-Path $arms[$arm].ConfigDir "pnp-$cell$($arms[$arm].Suffix).json"
            $runId = '{0}-{1}-{2}-p{3}' -f $fixture, $cell, $arm, $pair
            $out = Join-Path $RunDir "$cellSpec-$arm-p$pair.gcode"
            $stdout = Join-Path $RunDir "$cellSpec-$arm-p$pair.stdout.log"
            $stderr = Join-Path $RunDir "$cellSpec-$arm-p$pair.stderr.log"
            Remove-Item -Force -ErrorAction SilentlyContinue $out, $stdout, $stderr
            $p = Start-Process -FilePath $Exe -ArgumentList @('slice', '--model', ('"' + $FixturePaths[$fixture] + '"'),
                '--config', ('"' + $config + '"'), '--output', ('"' + $out + '"'),
                '--module-dir', ('"' + $Modules + '"'), '--no-default-module-paths', '--no-integrated-modules') `
                -PassThru -NoNewWindow -WorkingDirectory $RepoRoot -Environment @{ RAYON_NUM_THREADS = "$Threads" } `
                -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            $handle = $p.Handle
            if (-not $p.WaitForExit(3600000)) { $p.Kill(); throw "run timeout: $runId" }
            $p.Refresh()
            $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
            $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
            $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
            $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
            if (-not [T83WallProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) {
                throw 'GetProcessTimes failed'
            }
            $wall = ([T83WallProcessTimes]::T($e) - [T83WallProcessTimes]::T($c)) / 10000000.0
            $cpu = ([T83WallProcessTimes]::T($k) + [T83WallProcessTimes]::T($u)) / 10000000.0
            $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $stderr `
                -ExpectedGenerator 'arachne' -ExitCode $p.ExitCode
            [pscustomobject][ordered]@{
                run_id = $runId; fixture = $fixture; cell = $cell; arm = $arm; pair = $pair
                order_index = $runIndex; threads = $Threads; exit_code = $p.ExitCode
                wall_seconds = $wall; cpu_seconds = $cpu; cpu_wall_ratio = ($cpu / $wall)
                bytes = (Get-Item $out).Length; sha256 = (Get-Hash $out)
                status = $valid.CompletionStatus; degraded = $valid.Degraded
                fatal = $valid.FatalErrorCount; non_fatal = $valid.NonFatalErrorCount
                config = [IO.Path]::GetRelativePath($RepoRoot, $config).Replace('\', '/')
            } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
            Write-Host ('[t83-wall] {0}: wall={1:N2}s cpu={2:N1}s ratio={3:N2}' -f $runId, $wall, $cpu, ($cpu / $wall))
            $p.Dispose()
            $runIndex++
        }
    }
}
Write-Host "[t83-wall] complete: $Csv"
