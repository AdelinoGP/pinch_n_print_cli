# Host-only T69-DUMP probe must be applied and scratch executable built first.
[CmdletBinding()]
param([string]$RawRoot = '.local-artifacts\perf-vs-orca\t69-clipper-engine\raw')
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
if (-not [IO.Path]::IsPathRooted($RawRoot)) { $RawRoot = Join-Path $Repo $RawRoot }
New-Item -ItemType Directory -Path $RawRoot -Force | Out-Null
. (Join-Path $Repo 'resources\perimeter-acceptance\validate_measurement.ps1')
$Exe = Join-Path $Repo '.local-artifacts\perf-vs-orca\t69-clipper-engine\build\pnp_cli.exe'
$Modules = Join-Path $Repo 'modules\core-modules'
Remove-Item Env:SLICER_MODULE_PATH -ErrorAction SilentlyContinue
function Hash([string]$Path) { (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$refs = (Get-Content (Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\t58-finalization-copies\qualification.json') -Raw | ConvertFrom-Json).cells
$binaryHash = Hash $Exe
Push-Location $Repo
try {
    & $Exe module diagnose --module-dir $Modules --no-default-module-paths --no-integrated-modules `
        1> (Join-Path $RawRoot 'diagnose.json') 2> (Join-Path $RawRoot 'diagnose.stderr.log')
    if ($LASTEXITCODE -ne 0) { throw "diagnose exit $LASTEXITCODE" }
    $diag = Get-Content (Join-Path $RawRoot 'diagnose.json') -Raw | ConvertFrom-Json
    if (-not $diag.pass -or @($diag.modules).Count -eq 0 -or @($diag.modules | Where-Object provenance -ne 'external').Count -ne 0) { throw 'external provenance failed' }
    foreach ($model in @('benchy','base')) {
        $id = "$model-classic-off"
        $ref = @($refs | Where-Object cell -eq "ordinary-$id")
        if ($ref.Count -ne 1) { throw "reference missing: $id" }
        $dump = Join-Path $RawRoot $id
        New-Item -ItemType Directory -Path $dump -Force | Out-Null
        $out = Join-Path $RawRoot "$id.gcode"
        $err = Join-Path $RawRoot "$id.stderr.jsonl"
        $stdout = Join-Path $RawRoot "$id.stdout.log"
        $meta = Join-Path $RawRoot "$id.json"
        $config = Join-Path $Repo 'docs\specs\perf-vs-orca\evidence\matched-pair\configs\pnp-classic-supports-off.json'
        $stl = Join-Path $Repo $(if ($model -eq 'benchy') { 'tmp\3dbenchy.stl' } else { 'tmp\base.stl' })
        $arguments = @('slice','--model',('"'+$stl+'"'),'--config',('"'+$config+'"'),
            '--output',('"'+$out+'"'),'--module-dir',('"'+$Modules+'"'),
            '--no-default-module-paths','--no-integrated-modules','--instrument-stderr','--profile','--profile-verbose')
        $skip = (Test-Path $meta) -and (Test-Path $out) -and (Hash $out) -eq $ref[0].sha256 -and @(Get-ChildItem $dump -Filter 'CP-*.calls').Count -gt 0
        if ($skip) { Write-Host "[t69] verified existing output; skip $id"; continue }
        $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -NoNewWindow -WorkingDirectory $Repo `
            -Environment @{RAYON_NUM_THREADS='12'; PNP_T69_DUMP=$dump} -RedirectStandardError $err -RedirectStandardOutput $stdout
        if (-not $p.WaitForExit(1800000)) { $p.Kill(); throw "capture timeout: $id" }
        $p.Refresh()
        $valid = Test-MeasurementEvidence -ConfigPath $config -OutputPath $out -StderrPath $err -ExpectedGenerator classic -ExitCode $p.ExitCode
        if ($valid.CompletionStatus -ne 'ok' -or $valid.Degraded -or $valid.FatalErrorCount -ne 0 -or $valid.NonFatalErrorCount -ne 0) { throw "unclean output $id" }
        $sha = Hash $out
        $row = [ordered]@{run_id=$id; exit_code=$p.ExitCode; sha256=$sha; reference_sha256=$ref[0].sha256; byte_identical=($sha -eq $ref[0].sha256)
            bytes=(Get-Item $out).Length; status=$valid.CompletionStatus; degraded=$valid.Degraded; fatal=$valid.FatalErrorCount; non_fatal=$valid.NonFatalErrorCount
            exe_sha256=$binaryHash; config_sha256=(Hash $config); model_sha256=(Hash $stl); stderr_sha256=(Hash $err); stdout_sha256=(Hash $stdout); threads=12; command=($Exe+' '+($arguments -join ' ')); dump_root=$dump}
        $row | ConvertTo-Json -Depth 8 | Set-Content $meta -Encoding utf8
        if ($sha -ne $ref[0].sha256) { throw "reference drift: $id ($sha)" }
        Write-Host "[t69] $id identity PASS $sha"
        $p.Dispose()
    }
    if ((Hash $Exe) -ne $binaryHash) { throw 'binary changed' }
} finally { Pop-Location }
