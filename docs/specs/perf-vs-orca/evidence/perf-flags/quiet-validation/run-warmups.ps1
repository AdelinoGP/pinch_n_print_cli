$h='tmp/alloc-bench/run_bench.ps1'
$batch=Get-Date -Format 'yyyyMMdd-HHmmss'
$runDir="tmp/perf-flags/quiet-validation/validated-runs/warmups-$batch"
$csv=Join-Path $runDir 'warmups.csv'
$configs=@{classic='tmp/perf-flags/quiet-validation/benchy-supports-off-classic.json';arachne='tmp/perf-flags/quiet-validation/benchy-supports-off-arachne.json'}
$jobs=@(
 @('baseline','classic'), @('candidate','classic'), @('baseline','arachne'), @('candidate','arachne')
)
foreach ($j in $jobs) {
  $side=$j[0]; $gen=$j[1]
  $exe=if($side -eq 'baseline'){'tmp/perf-flags/baseline-artifacts/pnp_cli.exe'}else{'target/release/pnp_cli.exe'}
  $mods=if($side -eq 'baseline'){'tmp/perf-flags/baseline-artifacts/core-modules'}else{'modules/core-modules'}
  $cfg=$configs[$gen]
  $label="warmup-$gen-$side"
  $out=Join-Path $runDir "$label.gcode"
  & pwsh -NoProfile -File $h -ExePath $exe -InputModel 'tmp/3dbenchy.stl' -Config $cfg -ModuleDir $mods -OutputPath $out -Threads 12 -Warmup -Label $label -ResultsPath $csv -ExpectedGenerator $gen
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
