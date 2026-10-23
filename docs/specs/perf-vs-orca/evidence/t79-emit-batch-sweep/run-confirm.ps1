# t79 pre-gate confirmation: two alternating uninstrumented baseline/candidate
# pairs on the screening cell, using the frozen snapshots. This is a direction
# check only; the standing 6-pair campaign is the decision.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ordinary', 'accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('benchy', 'base')][string]$Fixture
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t79-emit-batch-sweep'
$Cell = "$Mode-$Fixture-classic-on"
$RunDir = Join-Path $Root "confirm\$Cell"
if (Test-Path $RunDir) { throw "one-shot guard: $RunDir exists" }
New-Item -ItemType Directory -Path $RunDir | Out-Null
$Csv = Join-Path $RunDir 'rows.csv'
$Config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-classic-supports-on.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
$References = Get-Content (Join-Path $PSScriptRoot '..\t58-finalization-copies\qualification.json') -Raw | ConvertFrom-Json
$Reference = $References.cells | Where-Object { $_.cell -eq $Cell }
if ($null -eq $Reference) { throw "no preserved t58 reference for $Cell" }
$ExpectedSha = $Reference.sha256
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T79ConfirmTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Invoke-Arm([string]$Arm, [int]$Pair, [bool]$Warmup) {
    $snapshot = Join-Path $Root "snapshots\$Mode-$Arm"
    $exe = Join-Path $snapshot 'pnp_cli.exe'
    $modules = Join-Path $snapshot 'modules'
    $id = "$Cell-$Arm-$(if ($Warmup) { 'w' } else { 'm' })$Pair"
    $out = Join-Path $RunDir "$id.gcode"
    $err = Join-Path $RunDir "$id.stderr.jsonl"
    $stdout = Join-Path $RunDir "$id.stdout.log"
    $args = @('slice', '--model', ('"'+$Model+'"'), '--config', ('"'+$Config+'"'),
        '--output', ('"'+$out+'"'), '--module-dir', ('"'+$modules+'"'),
        '--no-default-module-paths', '--no-integrated-modules')
    $p = Start-Process -FilePath $exe -ArgumentList $args -PassThru -NoNewWindow `
        -Environment @{RAYON_NUM_THREADS='12'} -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T79ConfirmTimes]::GetProcessTimes($handle, [ref]$c, [ref]$e, [ref]$k, [ref]$u)) { throw 'GetProcessTimes failed' }
    $wall = ([T79ConfirmTimes]::T($e) - [T79ConfirmTimes]::T($c)) / 10000000.0
    $cpu = ([T79ConfirmTimes]::T($k) + [T79ConfirmTimes]::T($u)) / 10000000.0
    $valid = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $out -StderrPath $err -ExpectedGenerator 'classic' -ExitCode $p.ExitCode
    if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output: $id" }
    $sha = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
    if ($sha -ne $ExpectedSha) { throw "output drift: $id" }
    [pscustomobject][ordered]@{
        run_id=$id; mode=$Mode; fixture=$Fixture; arm=$Arm; pair=$Pair; warmup=[int]$Warmup
        wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=(Get-Item $out).Length; sha256=$sha; status=$valid.CompletionStatus
        degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    Write-Host ("[t79-confirm] {0} wall={1:N3}s cpu={2:N3}s" -f $id, $wall, $cpu)
    $p.Dispose()
}
foreach ($arm in @('baseline', 'candidate')) { Invoke-Arm $arm 0 $true }
for ($i = 1; $i -le 2; $i++) {
    $order = if ($i % 2 -eq 1) { @('baseline', 'candidate') } else { @('candidate', 'baseline') }
    foreach ($arm in $order) { Invoke-Arm $arm $i $false }
}
Write-Host '[t79-confirm] done'
