# t89 pre-gate: one warmup pair + N alternating baseline/candidate pairs on the
# ordinary base classic supports-on cell. This is a STOP-DISCIPLINE check, not
# the standing gate: it decides whether the full 4-cell campaign is worth
# running when the candidate's process-CPU delta is already suspicious.
[CmdletBinding()]
param([int]$Pairs = 3)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t89-exact-z-parallel'
$RunDir = Join-Path $Root 'pregate'
New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
$Csv = Join-Path $RunDir 'rows.csv'
if (Test-Path $Csv) { throw "one-shot guard: $Csv exists" }
$Mode = 'ordinary'
$Config = Join-Path $PSScriptRoot '..\matched-pair\configs\pnp-classic-supports-on.json'
$Model = Join-Path $Repo 'tmp\base.stl'
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T89PregateTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Invoke-Arm([string]$Arm, [int]$Index, [bool]$Warmup) {
    $snapshot = Join-Path $Root "snapshots\$Mode-$Arm"
    $id = "pregate-$Arm-$(if ($Warmup) { 'w' } else { 'm' })$Index"
    $out = Join-Path $RunDir "$id.gcode"
    $err = Join-Path $RunDir "$id.stderr.jsonl"
    $stdout = Join-Path $RunDir "$id.stdout.log"
    $slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',(Join-Path $snapshot 'modules'),
        '--no-default-module-paths','--no-integrated-modules')
    $p = Start-Process -FilePath (Join-Path $snapshot 'pnp_cli.exe') -ArgumentList $slicerArgs `
        -PassThru -NoNewWindow -Environment @{RAYON_NUM_THREADS='12'} `
        -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T89PregateTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
    $wall = ([T89PregateTimes]::T($e)-[T89PregateTimes]::T($c))/10000000.0
    $cpu = ([T89PregateTimes]::T($k)+[T89PregateTimes]::T($u))/10000000.0
    if ($p.ExitCode -ne 0) { throw "exit $($p.ExitCode): $id" }
    $sha = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
    [pscustomobject][ordered]@{
        arm=$Arm; pair=$Index; warmup=[int]$Warmup; wall_seconds=$wall; cpu_seconds=$cpu
        cpu_wall_ratio=($cpu/$wall); sha256=$sha; bytes=(Get-Item $out).Length
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    Write-Host ("[t89-pregate] {0} wall={1:N3}s cpu={2:N3}s ratio={3:N2} sha={4}" -f $id,$wall,$cpu,($cpu/$wall),$sha)
    $p.Dispose()
}
foreach ($arm in @('baseline','candidate')) { Invoke-Arm $arm 0 $true }
$warm = @(Import-Csv $Csv)
if ($warm[0].sha256 -ne $warm[1].sha256) { throw 'warmup output mismatch between arms' }
for ($i = 1; $i -le $Pairs; $i++) {
    $order = if ($i % 2 -eq 1) { @('baseline','candidate') } else { @('candidate','baseline') }
    foreach ($arm in $order) { Invoke-Arm $arm $i $false }
}
Write-Host '[t89-pregate] complete'
