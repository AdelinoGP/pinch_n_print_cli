$h='tmp/alloc-bench/run_bench.ps1'
$batch=Get-Date -Format 'yyyyMMdd-HHmmss'
$runDir="tmp/perf-flags/quiet-validation/validated-runs/$batch"
$csv=Join-Path $runDir 'measured.csv'
$configs=@{classic='tmp/perf-flags/quiet-validation/benchy-supports-off-classic.json';arachne='tmp/perf-flags/quiet-validation/benchy-supports-off-arachne.json'}
# ABBA BAAB: four observations per side, balanced by position.
$schedule=@('baseline','candidate','candidate','baseline','candidate','baseline','baseline','candidate')
foreach ($gen in @('classic','arachne')) {
    $cfg=$configs[$gen]; $i=0
  foreach ($side in $schedule) {
    $i++
    $exe=if($side -eq 'baseline'){'tmp/perf-flags/baseline-artifacts/pnp_cli.exe'}else{'target/release/pnp_cli.exe'}
    $mods=if($side -eq 'baseline'){'tmp/perf-flags/baseline-artifacts/core-modules'}else{'modules/core-modules'}
    $label="validated-$gen-$side-$i"
    $out=Join-Path $runDir "$label.gcode"
    & pwsh -NoProfile -File $h -ExePath $exe -InputModel 'tmp/3dbenchy.stl' -Config $cfg -ModuleDir $mods -OutputPath $out -Threads 12 -Label $label -ResultsPath $csv -ExpectedGenerator $gen
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  }
}
