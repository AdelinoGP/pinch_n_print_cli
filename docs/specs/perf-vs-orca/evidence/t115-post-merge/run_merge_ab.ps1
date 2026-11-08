<#
.SYNOPSIS
    Pre-merge vs merged PnP paired A/B (perf-vs-orca ticket 115).

.DESCRIPTION
    Interleaved paired measurement of the pre-merge tree and the merged tree on
    the matched job. The pre-merge arm is the preserved t103 snapshot
    (`target/dist/developer/`, binary `4cb96942...`, its own 24-module tree,
    which reproduces the archived pre-merge reference hashes); the merged arm is
    `target/release/pnp_cli.exe` + `modules/core-modules`.

    Wall = process creation-to-exit, CPU = kernel+user via the process handle.
    The measurement math follows evidence/matched-pair/run_scoreboard.ps1. Order
    alternates per pair so no arm always leads.

.EXAMPLE
    pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t115-post-merge/run_merge_ab.ps1 -Pairs 3
#>
[CmdletBinding()]
param(
    [ValidateSet('benchy-classic-off', 'benchy-classic-on', 'base-classic-off', 'base-classic-on',
        'benchy-arachne-off', 'benchy-arachne-on', 'base-arachne-off', 'base-arachne-on')]
    [string[]]$Cells = @('benchy-classic-off', 'benchy-classic-on', 'base-classic-off', 'base-classic-on'),
    [int]$Pairs = 3,
    [int]$Threads = 12,
    [string]$BatchId = 't115-merge-ab',
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$ConfigDir = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\matched-pair\configs'
$OutDir = Join-Path $RepoRoot ('target\t115-ab\' + $BatchId)
$GcodeDir = Join-Path $OutDir 'gcode'
New-Item -ItemType Directory -Force -Path $GcodeDir | Out-Null
$CsvPath = Join-Path $OutDir 'ab-rows.csv'

$Arms = [ordered]@{
    'pre' = @{
        Exe = Join-Path $RepoRoot 'target\dist\developer\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'target\dist\developer\modules'
    }
    'post' = @{
        Exe = Join-Path $RepoRoot 'target\release\pnp_cli.exe'
        ModuleDir = Join-Path $RepoRoot 'modules\core-modules'
    }
}

$Fixtures = @{ benchy = (Join-Path $RepoRoot 'tmp\3dbenchy.stl'); base = (Join-Path $RepoRoot 'tmp\base.stl') }

function Get-CellParts {
    param([string]$Cell)
    $parts = $Cell.Split('-')
    $fixture = $parts[0]
    $generator = $parts[1]
    $supports = $parts[2]
    $config = Join-Path $ConfigDir ("pnp-$generator-supports-$supports.json")
    return @{ Fixture = $fixture; Model = $Fixtures[$fixture]; Config = $config }
}

function Measure-Slice {
    param([string]$Cell, [string]$Arm, [string]$OutPath)
    $parts = Get-CellParts -Cell $Cell
    $spec = $Arms[$Arm]
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $spec.Exe
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    foreach ($a in @('slice', '--model', $parts.Model, '--output', $OutPath, '--config', $parts.Config, '--module-dir', $spec.ModuleDir)) {
        $psi.ArgumentList.Add($a)
    }
    $psi.Environment['RAYON_NUM_THREADS'] = "$Threads"
    $stderrPath = [System.IO.Path]::ChangeExtension($OutPath, '.stderr.jsonl')
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $errTask = $p.StandardError.ReadToEndAsync()
    $p.StandardOutput.ReadToEnd() | Out-Null
    $p.WaitForExit()
    $sw.Stop()
    $stderr = $errTask.Result
    Set-Content -LiteralPath $stderrPath -Value $stderr -Encoding utf8
    $cpu = $p.TotalProcessorTime.TotalSeconds
    $sha = if (Test-Path -LiteralPath $OutPath) { (Get-FileHash -LiteralPath $OutPath -Algorithm SHA256).Hash.ToLower() } else { '' }
    $bytes = if (Test-Path -LiteralPath $OutPath) { (Get-Item -LiteralPath $OutPath).Length } else { 0 }
    return [pscustomobject]@{
        wall_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 4)
        cpu_seconds = [math]::Round($cpu, 4)
        gcode_bytes = $bytes
        sha256 = $sha
        exit_code = $p.ExitCode
    }
}

$header = 'pair,order,cell,arm,wall_seconds,cpu_seconds,gcode_bytes,sha256,exit_code,timestamp_utc'
Set-Content -LiteralPath $CsvPath -Value $header -Encoding utf8

Write-Host "[t115-ab] batch $BatchId cells=$($Cells -join ',') pairs=$Pairs threads=$Threads"
for ($pair = 1; $pair -le $Pairs; $pair++) {
    foreach ($cell in $Cells) {
        $order = if ($pair % 2 -eq 1) { @('pre', 'post') } else { @('post', 'pre') }
        foreach ($arm in $order) {
            $runId = "$cell-$arm-p$pair"
            Write-Host "[t115-ab] START $runId"
            $row = Measure-Slice -Cell $cell -Arm $arm -OutPath (Join-Path $GcodeDir ($runId + '.gcode'))
            $line = @(
                $pair, ($order -join '>'), $cell, $arm, $row.wall_seconds, $row.cpu_seconds,
                $row.gcode_bytes, $row.sha256, $row.exit_code, (Get-Date).ToUniversalTime().ToString('o')
            ) -join ','
            Add-Content -LiteralPath $CsvPath -Value $line -Encoding utf8
            Write-Host ("[t115-ab] DONE  {0} wall={1}s cpu={2}s bytes={3} sha={4} exit={5}" -f $runId, $row.wall_seconds, $row.cpu_seconds, $row.gcode_bytes, $row.sha256.Substring(0, [math]::Min(12, $row.sha256.Length)), $row.exit_code)
        }
    }
}
Write-Host "[t115-ab] complete: $CsvPath"
