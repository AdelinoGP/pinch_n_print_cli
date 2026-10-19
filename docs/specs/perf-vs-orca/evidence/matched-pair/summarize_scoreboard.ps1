<#
.SYNOPSIS
    Aggregate run_scoreboard.ps1 rows into the ticket-11 scoreboard view.

.DESCRIPTION
    Reads scoreboard-rows.csv from one or more batches and prints, per
    (fixture, cell, tool): every measured sample with its cpu/wall ratio, the
    starvation flags (map rule: cpu/wall quoted per sample, starved samples
    excluded), median wall / median CPU over retained samples, per-cell gaps
    against Orca and between PNP modes, and the output disclosure (gcode
    bytes, TYPE/FEATURE section counts from the median run's stats file,
    degraded flag and non-fatal error counts).

    Starvation rule (PERF-HANDOFF section 10.3): within a measured group a
    sample is starved when its cpu/wall ratio falls below 0.75x the group's
    best ratio - it could have held the CPU at least as well as its peers but
    did not. Excluded samples are still printed and never deleted.

.EXAMPLE
    pwsh -NoProfile -File summarize_scoreboard.ps1 -BatchIds s1-benchy,s2-base
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string[]]$BatchIds,
    [double]$StarvationRatio = 0.75,
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))))
)

$ErrorActionPreference = 'Stop'

$rows = @()
foreach ($batch in $BatchIds) {
    $runDir = Join-Path $RepoRoot ('target\matched-pair\' + $batch)
    $csv = Join-Path $runDir 'scoreboard-rows.csv'
    $batchRows = Import-Csv $csv
    foreach ($r in $batchRows) {
        $r | Add-Member -NotePropertyName batch -NotePropertyValue $batch -Force
        $r | Add-Member -NotePropertyName stats_dir -NotePropertyValue (Join-Path $runDir 'stats') -Force
        $rows += $r
    }
}

$measured = @($rows | Where-Object { $_.warmup -eq '0' })
Write-Output ('rows total=' + @($rows).Count + ' measured=' + $measured.Count)

$groups = $measured | Group-Object fixture, generator, supports, tool | Sort-Object Name
$byKey = @{}
foreach ($g in $groups) {
    $r0 = $g.Group[0]
    $key = "$($r0.fixture)/$($r0.generator)-$($r0.supports)"
    if (-not $byKey.ContainsKey($key)) { $byKey[$key] = @{} }
    $byKey[$key][$r0.tool] = $g.Group
}

foreach ($g in $groups) {
    $samples = @($g.Group | Sort-Object { [int]$_.run_index })
    $ratios = $samples | ForEach-Object { [double]$_.cpu_wall_ratio }
    $maxRatio = ($ratios | Measure-Object -Maximum).Maximum
    $lines = @()
    $retained = @()
    foreach ($s in $samples) {
        $starved = ([double]$s.cpu_wall_ratio) -lt ($StarvationRatio * $maxRatio)
        if (-not $starved) { $retained += $s }
        $lines += ('    run' + $s.run_index + ': wall=' + [math]::Round([double]$s.wall_seconds, 2) + 's cpu=' + [math]::Round([double]$s.cpu_seconds, 2) + 's ratio=' + $s.cpu_wall_ratio + $(if ($starved) { '  <-- STARVED, excluded' } else { '' }))
    }
    if ($retained.Count -eq 0) { $retained = $samples; $lines += '    (all samples starved-flagged; medians over all)' }
    $walls = $retained | ForEach-Object { [double]$_.wall_seconds } | Sort-Object
    $cpus = $retained | ForEach-Object { [double]$_.cpu_seconds } | Sort-Object
    $med = { param($arr) if ($arr.Count % 2 -eq 1) { $arr[[int](($arr.Count - 1) / 2)] } else { ($arr[$arr.Count / 2 - 1] + $arr[$arr.Count / 2]) / 2 } }
    $medWall = & $med $walls
    $medCpu = & $med $cpus
    $bytes = $samples | ForEach-Object { [int64]$_.gcode_bytes }
    $degraded = ($samples | ForEach-Object { $_.degraded } | Sort-Object -Unique) -join '/'
    $nonFatal = ($samples | ForEach-Object { $_.non_fatal_error_count } | Sort-Object -Unique) -join '/'
    $medianSample = $retained | Sort-Object { [double]$_.wall_seconds } | Select-Object -Last 1
    $runIdIdx = [int][math]::Floor(($retained.Count - 1) / 2)
    $medianSample = @($retained | Sort-Object { [double]$_.wall_seconds })[$runIdIdx]
    $statsPath = Join-Path $samples[0].stats_dir ($medianSample.run_id + '.json')
    $typeCounts = (Get-Content -LiteralPath $statsPath -Raw | ConvertFrom-Json).type_counts
    $typeStr = ($typeCounts.PSObject.Properties | ForEach-Object { $_.Name + '=' + $_.Value }) -join ', '
    Write-Output ('')
    Write-Output ('## ' + $samples[0].fixture + ' | ' + $samples[0].generator + '-' + $samples[0].supports + ' | ' + $samples[0].tool)
    $lines | ForEach-Object { Write-Output $_ }
    Write-Output ('    medians(n=' + $retained.Count + '): wall=' + [math]::Round($medWall, 2) + 's cpu=' + [math]::Round($medCpu, 2) + 's')
    Write-Output ('    gcode bytes: ' + (($bytes | Sort-Object) -join '/'))
    Write-Output ('    degraded=' + $degraded + ' non_fatal=' + $nonFatal)
    Write-Output ('    sections (' + $medianSample.run_id + '): ' + $typeStr)
}

Write-Output ('')
Write-Output ('## Per-cell gaps (median wall ratios)')
foreach ($key in ($byKey.Keys | Sort-Object)) {
    $cellTools = $byKey[$key]
    function Get-MedWall([object[]]$samples) {
        $ratios = $samples | ForEach-Object { [double]$_.cpu_wall_ratio }
        $maxRatio = ($ratios | Measure-Object -Maximum).Maximum
        $retained = @($samples | Where-Object { ([double]$_.cpu_wall_ratio) -ge ($StarvationRatio * $maxRatio) })
        if ($retained.Count -eq 0) { $retained = $samples }
        $walls = @($retained | ForEach-Object { [double]$_.wall_seconds } | Sort-Object)
        if ($walls.Count % 2 -eq 1) { return $walls[[int](($walls.Count - 1) / 2)] }
        return ($walls[$walls.Count / 2 - 1] + $walls[$walls.Count / 2]) / 2
    }
    $orca = $cellTools['orca']; $ord = $cellTools['pnp-ordinary']; $acc = $cellTools['pnp-accelerated']
    $wOrca = Get-MedWall $orca; $wOrd = Get-MedWall $ord; $wAcc = Get-MedWall $acc
    Write-Output ($key + ': pnp-ordinary/orca=' + [math]::Round($wOrd / $wOrca, 2) + 'x  pnp-accelerated/orca=' + [math]::Round($wAcc / $wOrca, 2) + 'x  accelerated/ordinary=' + [math]::Round($wAcc / $wOrd, 3))
}
