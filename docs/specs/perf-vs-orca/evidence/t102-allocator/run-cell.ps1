# t102 global-allocator substitution — standing GetProcessTimes measurement cell.
# Adapted from the t95 cell runner (itself the t78/t85/t91 protocol): one-shot
# cell guard, external GetProcessTimes wall+CPU, measure-phase guard to the
# approved timing cells, uninstrumented/isolation checks, and a per-cell output
# check against references.json.
#
# proof   : one run per arm, byte identity required, ordinary cells also
#           compared to the post-t81 reference set (t91's references.json, which
#           no production change has invalidated since t91).
# pregate : 1 warmup + 3 alternating pairs on ordinary base classic-on, the
#           ticket's pre-declared stop screen before the four-cell campaign.
# measure : 2 warmups + 6 alternating pairs. The pre-declared timing set is the
#           four base supports-on cells: {ordinary, accelerated} x
#           {classic, arachne}.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ordinary','accelerated')][string]$Mode,
    [Parameter(Mandatory)][ValidateSet('benchy','base')][string]$Fixture,
    [Parameter(Mandatory)][ValidateSet('classic','arachne')][string]$Generator,
    [Parameter(Mandatory)][ValidateSet('off','on')][string]$Supports,
    [Parameter(Mandatory)][ValidateSet('proof','pregate','measure')][string]$Phase
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t102-allocator'
$Cell = "$Mode-$Fixture-$Generator-$Supports"
$RunDir = Join-Path $Root "$Phase\$Cell"
if (Test-Path $RunDir) { throw "one-shot guard: $RunDir exists" }
if ($Phase -ne 'proof') {
    if ($Fixture -ne 'base' -or $Supports -ne 'on') { throw 'unapproved timing job' }
}
if ($Phase -eq 'pregate') {
    if ($Mode -ne 'ordinary' -or $Generator -ne 'classic') { throw 'pregate is ordinary base classic-on only' }
}
if ($Phase -eq 'measure') {
    $q = Get-Content (Join-Path $PSScriptRoot 'qualification.json') -Raw | ConvertFrom-Json
    if (-not $q.pass -or $q.cells.Count -ne 16) { throw 'full qualification gate absent' }
}
$Config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$Generator-supports-$Supports.json"
$Model = Join-Path $Repo $(if ($Fixture -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
$ReferenceSha = $null
$ReferenceBytes = $null
if ($Phase -eq 'proof' -and $Mode -eq 'ordinary') {
    $refs = Get-Content (Join-Path $PSScriptRoot '..\t91-exact-z-memo\references.json') -Raw | ConvertFrom-Json
    $ref = $refs.cells.$Cell
    if ($null -eq $ref) { throw "no derived reference for $Cell" }
    $ReferenceSha = $ref.sha256
    $ReferenceBytes = [int]$ref.bytes
}
if ($Phase -ne 'proof') {
    $proofRows = Join-Path $Root "proof\$Cell\rows.csv"
    if (-not (Test-Path $proofRows)) { throw "proof rows absent for $Cell" }
    $proof = @(Import-Csv $proofRows)
    if ($proof.Count -ne 2 -or $proof[0].sha256 -ne $proof[1].sha256) { throw 'invalid proof' }
    $ReferenceSha = $proof[0].sha256
    $ReferenceBytes = [int]$proof[0].bytes
}
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $RunDir | Out-Null
$Csv = Join-Path $RunDir 'rows.csv'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T102ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Invoke-Arm([string]$Arm,[int]$Index,[bool]$Warmup) {
    $snapshot = Join-Path $Root "snapshots\$Mode-$Arm"
    $exe = Join-Path $snapshot 'pnp_cli.exe'
    $modules = Join-Path $snapshot 'modules'
    $id = "$Cell-$Arm-$(if ($Warmup) { 'w' } else { 'm' })$Index"
    $out = Join-Path $RunDir "$id.gcode"
    $err = Join-Path $RunDir "$id.stderr.jsonl"
    $stdout = Join-Path $RunDir "$id.stdout.log"
    $slicerArgs = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$Config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',('"'+$modules+'"'),
        '--no-default-module-paths','--no-integrated-modules')
    $p = Start-Process -FilePath $exe -ArgumentList $slicerArgs -PassThru -NoNewWindow `
        -Environment @{RAYON_NUM_THREADS='12'} -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T102ProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) { throw 'GetProcessTimes failed' }
    $wall = ([T102ProcessTimes]::T($e)-[T102ProcessTimes]::T($c))/10000000.0
    $cpu = ([T102ProcessTimes]::T($k)+[T102ProcessTimes]::T($u))/10000000.0
    $valid = Test-MeasurementEvidence -ConfigPath $Config -OutputPath $out -StderrPath $err -ExpectedGenerator $Generator -ExitCode $p.ExitCode
    if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output: $id" }
    $warnings = @{}
    foreach ($line in [IO.File]::ReadLines($err)) {
        if ($line.Contains('shadows integrated') -or $line.Contains('PERF-T') -or $line -match '"event"\s*:\s*"(module_complete|stage_complete|profile_summary)"') { throw "probe/isolation failure: $id" }
        if ($line.Contains(' WARN ')) {
            $key = if ($line.Contains('is not closed')) { 'open_loop' } elseif ($line.Contains("claim 'perimeter-generator'")) { 'perimeter_claim' } elseif ($line.Contains("claim 'layer-infill'")) { 'infill_claim' } else { $line -replace '^.*? WARN ','' }
            if (-not $warnings.ContainsKey($key)) { $warnings[$key]=0 }
            $warnings[$key]++
        }
    }
    $sha = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
    $bytes = (Get-Item $out).Length
    if ($null -ne $ReferenceSha -and $sha -ne $ReferenceSha) { throw "output drift: $id (got $sha want $ReferenceSha)" }
    if ($null -ne $ReferenceBytes -and $bytes -ne $ReferenceBytes) { throw "output size drift: $id (got $bytes want $ReferenceBytes)" }
    $types = [ordered]@{}
    foreach ($line in [IO.File]::ReadLines($out)) {
        if ($line.StartsWith(';TYPE:')) {
            $key=$line.Substring(6)
            if (-not $types.Contains($key)) { $types[$key]=0 }
            $types[$key]++
        }
    }
    if (-not $types.Contains('Outer wall') -or -not $types.Contains('Sparse infill')) { throw 'vacuous model roles' }
    if ($Supports -eq 'on' -and (-not $types.Contains('Support') -or -not $types.Contains('Support interface'))) { throw 'vacuous support roles' }
    [pscustomobject][ordered]@{
        run_id=$id; phase=$Phase; mode=$Mode; fixture=$Fixture; generator=$Generator; supports=$Supports
        arm=$Arm; pair=$Index; warmup=[int]$Warmup; threads=12; exit_code=$p.ExitCode
        exe_sha256=(Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
        wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=$bytes; sha256=$sha; type_counts=($types|ConvertTo-Json -Compress)
        warnings=($warnings|ConvertTo-Json -Compress); status=$valid.CompletionStatus
        degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $p.Dispose()
    Write-Host ("[t102] {0} wall={1:N3}s cpu={2:N3}s sha={3}" -f $id,$wall,$cpu,$sha)
}
if ($Phase -eq 'proof') {
    foreach ($arm in @('baseline','candidate')) { Invoke-Arm $arm 0 $true }
} elseif ($Phase -eq 'pregate') {
    foreach ($arm in @('baseline','candidate')) { Invoke-Arm $arm 0 $true }
    $w = @(Import-Csv $Csv)
    if ($w[0].sha256 -ne $w[1].sha256) { throw 'baseline/candidate output mismatch' }
    for ($i=1;$i -le 3;$i++) {
        $order = if ($i%2 -eq 1) { @('baseline','candidate') } else { @('candidate','baseline') }
        foreach ($arm in $order) { Invoke-Arm $arm $i $false }
    }
} else {
    for ($i=0;$i -lt 2;$i++) { foreach ($arm in @('baseline','candidate')) { Invoke-Arm $arm $i $true } }
    $w = @(Import-Csv $Csv)
    $warmShas = @($w | ForEach-Object { $_.sha256 } | Select-Object -Unique)
    if ($warmShas.Count -ne 1) { throw 'warmup output mismatch' }
    for ($i=1;$i -le 6;$i++) {
        $order = if ($i%2 -eq 1) { @('baseline','candidate') } else { @('candidate','baseline') }
        foreach ($arm in $order) { Invoke-Arm $arm $i $false }
    }
}
