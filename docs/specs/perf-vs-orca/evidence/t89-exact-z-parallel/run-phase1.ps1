# t89 phase 1 driver: freshness gates, both snapshot arms, the 16-cell proof
# matrix, and the attribution pair. The paired timing campaign
# (run-all-measure.ps1) is deliberately NOT chained here: it needs an otherwise
# idle machine, which is a separate decision.
#
# Resumable: run-all-proof.ps1 skips completed cells, extract.py/reduce.py are
# idempotent, and snapshot builds overwrite their staged copies.
$ErrorActionPreference = 'Continue'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
Set-Location $Repo
$Script = $PSScriptRoot

function Step([string]$Name, [scriptblock]$Body, [int[]]$Accept = @(0)) {
    Write-Host "[t89-phase1] START $Name $(Get-Date -Format o)"
    & $Body
    $code = $LASTEXITCODE
    Write-Host "[t89-phase1] END $Name exit=$code $(Get-Date -Format o)"
    if ($Accept -notcontains $code) {
        Write-Host "[t89-phase1] ABORT at $Name"
        exit $code
    }
}

Step 'build-guests ordinary' { & cargo xtask build-guests }
Step 'check-guests ordinary (must be 0)' { & cargo xtask build-guests --check }
Step 'build-guests accelerated' { & cargo xtask build-guests --accelerated }
Step 'check-guests accelerated (must be 0)' { & cargo xtask build-guests --accelerated --check }

Step 'snapshots candidate (both modes)' { & pwsh -NoProfile -File (Join-Path $Script 'build-snapshots.ps1') -Arm candidate -Modes both }
Step 'snapshots baseline (both modes)' { & pwsh -NoProfile -File (Join-Path $Script 'build-snapshots.ps1') -Arm baseline -Modes both }

Step 'proof matrix (16 cells x 2 arms)' { & pwsh -NoProfile -File (Join-Path $Script 'run-all-proof.ps1') }
Step 'reduce qualification' {
    & python (Join-Path $Script 'extract.py')
    if ($LASTEXITCODE -eq 0) { & python (Join-Path $Script 'reduce.py') --raw }
}

Step 'attribution: baseline base classic on' {
    & pwsh -NoProfile -File (Join-Path $Script 'run-instrumented.ps1') `
        -Arm baseline -Mode ordinary -Fixture base -Generator classic -Supports on -Tag attribution
}
Step 'attribution: candidate base classic on' {
    & pwsh -NoProfile -File (Join-Path $Script 'run-instrumented.ps1') `
        -Arm candidate -Mode ordinary -Fixture base -Generator classic -Supports on -Tag attribution
}
Step 'reduce attribution' { & python (Join-Path $Script 'extract.py') }

Write-Host '[t89-phase1] complete'
exit 0
