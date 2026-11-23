<#
.SYNOPSIS
    Resolve the matched job through OrcaSlicer and capture each cell's
    G-code config appendix.

.DESCRIPTION
    Ticket 82 (perf-vs-orca). For every cell of the matched matrix
    {benchy, base} x {classic, arachne} x {supports off, on} this script:

      1. copies the vendor process profile and applies the same string-typed
         override block the matched-pair scoreboard rig uses
         (docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1),
      2. slices the fixture with OrcaSlicer,
      3. extracts every `; key = value` line of the emitted G-code appendix
         into `appendix/<fixture>-<generator>-supports-<state>.txt`.

    The appendix is the authority for what the resolved config actually was.
    Raw G-code, logs and derived configs stay under target/t82-key-audit/ and
    are never committed.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t82-fairness-key-audit/run_orca_appendix.ps1
#>
[CmdletBinding()]
param(
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

# --- Matched job (lock-step with evidence/matched-pair/run_scoreboard.ps1) ----
$Job = @{
    LayerHeight         = '0.2'
    FirstLayerHeight    = '0.2'
    WallLoops           = '2'
    SparseInfillDensity = '20%'
    SparseInfillPattern = 'gyroid'
    SupportType         = 'tree(auto)'
}

$OrcaExe     = 'C:\Program Files\OrcaSlicer\orca-slicer.exe'
$OrcaProfile = 'C:\Program Files\OrcaSlicer\resources\profiles\BBL'
$OrcaMachine = Join-Path $OrcaProfile 'machine\Bambu Lab X1 Carbon 0.4 nozzle.json'
$OrcaVendorProcess = Join-Path $OrcaProfile 'process\0.20mm Standard @BBL X1C.json'
$OrcaFilament = Join-Path $OrcaProfile 'filament\Generic PLA.json'

$FixturePaths = @{
    benchy = Join-Path $RepoRoot 'tmp\3dbenchy.stl'
    base   = Join-Path $RepoRoot 'tmp\base.stl'
}

$Here      = $PSScriptRoot
$AppendixDir = Join-Path $Here 'appendix'
$WorkRoot  = Join-Path $RepoRoot 'target\t82-key-audit'
$ConfigDir = Join-Path $WorkRoot 'configs'
$GcodeDir  = Join-Path $WorkRoot 'gcode'

foreach ($d in @($AppendixDir, $WorkRoot, $ConfigDir, $GcodeDir)) {
    New-Item -ItemType Directory -Path $d -Force | Out-Null
}

foreach ($p in @($OrcaExe, $OrcaMachine, $OrcaVendorProcess, $OrcaFilament)) {
    if (-not (Test-Path -LiteralPath $p)) { throw "missing-artifact: $p" }
}
foreach ($f in $FixturePaths.Keys) {
    if (-not (Test-Path -LiteralPath $FixturePaths[$f])) { throw "missing-artifact: $($FixturePaths[$f])" }
}

function New-OrcaDerivedConfig {
    param(
        [Parameter(Mandatory = $true)][string]$Generator,
        [Parameter(Mandatory = $true)][string]$SupportFlag
    )
    $path = Join-Path $ConfigDir ('derived-' + $Generator + '-support' + $SupportFlag + '.json')
    if (Test-Path -LiteralPath $path) { return $path }
    $j = Get-Content -LiteralPath $OrcaVendorProcess -Raw | ConvertFrom-Json
    $sets = [ordered]@{
        name                      = 'pnp-matched-' + $Generator + '-support' + $SupportFlag
        wall_generator            = $Generator
        wall_loops                = $Job.WallLoops
        sparse_infill_density     = $Job.SparseInfillDensity
        sparse_infill_pattern     = $Job.SparseInfillPattern
        enable_support            = $SupportFlag
        support_type              = $Job.SupportType
        layer_height              = $Job.LayerHeight
        initial_layer_print_height = $Job.FirstLayerHeight
    }
    foreach ($k in $sets.Keys) {
        $j | Add-Member -NotePropertyName $k -NotePropertyValue ([string]$sets[$k]) -Force
    }
    $j | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $path
    return $path
}

$runs = @()
foreach ($fixture in @('benchy', 'base')) {
    foreach ($generator in @('classic', 'arachne')) {
        foreach ($supports in @('off', 'on')) {
            $supportFlag = if ($supports -eq 'on') { '1' } else { '0' }
            $runId = "$fixture-$generator-supports-$supports"
            $derived = New-OrcaDerivedConfig -Generator $generator -SupportFlag $supportFlag
            $outDir = Join-Path $GcodeDir $runId
            New-Item -ItemType Directory -Path $outDir -Force | Out-Null

            $loadSettings = $OrcaMachine + ';' + $derived
            $argList = @(
                '--slice', '0',
                '--load-settings', ('"' + $loadSettings + '"'),
                '--load-filaments', ('"' + $OrcaFilament + '"'),
                '--outputdir', ('"' + $outDir + '"'),
                ('"' + $FixturePaths[$fixture] + '"')
            )

            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $proc = Start-Process -FilePath $OrcaExe -ArgumentList $argList -Wait -PassThru -NoNewWindow `
                -RedirectStandardOutput (Join-Path $outDir 'orca.log') `
                -RedirectStandardError (Join-Path $outDir 'orca.err.log')
            $sw.Stop()

            $gcodePath = Join-Path $outDir 'plate_1.gcode'
            if ($proc.ExitCode -ne 0) { throw "orca failed ($runId) exit $($proc.ExitCode)" }
            if (-not (Test-Path -LiteralPath $gcodePath)) { throw "orca produced no plate_1.gcode ($runId)" }

            $appendixPath = Join-Path $AppendixDir ($runId + '.txt')
            $lines = New-Object System.Collections.Generic.List[string]
            foreach ($line in [System.IO.File]::ReadLines($gcodePath)) {
                if ($line -match '^\s*;\s*([a-z0-9_]+)\s*=\s*(.*?)\s*$') {
                    $lines.Add($Matches[1] + ' = ' + $Matches[2])
                }
            }
            if ($lines.Count -lt 50) { throw "appendix truncated ($runId): $($lines.Count) keys" }
            Set-Content -LiteralPath $appendixPath -Value $lines -Encoding utf8

            $runs += [ordered]@{
                run_id        = $runId
                fixture       = $fixture
                generator     = $generator
                supports      = $supports
                exit_code     = $proc.ExitCode
                wall_s        = [math]::Round($sw.Elapsed.TotalSeconds, 3)
                appendix_keys = $lines.Count
                gcode_bytes   = (Get-Item -LiteralPath $gcodePath).Length
                derived_config = $derived
            }
            Write-Host ("{0}: {1} keys, {2:N0} bytes, {3:N1} s" -f $runId, $lines.Count, (Get-Item -LiteralPath $gcodePath).Length, $sw.Elapsed.TotalSeconds)
        }
    }
}

$runs | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $Here 'runs.json') -Encoding utf8
Write-Host ("wrote {0}" -f (Join-Path $Here 'runs.json'))
