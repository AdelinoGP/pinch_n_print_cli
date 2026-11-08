# t78 local sanity run: plain uninstrumented slice with external GetProcessTimes
# wall + CPU, same measurement mechanics as the standing run-cell.ps1.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Model,
    [Parameter(Mandatory)][string]$Config,
    [Parameter(Mandatory)][string]$Out,
    [Parameter(Mandatory)][string]$Err
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Repo
$exe = Join-Path $Repo 'target\release\pnp_cli.exe'
$env:RAYON_NUM_THREADS = '12'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T78ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
$args = @(
    'slice', '--model', $Model, '--config', $Config, '--output', $Out,
    '--module-dir', (Join-Path $Repo 'modules\core-modules'),
    '--no-default-module-paths', '--no-integrated-modules'
)
$p = Start-Process -FilePath $exe -ArgumentList $args -PassThru -NoNewWindow `
    -RedirectStandardError $Err -RedirectStandardOutput ($Out + '.stdout.log')
$handle = $p.Handle
if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw 'timeout' }
$p.Refresh()
$c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
$u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
if (-not [T78ProcessTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) { throw 'GetProcessTimes failed' }
$wall = ([T78ProcessTimes]::T($e) - [T78ProcessTimes]::T($c)) / 10000000.0
$cpu = ([T78ProcessTimes]::T($k) + [T78ProcessTimes]::T($u)) / 10000000.0
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
[pscustomobject][ordered]@{
    exit_code = $p.ExitCode
    wall_seconds = [math]::Round($wall, 3)
    cpu_seconds = [math]::Round($cpu, 3)
    cpu_wall_ratio = [math]::Round($cpu / $wall, 3)
    bytes = (Get-Item $Out).Length
    sha256 = $sha
} | ConvertTo-Json -Compress
$p.Dispose()
