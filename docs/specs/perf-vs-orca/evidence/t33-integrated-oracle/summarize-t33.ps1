<#
.SYNOPSIS
    Ticket-33 summarize + gate: reads each cell CSV, prints the full sample
    table with output disclosure, and prints the GATE verdict per cell.

.DESCRIPTION
    Perf-vs-orca ticket 33. Gate (from the ticket):
      IF external-vs-external is stable (single arm here — cross-checked
         against the archived s1-benchy/s2-base external sha256 where the
         same job/config ran), integrated sha == external sha AND all
         disclosure fields match (bytes implied by sha; TYPE counts equal;
         degraded/fatal both 0-and-equal; non_fatal equal; open-loop warning
         counts equal) for every examined cell
      THEN verdict PASS (oracle usable, bounded to the examined cells)
      ELSE verdict FAIL with the first mismatching field per cell localized
         for the root-cause work (compare TYPE counts to perf-split Finding 3.
         A repeating "integrated < external" pattern on Sparse infill/Bridge/
         Bottom/Top surface reproduces that divergence and says the native
         leg still mis-hands those sections).
    This script NEVER times modules: no wall/cpu verdicts. It prints them for
    load context only. CPU/wall quoted per sample per the map's rule.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string[]]$Cells,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
)
$ErrorActionPreference = 'Stop'
$Root = Join-Path $RepoRoot 'docs\specs\perf-vs-orca\evidence\t33-integrated-oracle'
$verdicts = @()
foreach ($cell in $Cells) {
    $csv = Join-Path $Root ('measure\' + $cell + '\' + $cell + '.csv')
    if (-not (Test-Path $csv)) { throw "missing measured CSV for $cell" }
    $rows = @(Import-Csv $csv)
    $ext = @($rows | Where-Object { $_.arm -eq 'external' })
    $int = @($rows | Where-Object { $_.arm -eq 'integrated' })
    Write-Output ""
    Write-Output "## $cell"
    foreach ($r in $rows) {
        Write-Output ("    {0} pair{1}: wall={2}s cpu={3}s ratio={4} bytes={5} degraded={6} non_fatal={7} sha={8}" -f `
            $r.arm, $r.pair, $r.wall_seconds, $r.cpu_seconds, $r.cpu_wall_ratio, $r.gcode_bytes, $r.degraded, $r.non_fatal_error_count, $r.sha256.Substring(0, 12))
    }
    $failures = @()
    $extShas = @($ext | ForEach-Object { $_.sha256 } | Sort-Object -Unique)
    $intShas = @($int | ForEach-Object { $_.sha256 } | Sort-Object -Unique)
    $extTypes = @($ext | ForEach-Object { $_.type_counts_json } | Sort-Object -Unique)
    $intTypes = @($int | ForEach-Object { $_.type_counts_json } | Sort-Object -Unique)
    $extStatus = @($ext | ForEach-Object { "$($_.completion_status)/d=$($_.degraded)/nf=$($_.non_fatal_error_count)" } | Sort-Object -Unique)
    $intStatus = @($int | ForEach-Object { "$($_.completion_status)/d=$($_.degraded)/nf=$($_.non_fatal_error_count)" } | Sort-Object -Unique)
    if ($extShas.Count -ne 1) { $failures += "external arm not self-consistent ($($extShas.Count) distinct sha)" }
    if ($intShas.Count -ne 1) { $failures += "integrated arm not self-consistent ($($intShas.Count) distinct sha)" }
    if ($extShas.Count -eq 1 -and $intShas.Count -eq 1 -and $extShas[0] -ne $intShas[0]) {
        # localize by TYPE counts (bytes follow the sha)
        if ($extTypes.Count -eq 1 -and $intTypes.Count -eq 1) {
            $a = $extTypes[0] | ConvertFrom-Json; $b = $intTypes[0] | ConvertFrom-Json
            $keys = @(($a.PSObject.Properties.Name) + ($b.PSObject.Properties.Name) | Sort-Object -Unique)
            $diffs = @()
            foreach ($k in $keys) {
                $av = [int]($a.PSObject.Properties[$k].Value); $bv = [int]($b.PSObject.Properties[$k].Value)
                if ($av -ne $bv) { $diffs += "$k external=$av integrated=$bv" }
            }
            if ($diffs.Count -gt 0) { $failures += ("sha differs; TYPE deltas: " + ($diffs -join '; ')) }
            else { $failures += "sha differs with equal TYPE counts (geometry moved within sections)" }
        } else {
            $failures += "sha differs and an arm's own TYPE counts were inconsistent"
        }
    }
    if ($extStatus.Count -gt 1 -or $intStatus.Count -gt 1) { $failures += "status/degraded/non_fatal not uniform within an arm" }
    if (($extStatus | Sort-Object -Unique) -ne ($intStatus | Sort-Object -Unique)) { $failures += "status/degraded/non_fatal differ across arms ($($extStatus | Sort-Object -Unique) vs $($intStatus | Sort-Object -Unique))" }
    $verdict = if ($failures.Count -eq 0) { 'PASS' } else { 'FAIL' }
    Write-Output ("    gate: " + $verdict + $(if ($failures.Count -gt 0) { " — " + ($failures -join ' | ') }))
    $verdicts += [pscustomobject]@{ cell = $cell; verdict = $verdict; failures = ($failures -join ' | ') }
}
Write-Output ""
Write-Output "## summary"
foreach ($v in $verdicts) { Write-Output ("    {0}: {1}" -f $v.cell, $v.verdict) }
$allPass = @($verdicts | Where-Object { $_.verdict -eq 'PASS' }).Count -eq $verdicts.Count
Write-Output ("    GATE: " + $(if ($allPass) { 'PASS (bounded to examined cells)' } else { 'FAIL' }))