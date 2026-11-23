# T64 probe captures: diagnostic attribution only (wayfinder perf-vs-orca
# ticket 64). Requires the temporary T64-PROBE instrument applied to
# `modules/core-modules/tree-support/src/lib.rs`; the guest artifact and this
# probe are removed after the take. Not an optimization or acceptance campaign:
# no uninstrumented wall claim may be made from these runs.
[CmdletBinding()]
param([string]$RawRoot = 'target\t64-tree-support-renderer')
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

# The tree-support guest artifact must currently be the PROBED build (fresh as
# built, but its hash differs from the committed state); disclosed in inputs.
$treeSupportArtifact = Get-Item (Join-Path $Modules 'tree-support\tree-support.component.wasm') -ErrorAction SilentlyContinue
if (-not $treeSupportArtifact) {
    $treeSupportArtifact = Get-ChildItem (Join-Path $Modules 'tree-support') -Recurse -Filter '*.wasm' | Select-Object -First 1
}
if (-not $treeSupportArtifact) { throw 'tree-support guest artifact not found' }

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class T64ProbeProcessTimes {
 [DllImport("kernel32.dll",SetLastError=true)]
 public static extern bool GetProcessTimes(IntPtr h,out System.Runtime.InteropServices.ComTypes.FILETIME c,out System.Runtime.InteropServices.ComTypes.FILETIME e,out System.Runtime.InteropServices.ComTypes.FILETIME k,out System.Runtime.InteropServices.ComTypes.FILETIME u);
 public static long T(System.Runtime.InteropServices.ComTypes.FILETIME f) => ((long)f.dwHighDateTime<<32)+(uint)f.dwLowDateTime;
}
'@
function Hash([string]$Path) { (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$inputs = [ordered]@{
    captured_commit=$commit; probed_tree_support_artifact=$treeSupportArtifact.FullName
    probed_tree_support_artifact_sha256=(Hash $treeSupportArtifact.FullName)
    exe_sha256=(Hash $Exe); model_sha256=(Hash $Model)
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
    @('classic','on'), @('arachne','on'),
    @('classic','off'), @('arachne','off')
)
foreach ($job in $jobs) {
    $generator,$supports = $job
    $id = "ordinary-benchy-$generator-$supports-probe64"
    $config = Join-Path $PSScriptRoot "..\matched-pair\configs\pnp-$generator-supports-$supports.json"
    $reference = @($qualification.cells | Where-Object cell -eq "ordinary-benchy-$generator-$supports")
    if ($reference.Count -ne 1) { throw "missing reference for $id" }
    $out = Join-Path $RawRoot "$id.gcode"
    $err = Join-Path $RawRoot "$id.stderr.jsonl"
    $stdout = Join-Path $RawRoot "$id.stdout.log"
    $arguments = @('slice','--model',('"'+$Model+'"'),'--config',('"'+$config+'"'),
        '--output',('"'+$out+'"'),'--module-dir',('"'+$Modules+'"'),
        '--no-default-module-paths','--no-integrated-modules','--instrument-stderr',
        '--profile','--profile-verbose')
    $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow `
        -WorkingDirectory $Repo -Environment @{RAYON_NUM_THREADS='12'} `
        -RedirectStandardError $err -RedirectStandardOutput $stdout
    $handle = $p.Handle
    if (-not $p.WaitForExit(1200000)) { $p.Kill(); throw "diagnostic timeout: $id" }
    $p.Refresh()
    $c = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $e = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $k = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    $u = New-Object System.Runtime.InteropServices.ComTypes.FILETIME
    if (-not [T64ProbeProcessTimes]::GetProcessTimes($handle,[ref]$c,[ref]$e,[ref]$k,[ref]$u)) {
        throw 'GetProcessTimes failed'
    }
    $wall = ([T64ProbeProcessTimes]::T($e)-[T64ProbeProcessTimes]::T($c))/10000000.0
    $cpu = ([T64ProbeProcessTimes]::T($k)+[T64ProbeProcessTimes]::T($u))/10000000.0
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
        run_id=$id; generator=$generator; supports=$supports; threads=12
        exit_code=$p.ExitCode; wall_seconds=$wall; cpu_seconds=$cpu; cpu_wall_ratio=($cpu/$wall)
        bytes=(Get-Item $out).Length; sha256=$sha; reference_sha256=$reference[0].sha256
        config_sha256=(Hash $config); stderr_sha256=(Hash $err)
        type_counts=($types|ConvertTo-Json -Compress); status=$valid.CompletionStatus
        degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
        command=($Exe+' '+($arguments -join ' '))
    } | Export-Csv -Path $Csv -Append -NoTypeInformation -Encoding utf8
    $p.Dispose()
    Write-Host ("[t64-probe] {0}: wall={1:N3}s cpu={2:N3}s output={3}" -f $id,$wall,$cpu,$sha)
}
if ((Hash $Exe) -ne $inputs.exe_sha256) { throw 'binary changed during batch' }
foreach ($module in $inputs.modules) {
    if ((Hash (Join-Path $Repo $module.path)) -ne $module.sha256) { throw "module changed: $($module.path)" }
}
Write-Host '[t64-probe] batch complete'