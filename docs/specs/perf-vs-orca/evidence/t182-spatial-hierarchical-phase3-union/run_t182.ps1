param(
    [ValidateSet("manifest", "sweep", "output", "ab")]
    [string]$Mode = "output",
    [string[]]$Cells = @(),
    [string[]]$SweepArms = @("baseline", "worker-k4", "worker-k8", "worker-k16", "nested-k4", "nested-k8", "nested-k16"),
    [int]$SweepRuns = 3,
    [int]$Pairs = 5,
    [int]$PairFrom = 1,
    [int]$PairTo = 0,
    [switch]$Append,
    [switch]$OnlyMissing,
    [switch]$NoWarmup,
    [string]$BatchId = "default",
    [switch]$NullControl,
    [switch]$Accelerated
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..\..")).Path
$TargetRoot = Join-Path $RepoRoot "target\t182"
$ConfigDir = Join-Path $RepoRoot "docs\specs\perf-vs-orca\evidence\matched-pair\configs"
$BenchyModel = Join-Path $RepoRoot "tmp\3dbenchy.stl"
$BaseModel = Join-Path $RepoRoot "tmp\base.stl"

# Active reference currency from t175 (docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reference.json)
$T175Reference = @{
    "benchy-classic-off" = "4c67ef0b73a885e20a9701a695c06d38507b0d17d3aaaff9d5e1046923e60d73"
    "benchy-classic-on"  = "fdca4e2d3d0cc32b80985c898e99b71fe7dd01330bce539bf5f3110bb84bcf82"
    "benchy-arachne-off" = "fd6313841eed2096778901ddb65be5bdc22e366a188b636928a62ee637288dd1"
    "benchy-arachne-on"  = "2869186c31a7d57e2b8e3a2edfd624623b54af43155efb8c5b68863fafd2a13c"
    "base-classic-off"   = "02ffa2d6e7e74732901d05c199bb09e4f49d12e4ab6f14662d349df538234fab"
    "base-classic-on"    = "7b90e7499bfad0e73a746307d05b657732f322e41fdccfb8b166a6e0a7d8e799"
    "base-arachne-off"   = "e806296f52f7200f5eaf91495e822e1ccf52a958a319f93672e40b8f0b03376c"
    "base-arachne-on"    = "f77633affab2fbea4f3b3b546aa564ba1a151a457fd20b2c4991f34526a58d5c"
}

$AllCells = @(
    [pscustomobject]@{ Id = "benchy-classic-off"; Model = $BenchyModel; Config = "pnp-classic-supports-off.json" },
    [pscustomobject]@{ Id = "benchy-classic-on";  Model = $BenchyModel; Config = "pnp-classic-supports-on.json"  },
    [pscustomobject]@{ Id = "benchy-arachne-off"; Model = $BenchyModel; Config = "pnp-arachne-supports-off.json" },
    [pscustomobject]@{ Id = "benchy-arachne-on";  Model = $BenchyModel; Config = "pnp-arachne-supports-on.json"  },
    [pscustomobject]@{ Id = "base-classic-off";   Model = $BaseModel;   Config = "pnp-classic-supports-off.json" },
    [pscustomobject]@{ Id = "base-classic-on";    Model = $BaseModel;   Config = "pnp-classic-supports-on.json"  },
    [pscustomobject]@{ Id = "base-arachne-off";   Model = $BaseModel;   Config = "pnp-arachne-supports-off.json" },
    [pscustomobject]@{ Id = "base-arachne-on";    Model = $BaseModel;   Config = "pnp-arachne-supports-on.json"  }
)

if ($Cells.Count -gt 0) {
    $Cells = @($Cells | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' })
    $Selected = @($AllCells | Where-Object { $Cells -contains $_.Id })
    if ($Selected.Count -ne $Cells.Count) {
        throw "Unknown cell id in $($Cells -join ', ')"
    }
    $AllCells = $Selected
}

$Arms = @{
    "baseline-ordinary" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t182-baseline\pnp_cli-ordinary.exe"
        Mod = Join-Path $RepoRoot "target\t182-baseline\modules"
    }
    "candidate-ordinary" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t182-candidate\pnp_cli-ordinary.exe"
        Mod = Join-Path $RepoRoot "target\t182-candidate\modules"
    }
    "baseline-accelerated" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t182-baseline-accelerated\pnp_cli-accelerated.exe"
        Mod = Join-Path $RepoRoot "target\t182-baseline-accelerated\modules"
    }
    "candidate-accelerated" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t182-candidate-accelerated\pnp_cli-accelerated.exe"
        Mod = Join-Path $RepoRoot "target\t182-candidate-accelerated\modules"
    }
}

