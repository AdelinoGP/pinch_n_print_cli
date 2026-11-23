# Launch the T70 campaign driver detached from the agent process tree.
#
# Motivated by the documented eviction-class interruptions (t57 stop/resume
# history item 2; t58 recovery disclosures): an opencode location-services
# eviction / server teardown kills the harness's background shell and every
# process in its tree, aborting an in-flight campaign cell. A Start-Process
# pwsh child with its own console and its own log files survives that teardown
# (t58 precedent: "launched detached from the agent process tree ... only a
# machine reboot or explicit kill can [terminate] it").
#
# The drivers are resumable: completed cells are skipped by run-cell.ps1's
# one-shot guard, and an externally interrupted partial cell must be deleted
# by hand (never auto-retried) before relaunch. This script refuses to
# double-launch if a driver for the same phase is already alive.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('proof','measure')][string]$Phase
)
$ErrorActionPreference = 'Stop'
# Repo-root walk, depth-agnostic (t67's fixed five-up count broke when the T70
# clone added the campaign/ subdir level). See run-cell.ps1 for the rationale.
$Repo = $PSScriptRoot
while (-not (Test-Path (Join-Path $Repo '.local-artifacts'))) {
    $parent = Split-Path $Repo -Parent
    if (-not $parent -or $parent -eq $Repo) { throw 'could not locate repo root (.local-artifacts)' }
    $Repo = $parent
}
$Root = Join-Path $Repo '.local-artifacts\perimeter-reference-preparation\t70-clipper-candidate'
$script = Join-Path $PSScriptRoot "run-all-$Phase.ps1"
$log = Join-Path $Root "$Phase-driver-detached.log"
$err = Join-Path $Root "$Phase-driver-detached.err"
$pidJson = Join-Path $Root "$Phase-driver-detached.pid.json"

$alive = @(Get-CimInstance Win32_Process -Filter "Name='pwsh.exe'" |
    Where-Object { $_.CommandLine -like "*run-all-$Phase.ps1*" })
if ($alive) {
    throw "driver already running for phase '$Phase' (pid $($alive.ProcessId -join ',')); refusing to double-launch"
}

$p = Start-Process pwsh -ArgumentList @('-NoProfile', '-File', $script) `
    -WindowStyle Hidden -RedirectStandardOutput $log -RedirectStandardError $err -PassThru
[pscustomobject]@{
    phase = $Phase
    pid = $p.Id
    started = (Get-Date -Format o)
    log = $log
    err = $err
} | ConvertTo-Json | Set-Content -Path $pidJson -Encoding utf8
Get-Content $pidJson
