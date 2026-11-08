# Diagnostic captures only; this is not an optimization or acceptance campaign.
[CmdletBinding()]
param([string]$RawRoot = 'target\perf-refresh-2026-10-13')
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
public static class BenchyRefreshProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Hash([string]$Path) { (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$inputs = [ordered]@{
    captured_commit=$commit; exe_sha256=(Hash $Exe); model_sha256=(Hash $Model)
    threads=12; guest_freshness_exit=0; provenance=$diagnose.modules
    modules=@(Get-ChildItem $Modules -Recurse -File | Where-Object Extension -in @('.wasm','.toml') |
        Sort-Object FullName | ForEach-Object {
            [pscustomobject]@{path=[IO.Path]::GetRelativePath($Repo,$_.FullName).Replace('\','/'); sha256=(Hash $_.FullName)}
        })
}
$inputs | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $RawRoot 'inputs.json') -Encoding utf8
$qualification = Get-Content (Join-Path $PSScriptRoot '..\t58-finalization-copies\qualification.json') -Raw | ConvertFrom-Json
$Csv = Join-Path $RawRoot 'runs.csv'
$jobs = @(
    @('classic','off','profile'), @('classic','on','profile'),
    @('arachne','off','profile'), @('arachne','on','profile'),
    @('arachne','off','instrumented'), @('arachne','on','instrumented')
)
foreach ($job in $jobs) {
    $generator,$supports,$kind = $job
    $id = "ordinary-benchy-$generator-$supports-$kind"
    $config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$generator-supports-$supports.json"
    $reference = @($qualification.cells | Where-Object cell -eq "ordinary-benchy-$generator-$supports")
    if ($reference.Count -ne 1) { throw "missing reference for $id" }
    $out = Join-Path $RawRoot "$id.gcode"
    $err = Join-Path $RawRoot "$id.stderr.jsonl"
    $stdout = Join-Path $RawRoot "$id.stdout.log"
    $arguments = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',('"'+$Modules+'"'),
        '--no-default-module-paths','--no-integrated-modules','--instrument-stderr')
    if ($kind -eq 'profile') { $arguments += @('--profile','--profile-verbose') }
    $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
        -WorkingDirectory $Repo -Environment @{RAYON_NUM_THREADS='12'} `
        -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(600000)) { $p.Kill(); throw "diagnostic timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [BenchyRefreshProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([BenchyRefreshProcessTimes]::T($e)-[BenchyRefreshProcessTimes]::T($c))/10000000.0
    $cpu = ([BenchyRefreshProcessTimes]::T($k)+[BenchyRefreshProcessTimes]::T($u))/10000000.0
    $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err `
        -ExpectedGenerator $generator -ExitCode $p.ExitCode
    if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or
        $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output: $id" }
    $sha = Hash $out
    if ($sha -ne $reference[0].sha256) { throw "reference output drift: $id ($sha)" }
    $types = [ordered]@{}
    foreach ($line in [IO.File]::ReadLines($out)) {
        if ($line.StartsWith(';TYPE:')) {
            $key = $line.Substring(6)
            if (-not $types.Contains($key)) { $types[$key]=0 }
            $types[$key]++
        }
    }
    if (-not $types.Contains('Outer wall') -or -not $types.Contains('Sparse infill')) { throw 'vacuous model roles' }
    if ($supports -eq 'on' -and (-not $types.Contains('Support') -or -not $types.Contains('Support interface'))) {
        throw 'vacuous support roles'
    }
    [pscustomobject][ordered]@{
        run_id=$id; kind=$kind; generator=$generator; supports=$supports; threads=12
        exit_code=$p.ExitCode; wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=(Get-Item $out).Length; sha256=$sha; reference_sha256=$reference[0].sha256
        config_sha256=(Hash $config); stderr_sha256=(Hash $err)
        type_counts=($types|ConvertTo-Json -Compress); status=$valid.CompletionStatus
        degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
        command=($Exe+' '+($arguments -join ' '))
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $p.Dispose()
    Write-Host ("[benchy-refresh] {0}: wall={1:N3}s cpu={2:N3}s output={3}" -f $id,$wall,$cpu,$sha)
}
if ((Hash $Exe) -ne $inputs.exe_sha256) { throw 'binary changed during batch' }
foreach ($module in $inputs.modules) {
    if ((Hash (Join-Path $Repo $module.path)) -ne $module.sha256) { throw "module changed: $($module.path)" }
}
