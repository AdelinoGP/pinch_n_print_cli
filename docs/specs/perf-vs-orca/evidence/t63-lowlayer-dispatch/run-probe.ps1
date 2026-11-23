# t63 low-layer dispatch probe captures - DIAGNOSTIC ONLY.
# Attribution-only worker-identity captures for wayfinder ticket 63.
# Not an uninstrumented timing campaign; not a keep/drop gate; no wall claims.
[CmdletBinding()]
param([string]$RawRoot = 'target\t63-lowlayer-dispatch')
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
if (-not [IO.Path]::IsPathRooted($RawRoot)) { $RawRoot = Join-Path $Repo $RawRoot }
if (Test-Path $RawRoot) { throw "one-shot guard: $RawRoot exists" }
New-Item -ItemType Directory -Path $RawRoot | Out-Null
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Exe = Join-Path $Repo 'target\release\pnp_cli.exe'
$Modules = Join-Path $Repo 'modules\core-modules'
$Model = Join-Path $Repo 'tmp\3dbenchy.stl'
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue

Push-Location $Repo
try {
    & cargo xtask build-guests --check *> (Join-Path $RawRoot 'freshness.log')
    if ($LASTEXITCODE -ne 0) { throw "guest freshness exit $LASTEXITCODE" }
    & $Exe module diagnose --module-dir $Modules --no-default-module-paths --no-integrated-modules `
        1> (Join-Path $RawRoot 'diagnose.json') 2> (Join-Path $RawRoot 'diagnose.stderr.log')
    if ($LASTEXITCODE -ne 0) { throw "module diagnose exit $LASTEXITCODE" }
    $diagnose = Get-Content (Join-Path $RawRoot 'diagnose.json') -Raw | ConvertFrom-Json
    if (-not $diagnose.pass -or @($diagnose.modules).Count -eq 0 -or
        @($diagnose.modules | Where-Object provenance -ne 'external').Count -ne 0) {
        throw 'external-module provenance failed'
    }
    $commit = (& git rev-parse HEAD).Trim()
} finally { Pop-Location }

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T63ProbeProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Hash([string]$Path) { (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$probeEnv = 'PNP_T63_WORKER_PROBE=1'
$inputs = [ordered]@{
    captured_commit=$commit; exe_sha256=(Hash $Exe); model_sha256=(Hash $Model)
    probe_environment=$probeEnv; guest_freshness_exit=0
    provenance=$diagnose.modules
    modules=@(Get-ChildItem $Modules -Recurse -File | Where-Object Extension -in @('.wasm','.toml') |
        Sort-Object FullName | ForEach-Object {
            [pscustomobject]@{path=[IO.Path]::GetRelativePath($Repo,$_.FullName).Replace('\','/'); sha256=(Hash $_.FullName)}
        })
}
$inputs | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $RawRoot 'inputs.json') -Encoding utf8
$qualification = Get-Content (Join-Path $PSScriptRoot '..\t58-finalization-copies\qualification.json') -Raw | ConvertFrom-Json
# jobs: run_id, config generator, supports, RAYON_NUM_THREADS
$jobs = @(
    @('arachne-on-r1','arachne','on',12),
    @('arachne-on-r2','arachne','on',12),
    @('arachne-off-r1','arachne','off',12),
    @('classic-on-r1','classic','on',12),
    @('classic-off-r1','classic','off',12),
    @('arachne-on-t6-r1','arachne','on',6),
    @('arachne-on-t24-r1','arachne','on',24)
)
foreach ($job in $jobs) {
    $id,$generator,$supports,$threads = $job
    $config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$generator-supports-$supports.json"
    $reference = @($qualification.cells | Where-Object cell -eq "ordinary-benchy-$generator-$supports")
    if ($reference.Count -ne 1) { throw "missing reference for $id" }
    $out = Join-Path $RawRoot "$id.gcode"
    $err = Join-Path $RawRoot "$id.stderr.jsonl"
    $stdout = Join-Path $RawRoot "$id.stdout.log"
    $arguments = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',('"'+$Modules+'"'),
        '--no-default-module-paths','--no-integrated-modules','--instrument-stderr')
    $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
        -WorkingDirectory $Repo -Environment @{RAYON_NUM_THREADS="$threads"; PNP_T63_WORKER_PROBE='1'} `
        -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(600000)) { $p.Kill(); throw "diagnostic timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T63ProbeProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([T63ProbeProcessTimes]::T($e)-[T63ProbeProcessTimes]::T($c))/10000000.0
    $cpu = ([T63ProbeProcessTimes]::T($k)+[T63ProbeProcessTimes]::T($u))/10000000.0
    $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err `
        -ExpectedGenerator $generator -ExitCode $p.ExitCode
    if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or
        $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output: $id" }
    $sha = Hash $out
    if ($sha -ne $reference[0].sha256) { throw "reference output drift: $id ($sha)" }
    [pscustomobject][ordered]@{
        run_id=$id; generator=$generator; supports=$supports; threads=$threads
        exit_code=$p.ExitCode; wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=(Get-Item $out).Length; sha256=$sha; reference_sha256=$reference[0].sha256
        config_sha256=(Hash $config); stderr_sha256=(Hash $err)
        status=$valid.CompletionStatus; degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
        command=($Exe+' '+($arguments -join ' '))
    } | Export-Csv -Path (Join-Path $RawRoot 'runs.csv') -Append -NoTypeInformation -Encoding utf8
    $p.Dispose()
    Write-Host ("[t63-probe] {0}: wall={1:N3}s cpu={2:N3}s output={3}" -f $id,$wall,$cpu,$sha)
}
if ((Hash $Exe) -ne $inputs.exe_sha256) { throw 'binary changed during batch' }
foreach ($module in $inputs.modules) {
    if ((Hash (Join-Path $Repo $module.path)) -ne $module.sha256) { throw "module changed: $($module.path)" }
}
Write-Host '[t63-probe] batch complete'