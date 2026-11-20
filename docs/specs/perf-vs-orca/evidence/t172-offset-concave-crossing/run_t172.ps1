param(
    [ValidateSet("manifest", "output", "ab")]
    [string]$Mode = "output",
    [string[]]$Cells = @(),
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
$TargetRoot = Join-Path $RepoRoot "target\t172"
$ConfigDir = Join-Path $RepoRoot "docs\specs\perf-vs-orca\evidence\matched-pair\configs"
$BenchyModel = Join-Path $RepoRoot "tmp\3dbenchy.stl"
$BaseModel = Join-Path $RepoRoot "tmp\base.stl"

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
    "baseline-ordinary"  = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t172-baseline\pnp_cli-ordinary.exe"
        Mod = Join-Path $RepoRoot "target\t172-baseline\modules"
    }
    "candidate-ordinary" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t172-candidate\pnp_cli-ordinary.exe"
        Mod = Join-Path $RepoRoot "target\t172-candidate\modules"
    }
    "baseline-accelerated"  = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t172-baseline-accelerated\pnp_cli-accelerated.exe"
        Mod = Join-Path $RepoRoot "target\t172-baseline-accelerated\modules"
    }
    "candidate-accelerated" = [pscustomobject]@{
        Exe = Join-Path $RepoRoot "target\t172-candidate-accelerated\pnp_cli-accelerated.exe"
        Mod = Join-Path $RepoRoot "target\t172-candidate-accelerated\modules"
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

function Get-CpuLoadSample {
    $samples = Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average
    [double]$samples.Average
}

function Write-Manifest {
    New-Item -ItemType Directory -Force -Path $TargetRoot | Out-Null
    $out = [ordered]@{}
    foreach ($name in ($Arms.Keys | Sort-Object)) {
        $arm = $Arms[$name]
        if (-not (Test-Path -LiteralPath $arm.Exe)) {
            Write-Host "manifest: arm '$name' not staged yet ($($arm.Exe))"
            $out[$name] = [ordered]@{ exe_path = $arm.Exe; missing = $true }
            continue
        }
        $out[$name] = [ordered]@{
            exe_path            = $arm.Exe
            exe_sha256          = (Get-FileHash -LiteralPath $arm.Exe -Algorithm SHA256).Hash.ToLowerInvariant()
            exe_bytes           = (Get-Item -LiteralPath $arm.Exe).Length
            modules_dir         = $arm.Mod
            modules_tree_sha256 = Get-TreeHash -Dir $arm.Mod
        }
    }
    $path = Join-Path $TargetRoot "manifest.json"
    ($out | ConvertTo-Json -Depth 5) | Set-Content -LiteralPath $path -Encoding UTF8
    Write-Host "Wrote $path"
}

function Measure-Slice {
    param(
        [string]$ArmName,
        [pscustomobject]$Cell,
        [string]$OutBase,
        [int]$Threads = 12
    )
    $arm = $Arms[$ArmName]
    $cfg = Join-Path $ConfigDir $Cell.Config
    $gcode = "$OutBase.gcode"
    $stdout = "$OutBase.stdout.log"
    $stderr = "$OutBase.stderr.jsonl"
    Remove-Item -LiteralPath $gcode, $stdout, $stderr -ErrorAction SilentlyContinue

    $prevRayon = $env:RAYON_NUM_THREADS
    $env:RAYON_NUM_THREADS = "$Threads"
    try {
        $psi = [System.Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $arm.Exe
        foreach ($arg in @(
            "slice",
            "--model", $Cell.Model,
            "--output", $gcode,
            "--config", $cfg,
            "--module-dir", $arm.Mod,
            "--no-default-module-paths"
        )) {
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

Write-Manifest
if ($Mode -eq "manifest") {
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
        foreach ($armName in $armList) {
            $base = Join-Path $outDir "$($cell.Id)__$armName"
            Write-Host "[output] $($cell.Id) / $armName"
            $r = Measure-Slice -ArmName $armName -Cell $cell -OutBase $base
            $rows.Add($r)
            if ($r.exit_code -ne 0) {
                throw "Non-zero exit ($($r.exit_code)) on $($cell.Id) / $armName"
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
                Write-Host ("[ab $BatchId] DONE $tag wall=$($r.wall_s)s cpu=$($r.cpu_s)s")
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
        pairs            = $Pairs
        cells            = @($AllCells | ForEach-Object { $_.Id })
        cpu_load_before  = $prevBefore
        cpu_load_after   = $loadAfter
        completed_utc    = (Get-Date).ToUniversalTime().ToString("o")
    }
    ($meta | ConvertTo-Json -Depth 4) | Set-Content -LiteralPath $metaPath -Encoding UTF8
    Write-Host "Updated $csvPath and $metaPath (CPU load after: $loadAfter%)"
}
