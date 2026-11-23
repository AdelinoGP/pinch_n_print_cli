# Reproduce the ticket-115 reconciliation

Everything below runs from a plain checkout at the take's HEAD. No gitignored
artifact is required except the two user-supplied fixtures (`tmp/3dbenchy.stl`,
`tmp/base.stl`).

## 1. Verify the committed numbers

```bash
python docs/specs/perf-vs-orca/evidence/t115-post-merge/verify.py --self-test
```

Exits 0 with `PASS: 403 checks; 6/6 negative controls rejected`. The positive
checks re-derive every headline number from `summary.json` and
`ab-rows-classic.csv`; the negative controls mutate the summary in memory and
must be rejected.

## 2. Re-run the paired pre-merge vs merged A/B

The pre-merge arm needs the t103 developer snapshot, which the post-merge
rebuild in this take consumed. Recreate it in a detached worktree at the commit
the t103 rig measured (`c135e3e7`), then stage the ordinary developer dist:

```bash
git worktree add ../pnp-premerge c135e3e7
cd ../pnp-premerge && cargo xtask dist --edition developer
# copy ../pnp-premerge/target/dist/developer over ./target/dist/developer
```

The snapshot binary's SHA-256 must be
`4cb969421ca1193be9501b8147324e2fcf86fbf6dcd42bd5903315cf5844a1e9`, and its
benchy classic-off slice must reproduce `3672dc7e1649cab69a0ffab2ebd293233a9af0504d0f98666746da933d8c641d`.

Then, with the merged tree's `target/release/pnp_cli.exe` built from source:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t115-post-merge/run_merge_ab.ps1 -Pairs 3
```

Rows land in `target/t115-ab/<batch>/ab-rows.csv`. The committed
`ab-rows-classic.csv` is that file for the classic four cells.

## 3. Re-run the instrumented attribution probe

```bash
./target/release/pnp_cli.exe slice --model tmp/base.stl \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules --no-default-module-paths --instrument-stderr \
  --output /tmp/post.gcode 2>/tmp/post.jsonl
```

with the pre-merge snapshot (`target/dist/developer/pnp_cli.exe` +
`target/dist/developer/modules`) for the `pre` arm. The `phase_complete` rows
give the per-layer split; `module_complete` rows give the brackets.

## 4. Re-record the matched-job output references

```bash
./target/release/pnp_cli.exe slice --model tmp/base.stl \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules --no-default-module-paths --output /tmp/base-on.gcode
sha256sum /tmp/base-on.gcode   # 4b6d28434d83f6e347dd353ff0300ee25f2c314b46dd9546b589d9a911438bb1
```

`summary.json` `output_audit` carries the full post-merge set.

## 5. Re-run the painted-fixture audit

```bash
./target/release/pnp_cli.exe slice --model resources/cube_4color.3mf \
  --module-dir modules/core-modules --no-default-module-paths --output /tmp/post-4color.gcode
sha256sum /tmp/post-4color.gcode  # 566730cb53d7e0214edf2e32e0f52bad8574ceb9f27a13c9489a40673b0266b3
```

`painted_seams.3mf` needs `{"seam_mode":"aligned","line_width":0.4}` as its
config; the 3MF's own settings otherwise resolve `line_width = 105`, which is
out of range on both arms (a pre-existing model/settings issue, not a merge
effect).
