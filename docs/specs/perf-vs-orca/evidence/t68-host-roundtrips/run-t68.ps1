# t68 classic-perimeters host-service round-trip captures (probe run; attribution only).
# Wayfinder perf-vs-orca ticket 68. Mirrors evidence/t60-host-native/run-t60.ps1.
# Requires the T68-PROBE to be applied (see probe.patch) and the probe guest
# rebuilt (`cargo xtask build-guests`) before running.
[CmdletBinding()]
param([string]$RawRoot = 'target\t68-host-roundtrips')
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
if (-not [IO.Path]::IsPathRooted($RawRoot)) { $RawRoot = Join-Path $Repo $RawRoot }
New-Item -ItemType Directory -Path $RawRoot -Force | Out-Null
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Exe = Join-Path $Repo 'target\release\pnp_cli.exe'
$Modules = Join-Path $Repo 'modules\core-modules'
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
public static class T68ProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) { return ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime; }
}
'@
function Hash([string]$Path) { (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$guest = Join-Path $Modules 'classic-perimeters\classic-perimeters.wasm'
$inputs = [ordered]@{
    captured_commit=$commit; exe_sha256=(Hash $Exe); probe_guest_sha256=(Hash $guest)
    threads=12; guest_freshness_exit=0
    probe='T68-PROBE applied (probe.patch); tree state = HEAD + probe'
    provenance=$diagnose.modules
    models=[ordered]@{
        benchy=(Hash (Join-Path $Repo 'tmp\3dbenchy.stl'))
        base=(Hash (Join-Path $Repo 'tmp\base.stl'))
    }
}
$inputs | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $RawRoot 'inputs.json') -Encoding utf8
$qualification = Get-Content (Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t58-finalization-copies\qualification.json') -Raw | ConvertFrom-Json
$Csv = Join-Path $RawRoot 'runs.csv'
# Ticket 68 headline cells: the two classic supports-off jobs (base-off is the
# largest owner; benchy-off is the contrast). Supports-on cells differ by only
# ~4% in t60's owner wall and accelerated mode is host-boundary invariant.
$jobs = @(
    @('benchy','classic','off'), @('base','classic','off')
)
foreach ($job in $jobs) {
    $model,$generator,$supports = $job
    $id = "$model-$generator-$supports-profile"
    $config = Join-Path $Repo "docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-$generator-supports-$supports.json"
    $reference = @($qualification.cells | Where-Object cell -eq "ordinary-$model-$generator-$supports")
    if ($reference.Count -ne 1) { throw "missing reference for $id" }
    $stl = Join-Path $Repo "tmp\$($model -replace 'benchy','3dbenchy').stl"
    $out = Join-Path $RawRoot "$id.gcode"
    $err = Join-Path $RawRoot "$id.stderr.jsonl"
    $stdout = Join-Path $RawRoot "$id.stdout.log"
    $arguments = @('slice','--model',('"'+$stl+'"'),'--config',('"'+$config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',('"'+$Modules+'"'),
        '--no-default-module-paths','--no-integrated-modules','--instrument-stderr',
        '--profile','--profile-verbose')
    $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
        -WorkingDirectory $Repo -Environment @{RAYON_NUM_THREADS='12'} `
        -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "diagnostic timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T68ProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([T68ProcessTimes]::T($e)-[T68ProcessTimes]::T($c))/10000000.0
    $cpu = ([T68ProcessTimes]::T($k)+[T68ProcessTimes]::T($u))/10000000.0
    $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err `
        -ExpectedGenerator $generator -ExitCode $p.ExitCode
    if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or
        $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output: $id" }
    $sha = Hash $out
    if ($sha -ne $reference[0].sha256) { throw "reference output drift: $id ($sha)" }
    [pscustomobject][ordered]@{
        run_id=$id; model=$model; generator=$generator; supports=$supports; threads=12
        exit_code=$p.ExitCode; wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=(Get-Item $out).Length; sha256=$sha; reference_sha256=$reference[0].sha256
        config_sha256=(Hash $config); stderr_sha256=(Hash $err)
        status=$valid.CompletionStatus; degraded=$valid.Degraded
        fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
        command=($Exe+' '+($arguments -join ' '))
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $p.Dispose()
    Write-Host ("[t68] {0}: wall={1:N3}s cpu={2:N3}s output={3}" -f $id,$wall,$cpu,$sha)
}
if ((Hash $Exe) -ne $inputs.exe_sha256) { throw 'binary changed during batch' }
if ((Hash $guest) -ne $inputs.probe_guest_sha256) { throw 'guest changed during batch' }