function Get-TreeHash {
    param([string]$Dir)
    $files = Get-ChildItem -LiteralPath $Dir -Recurse -File |
        Where-Object { $_.Extension -in @(".wasm", ".json") } |
        Sort-Object FullName
    $lines = foreach ($f in $files) {
        $rel = $f.FullName.Substring($Dir.Length).TrimStart('\', '/') -replace '\\', '/'
        $h = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$h  $rel"
    }
    $joined = ($lines -join "`n") + "`n"
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($joined)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") }) -join ""
    } finally {
        $sha.Dispose()
    }
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class CpuLoadProbeT182 {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GetSystemTimes(out ulong idle, out ulong kernel, out ulong user);
    public static double SamplePct(int ms) {
        ulong i1, k1, u1, i2, k2, u2;
        if (!GetSystemTimes(out i1, out k1, out u1)) return -1.0;
        System.Threading.Thread.Sleep(ms);
        if (!GetSystemTimes(out i2, out k2, out u2)) return -1.0;
        double idle = (double)(i2 - i1);
        double total = (double)((k2 - k1) + (u2 - u1));
        if (total <= 0.0) return 0.0;
        return Math.Round((1.0 - idle / total) * 100.0, 2);
    }
}
"@ -ErrorAction SilentlyContinue

function Get-CpuLoadSample {
    try {
        $samples = Get-CimInstance Win32_Processor -ErrorAction Stop | Measure-Object -Property LoadPercentage -Average
        return [double]$samples.Average
    } catch {
        return [CpuLoadProbeT182]::SamplePct(500)
    }
}

function Write-Manifest {
    New-Item -ItemType Directory -Force -Path $TargetRoot | Out-Null
    $out = [ordered]@{}
    foreach ($name in ($Arms.Keys | Sort-Object)) {
        $arm = $Arms[$name]
        if (-not (Test-Path -LiteralPath $arm.Exe)) {
            $out[$name] = [ordered]@{ exe_path = $arm.Exe; missing = $true }
            continue
        }
        $plannerWasm = Join-Path $arm.Mod "tree-support-planner\tree-support-planner.wasm"
        $plannerSha = if (Test-Path -LiteralPath $plannerWasm) {
            (Get-FileHash -LiteralPath $plannerWasm -Algorithm SHA256).Hash.ToLowerInvariant()
        } else { "" }
        $out[$name] = [ordered]@{
            exe_path            = $arm.Exe
            exe_sha256          = (Get-FileHash -LiteralPath $arm.Exe -Algorithm SHA256).Hash.ToLowerInvariant()
            exe_bytes           = (Get-Item -LiteralPath $arm.Exe).Length
            modules_dir         = $arm.Mod
            modules_tree_sha256 = Get-TreeHash -Dir $arm.Mod
            planner_wasm_sha256 = $plannerSha
        }
    }
    $path = Join-Path $TargetRoot "manifest.json"
    ($out | ConvertTo-Json -Depth 5) | Set-Content -LiteralPath $path -Encoding UTF8
    Write-Host "Wrote $path"
}

function Measure-SliceDirect {
    param(
        [string]$ArmName,
        [string]$ExePath,
        [string]$ModDir,
        [pscustomobject]$Cell,
        [string]$OutBase,
        [int]$Threads = 12,
        [string[]]$ExtraArgs = @()
    )
    $cfg = Join-Path $ConfigDir $Cell.Config
    $gcode = "$OutBase.gcode"
    $stdout = "$OutBase.stdout.log"
    $stderr = "$OutBase.stderr.jsonl"
    Remove-Item -LiteralPath $gcode, $stdout, $stderr -ErrorAction SilentlyContinue

    $prevRayon = $env:RAYON_NUM_THREADS
    $env:RAYON_NUM_THREADS = "$Threads"
    try {
        $psi = [System.Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $ExePath
        foreach ($arg in @(
            "slice",
            "--model", $Cell.Model,
            "--output", $gcode,
            "--config", $cfg,
            "--module-dir", $ModDir,
            "--no-default-module-paths"
        )) {
            [void]$psi.ArgumentList.Add($arg)
        }
        foreach ($arg in $ExtraArgs) {
            [void]$psi.ArgumentList.Add($arg)
        }
        $psi.WorkingDirectory = $RepoRoot
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $proc = [System.Diagnostics.Process]::Start($psi)
        $outTask = $proc.StandardOutput.ReadToEndAsync()
        $errTask = $proc.StandardError.ReadToEndAsync()
        $proc.WaitForExit()
        $sw.Stop()

        [System.IO.File]::WriteAllText($stdout, $outTask.GetAwaiter().GetResult())
        [System.IO.File]::WriteAllText($stderr, $errTask.GetAwaiter().GetResult())

        $wallS = $sw.Elapsed.TotalSeconds
        $cpuS = $proc.TotalProcessorTime.TotalSeconds
        $userS = $proc.UserProcessorTime.TotalSeconds
        $privS = $proc.PrivilegedProcessorTime.TotalSeconds
        $peakBytes = [int64]$proc.PeakWorkingSet64
        $exitCode = $proc.ExitCode
    } finally {
        if ($null -eq $prevRayon) {
            Remove-Item Env:RAYON_NUM_THREADS -ErrorAction SilentlyContinue
        } else {
            $env:RAYON_NUM_THREADS = $prevRayon
        }
    }

    $gcodeSha = ""
    $gcodeBytes = 0L
    if (Test-Path -LiteralPath $gcode) {
        $gcodeSha = (Get-FileHash -LiteralPath $gcode -Algorithm SHA256).Hash.ToLowerInvariant()
        $gcodeBytes = (Get-Item -LiteralPath $gcode).Length
    }

    [pscustomobject]@{
        arm           = $ArmName
        cell          = $Cell.Id
        exit_code     = $exitCode
        wall_s        = [math]::Round($wallS, 6)
        cpu_s         = [math]::Round($cpuS, 6)
        user_s        = [math]::Round($userS, 6)
        privileged_s  = [math]::Round($privS, 6)
        peak_ws_bytes = $peakBytes
        gcode_sha256  = $gcodeSha
        gcode_bytes   = $gcodeBytes
        gcode_path    = $gcode
        stderr_path   = $stderr
        threads       = $Threads
    }
}

function Measure-Slice {
    param(
        [string]$ArmName,
        [pscustomobject]$Cell,
        [string]$OutBase,
        [int]$Threads = 12
    )
    $arm = $Arms[$ArmName]
    Measure-SliceDirect -ArmName $ArmName -ExePath $arm.Exe -ModDir $arm.Mod -Cell $Cell -OutBase $OutBase -Threads $Threads
}

Write-Manifest
if ($Mode -eq "manifest") {
    return
}

if ($Mode -eq "sweep") {
    $sweepDir = Join-Path $TargetRoot "sweep\$BatchId"
    New-Item -ItemType Directory -Force -Path $sweepDir | Out-Null
    $csvPath = Join-Path $sweepDir "rows.csv"
    $metaPath = Join-Path $sweepDir "batch_meta.json"
    if (Test-Path -LiteralPath $csvPath) { Remove-Item -LiteralPath $csvPath -Force }

    $cell = $AllCells | Where-Object { $_.Id -eq "base-classic-on" }
    if (-not $cell) { $cell = $AllCells[0] }
    $expectedSha = $T175Reference[$cell.Id]

    $modDir = $Arms["baseline-ordinary"].Mod
    $modTreeHash = Get-TreeHash -Dir $modDir
    $armExeMap = [ordered]@{}
    $armExeSha = [ordered]@{}
    foreach ($a in $SweepArms) {
        if ($a -eq "baseline") {
            $armExeMap[$a] = $Arms["baseline-ordinary"].Exe
        } else {
            $armExeMap[$a] = Join-Path $RepoRoot "target\t182-sweep\$a\pnp_cli.exe"
        }
        if (-not (Test-Path -LiteralPath $armExeMap[$a])) {
            throw "Missing sweep arm binary: $($armExeMap[$a])"
        }
        $armExeSha[$a] = (Get-FileHash -LiteralPath $armExeMap[$a] -Algorithm SHA256).Hash.ToLowerInvariant()
    }

    $loadBefore = Get-CpuLoadSample
    Write-Host "[sweep $BatchId] CPU load before: $loadBefore%"

    if (-not $NoWarmup) {
        foreach ($a in $SweepArms) {
            Write-Host "[sweep warmup] $a on $($cell.Id)"
            $r = Measure-SliceDirect -ArmName $a -ExePath $armExeMap[$a] -ModDir $modDir -Cell $cell -OutBase (Join-Path $sweepDir "warmup__$a")
            if ($r.exit_code -ne 0) { throw "Warmup non-zero exit ($($r.exit_code)) on $a" }
            $m = ($r.gcode_sha256 -eq $expectedSha)
            if ($a -eq "baseline" -and -not $m) {
                throw "BASELINE OUTPUT MISMATCH on sweep warmup $a : got $($r.gcode_sha256) want $expectedSha"
            }
            Remove-Item -LiteralPath $r.gcode_path -ErrorAction SilentlyContinue
            Write-Host "[sweep warmup] DONE $a wall=$($r.wall_s)s cpu=$($r.cpu_s)s sha=$($r.gcode_sha256.Substring(0,8)) match_t175=$m"
        }
    }

    for ($round = 1; $round -le $SweepRuns; $round++) {
        $order = if (($round % 2) -eq 0) {
            $rev = @($SweepArms)
            [Array]::Reverse($rev)
            $rev
        } else {
            @($SweepArms)
        }
        $pos = 0
        foreach ($a in $order) {
            $pos++
            $tag = "r${round}__$a"
            Write-Host "[sweep $BatchId] round $round/$SweepRuns pos $pos : $a ($($cell.Id))"
            $r = Measure-SliceDirect -ArmName $a -ExePath $armExeMap[$a] -ModDir $modDir -Cell $cell -OutBase (Join-Path $sweepDir $tag)
            if ($r.exit_code -ne 0) { throw "Non-zero exit ($($r.exit_code)) on $tag" }
            $m = ($r.gcode_sha256 -eq $expectedSha)
            if ($a -eq "baseline" -and -not $m) {
                throw "BASELINE OUTPUT MISMATCH on $tag : got $($r.gcode_sha256) want $expectedSha"
            }
            # Keep round 1 G-code for per-arm section delta audit; remove rounds 2+.
            if ($round -gt 1) {
                Remove-Item -LiteralPath $r.gcode_path -ErrorAction SilentlyContinue
            }
            $rowObj = [pscustomobject]@{
                batch               = $BatchId
                round               = $round
                position_in_round   = $pos
                arm                 = $a
                cell                = $cell.Id
                exit_code           = $r.exit_code
                wall_s              = $r.wall_s
                cpu_s               = $r.cpu_s
                user_s              = $r.user_s
                privileged_s        = $r.privileged_s
                peak_ws_bytes       = $r.peak_ws_bytes
                gcode_sha256        = $r.gcode_sha256
                gcode_bytes         = $r.gcode_bytes
                matches_t175        = $m
                exe_sha256          = $armExeSha[$a]
                modules_tree_sha256 = $modTreeHash
                stderr_path         = $r.stderr_path
                threads             = $r.threads
            }
            @($rowObj) | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8 -Append
            Write-Host "[sweep $BatchId] DONE $tag wall=$($r.wall_s)s cpu=$($r.cpu_s)s sha=$($r.gcode_sha256.Substring(0,8)) match_t175=$m"
        }
    }

    $loadAfter = Get-CpuLoadSample
    $meta = [ordered]@{
        batch_id            = $BatchId
        cell                = $cell.Id
        arms                = @($SweepArms)
        runs_per_arm        = $SweepRuns
        modules_tree_sha256 = $modTreeHash
        arm_exe_sha256      = $armExeSha
        cpu_load_before     = $loadBefore
        cpu_load_after      = $loadAfter
        completed_utc       = (Get-Date).ToUniversalTime().ToString("o")
    }
    ($meta | ConvertTo-Json -Depth 4) | Set-Content -LiteralPath $metaPath -Encoding UTF8
    Write-Host "Wrote $csvPath and $metaPath (CPU load after: $loadAfter%)"
    return
}

if ($Mode -eq "output") {
    $outDir = Join-Path $TargetRoot "output\$BatchId"
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    $csvPath = Join-Path $outDir "rows.csv"
    $rows = [System.Collections.Generic.List[object]]::new()
    $armList = if ($Accelerated) {
        @("baseline-accelerated", "candidate-accelerated")
    } else {
        @("baseline-ordinary", "candidate-ordinary")
    }
    foreach ($cell in $AllCells) {
        $expectedSha = $T175Reference[$cell.Id]
        foreach ($armName in $armList) {
            $base = Join-Path $outDir "$($cell.Id)__$armName"
            Write-Host "[output] $($cell.Id) / $armName"
            $r = Measure-Slice -ArmName $armName -Cell $cell -OutBase $base
            $rows.Add($r)
            if ($r.exit_code -ne 0) {
                throw "Non-zero exit ($($r.exit_code)) on $($cell.Id) / $armName"
            }
            $match = ($r.gcode_sha256 -eq $expectedSha)
            Write-Host "[output] DONE $($cell.Id) / $armName wall=$($r.wall_s)s cpu=$($r.cpu_s)s sha=$($r.gcode_sha256.Substring(0,8)) match_t175=$match"
            if ($armName -like "baseline-*" -and -not $match) {
                throw "BASELINE OUTPUT MISMATCH on $($cell.Id) / $armName : got $($r.gcode_sha256) want $expectedSha"
            }
            # Keep ordinary output G-code for delta audit; delete accelerated G-code after hash.
            if ($Accelerated) {
                Remove-Item -LiteralPath $r.gcode_path -ErrorAction SilentlyContinue
            }
        }
    }
    $rows | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8
    Write-Host "Wrote $csvPath"
    return
}

if ($Mode -eq "ab") {
    $abDir = Join-Path $TargetRoot "ab\$BatchId"
    New-Item -ItemType Directory -Force -Path $abDir | Out-Null
    $csvPath = Join-Path $abDir "rows.csv"
    $metaPath = Join-Path $abDir "batch_meta.json"

    $existing = @{}
    if ($Append -and (Test-Path -LiteralPath $csvPath)) {
        foreach ($row in (Import-Csv -LiteralPath $csvPath)) {
            $existing["$($row.pair)|$($row.cell)|$($row.side)"] = $true
        }
        Write-Host "[ab $BatchId] append mode: $($existing.Count) existing row(s)"
    } elseif (Test-Path -LiteralPath $csvPath) {
        Remove-Item -LiteralPath $csvPath -Force
    }

    $armA = if ($Accelerated) { "baseline-accelerated" } else { "baseline-ordinary" }
    $armB = if ($NullControl) { $armA } elseif ($Accelerated) { "candidate-accelerated" } else { "candidate-ordinary" }
    $labelA = "baseline"
    $labelB = if ($NullControl) { "baseline2" } else { "candidate" }

    $loadBefore = Get-CpuLoadSample
    Write-Host "CPU load before batch '$BatchId': $loadBefore%"

    if (-not $NoWarmup -and -not ($Append -and $existing.Count -gt 0)) {
        $warmCell = $AllCells[0]
        Write-Host "[warmup] $($warmCell.Id) ($labelA, $labelB)"
        [void](Measure-Slice -ArmName $armA -Cell $warmCell -OutBase (Join-Path $abDir "warmup__a"))
        [void](Measure-Slice -ArmName $armB -Cell $warmCell -OutBase (Join-Path $abDir "warmup__b"))
        Remove-Item -Path (Join-Path $abDir "warmup__*.gcode") -ErrorAction SilentlyContinue
    }

    $pairLast = if ($PairTo -gt 0) { $PairTo } else { $Pairs }
    for ($p = $PairFrom; $p -le $pairLast; $p++) {
        $even = (($p % 2) -eq 0)
        $order = if ($even) { @(@($armB, $labelB), @($armA, $labelA)) } else { @(@($armA, $labelA), @($armB, $labelB)) }
        $orderTag = if ($even) { "$labelB-$labelA" } else { "$labelA-$labelB" }
        foreach ($cell in $AllCells) {
            $expectedSha = $T175Reference[$cell.Id]
            $pos = 0
            foreach ($pair in $order) {
                $pos++
                $armName = $pair[0]
                $side = $pair[1]
                $key = "$p|$($cell.Id)|$side"
                $tag = "p${p}__$($cell.Id)__${side}"
                if ($OnlyMissing -and $existing.ContainsKey($key)) {
                    Write-Host "[ab $BatchId] SKIP $tag"
                    continue
                }
                $base = Join-Path $abDir $tag
                Write-Host "[ab $BatchId] pair $p/$pairLast ($orderTag) $($cell.Id) $side"
                $r = Measure-Slice -ArmName $armName -Cell $cell -OutBase $base
                if ($r.exit_code -ne 0) {
                    throw "Non-zero exit ($($r.exit_code)) on $tag"
                }
                if ($side -like "baseline*" -and $r.gcode_sha256 -ne $expectedSha) {
                    throw "BASELINE OUTPUT MISMATCH on $tag : got $($r.gcode_sha256) want $expectedSha"
                }
                Remove-Item -LiteralPath $r.gcode_path -ErrorAction SilentlyContinue
                $rowObj = [pscustomobject]@{
                    batch            = $BatchId
                    null_control     = [bool]$NullControl
                    accelerated      = [bool]$Accelerated
                    pair             = $p
                    order            = $orderTag
                    position_in_pair = $pos
                    side             = $side
                    arm              = $armName
                    cell             = $cell.Id
                    exit_code        = $r.exit_code
                    wall_s           = $r.wall_s
                    cpu_s            = $r.cpu_s
                    user_s           = $r.user_s
                    privileged_s     = $r.privileged_s
                    peak_ws_bytes    = $r.peak_ws_bytes
                    gcode_sha256     = $r.gcode_sha256
                    gcode_bytes      = $r.gcode_bytes
                    stderr_path      = $r.stderr_path
                    threads          = $r.threads
                }
                @($rowObj) | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8 -Append
                Write-Host ("[ab $BatchId] DONE $tag wall=$($r.wall_s)s cpu=$($r.cpu_s)s sha=$($r.gcode_sha256.Substring(0,8))")
            }
        }
    }

    $loadAfter = Get-CpuLoadSample
    $prevBefore = $loadBefore
    if ($Append -and (Test-Path -LiteralPath $metaPath)) {
        try {
            $oldMeta = Get-Content -LiteralPath $metaPath -Raw | ConvertFrom-Json
            if ($null -ne $oldMeta.cpu_load_before) {
                $prevBefore = [double]$oldMeta.cpu_load_before
            }
        } catch {}
    }
    $meta = [ordered]@{
        batch_id         = $BatchId
        null_control     = [bool]$NullControl
        accelerated      = [bool]$Accelerated
        pairs            = $Pairs
        cells            = @($AllCells | ForEach-Object { $_.Id })
        cpu_load_before  = $prevBefore
        cpu_load_after   = $loadAfter
        completed_utc    = (Get-Date).ToUniversalTime().ToString("o")
    }
    ($meta | ConvertTo-Json -Depth 4) | Set-Content -LiteralPath $metaPath -Encoding UTF8
    Write-Host "Updated $csvPath and $metaPath (CPU load after: $loadAfter%)"
}
