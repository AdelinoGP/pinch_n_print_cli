#!/bin/bash
# t65 oracle capture driver: runs the probed candidate over the oracle cells,
# capturing instrument-stderr (T65-PROBE counters) + gcode outputs.
set -u
cd /d/slicerProject/pinch_n_print_cli_2
ORACLE=target/t65/oracle
mkdir -p "$ORACLE"

run_cell () {
  local name="$1" model="$2" config="$3" dist="$4" extra="$5"
  local dir="$ORACLE/$name"
  rm -rf "$dir"; mkdir -p "$dir"
  RAYON_NUM_THREADS=12 "$dist/pnp_cli.exe" slice \
    --model "$model" \
    --output "$dir/out.gcode" \
    --config "docs/specs/perf-vs-orca/evidence/matched-pair/configs/$config" \
    --module-dir "$dist/modules" \
    --no-default-module-paths --no-integrated-modules \
    --instrument-stderr $extra \
    2> "$dir/stderr.jsonl"
  local code=$?
  echo "$name slice_exit=$code sha=$(sha256sum "$dir/out.gcode" 2>/dev/null | cut -d' ' -f1)"
  if [ "$extra" = "--profile-verbose" ]; then
    mv "$dir/stderr.jsonl" "$dir/profile.jsonl"
  fi
}

ORD=target/t65/probe-ordinary
ACC=target/t65/probe-accelerated
B=tmp/3dbenchy.stl
BASE=tmp/base.stl
CFG=docs/specs/perf-vs-orca/evidence/matched-pair/configs

# benchy x {classic, arachne} x {off, on} x {ordinary, accelerated}
run_cell b-c-off "$B" pnp-classic-supports-off.json "$ORD" ""
run_cell b-c-on "$B" pnp-classic-supports-on.json "$ORD" ""
run_cell b-a-off "$B" pnp-arachne-supports-off.json "$ORD" ""
run_cell b-a-on "$B" pnp-arachne-supports-on.json "$ORD" ""
run_cell b-c-off-acc "$B" pnp-classic-supports-off.json "$ACC" ""
run_cell b-c-on-acc "$B" pnp-classic-supports-on.json "$ACC" ""
run_cell b-a-off-acc "$B" pnp-arachne-supports-off.json "$ACC" ""
run_cell b-a-on-acc "$B" pnp-arachne-supports-on.json "$ACC" ""

# base coverage (ordinary; the bigger boundaries + heavier invocations)
run_cell base-c-off "$BASE" pnp-classic-supports-off.json "$ORD" ""
run_cell base-a-off "$BASE" pnp-arachne-supports-off.json "$ORD" ""
run_cell base-a-on "$BASE" pnp-arachne-supports-on.json "$ORD" ""

# fuel-scope runs (batch vs per-path clip fuel, same real inputs)
run_cell b-a-off-fuel "$B" pnp-arachne-supports-off.json "$ORD" "--profile --profile-verbose"
run_cell b-c-off-fuel "$B" pnp-classic-supports-off.json "$ORD" "--profile --profile-verbose"
run_cell base-a-off-fuel "$BASE" pnp-arachne-supports-off.json "$ORD" "--profile --profile-verbose"

echo ALL_ORACLE_CELLS_DONE