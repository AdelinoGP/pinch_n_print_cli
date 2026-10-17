# T69 host engine-input capture

**Complete:** both matched outputs pass their preserved t58 byte-identity gates;
the corpus verifier passes; the probe is removed; guest freshness exits 0 before
and after capture. This was recording only, not an optimization or timing campaign.

## Method

`probe-t69-dump.patch` brackets classic-perimeters dispatches with a thread-local
owner/layer recorder, notes each `hs::Host::offset_polygons` /
`hs::Host::clip_polygons` call (`crates/slicer-wasm-host/src/host.rs`), and
records the actual flattened integer paths and parameters immediately before
the native engine invocation in `inflate_once` / `clip_polygons`
(`crates/slicer-core/src/polygon_ops.rs`). Recording never mutates those paths.
The recorder is compiled only for non-wasm targets. No guest module source,
WIT, engine dependency, or standing `target/dist-accelerated/` snapshot is edited.

Classic-perimeters services and engine invocations have a census entry, and
the reducer checks the total against the matched-job t68 census. Geometry capture uses
uniform layers divisible by 20 plus existing layers 239, 270, 494. The reducer
retains whole sampled layers within each of the classic and optional RP
families. It prefers the smallest classic stride, then the smallest RP stride
that fits, from power-of-two multiples of 20 through 1280, with a geometry
budget reserving documentation space below the 20,000,000-byte total ceiling.
This explicitly samples shapes, not
frequency-weighted timing. Parameters are engine values, not pre-conversion
WIT values; f64 values are serialized as unsigned integer bit patterns.

The complete dispatch service signature, rather than sequence alone, resolves
A0..K. A collapsed inner inset skips the B/C/D trio and shifts later service
indices. Unsupported signatures stop reduction rather than silently mislabeling
cases. Empty/zero offsets that return before invoking the engine are counted
as services but do not synthesize nonexistent engine calls.

The best-effort region-preparation leg brackets actual derivations entered
from `LayerArena::ensure_prepared_regions` (RP1) and
`LayerArena::ensure_prepared_perimeter_source_regions` (RP2)
(`crates/slicer-runtime/src/blackboard.rs`), recording their nested native
offset/boolean invocations through the same engine-boundary recorder.
Ticket 67 shares these preparations on identity projections: an RP2-initiated
derivation can populate the ordinary cache, so two tags do not imply two
independent derivations per layer.

The optional RP leg is a retained per-owner/per-layer snapshot, not a proof of
an exhaustive preparation census: repeated derivations of the same owner and
layer overwrite that owner's earlier snapshot. Its raw counts mean invocations
in retained snapshots. The classic-site census has separate matched-job total
checks; no RP counts are used as frequency weights or timing denominators.

## Commands

From the workspace root, before applying the probe:

```bash
cargo xtask build-guests --check
git apply docs/specs/perf-vs-orca/evidence/t69-clipper-engine/probe-t69-dump.patch
```

Build the ordinary release target, preserving an existing CLI first. Copy the
probe executable to the gitignored scratch build directory and run only that
copy. The capture build uses the ordinary release cache, not a dist snapshot:

```bash
mkdir -p .local-artifacts/perf-vs-orca/t69-clipper-engine/build
if [ -f target/release/pnp_cli.exe ]; then cp -f target/release/pnp_cli.exe .local-artifacts/perf-vs-orca/t69-clipper-engine/build/pnp_cli-before.exe; fi
set -o pipefail
cargo build --release --bin pnp_cli 2>&1 | tee .local-artifacts/perf-vs-orca/t69-clipper-engine/raw/build.log
cp -f target/release/pnp_cli.exe .local-artifacts/perf-vs-orca/t69-clipper-engine/build/pnp_cli.exe
pwsh -NoProfile -NonInteractive -File docs/specs/perf-vs-orca/evidence/t69-clipper-engine/run-t69.ps1 2>&1 | tee .local-artifacts/perf-vs-orca/t69-clipper-engine/raw/run.log
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/reduce.py
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/verify-capture.py --raw .local-artifacts/perf-vs-orca/t69-clipper-engine/raw
```

The PowerShell runner records exact CLI commands in each job's metadata,
uses the t68 matched configs, external module provenance preflight,
`--module-dir modules/core-modules --no-default-module-paths --no-integrated-modules`,
`RAYON_NUM_THREADS=12`, `--instrument-stderr --profile --profile-verbose`, and
the existing `Test-MeasurementEvidence` gate. Its reference hashes are re-read
from t58 `qualification.json`. A job can be skipped only if its metadata,
census, and correctly hashed output already exist. Each CLI child has an
explicit timeout. Long build/capture commands run detached with log capture.

After capture, reverse only the preserved probe patch, restore the prior
ordinary release executable (or remove the probe executable if none existed),
and run the freshness check again. No tests or guest rebuilds are required or
run by this capture workflow.

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t69-clipper-engine/probe-t69-dump.patch
cargo xtask build-guests --check
git status --short -- crates modules
```

See `corpus/t68sites/README.md` and `summary.json` for engine enum mappings,
site mappings, exhaustive versus sampled counts, selected layers, and sizes.
`verify-capture.py` validates the in-repo ASCII corpus and preserved identity
evidence without raw files; `--raw` also re-hashes the actual captured G-code,
checks the selected raw-input hashes, and proves corpus geometry/parameter
bytes equal the raw dump after changing only each case's resolved site label.

## Captured results

The ordinary release build and both job processes exited 0. External-module
diagnosis passed. Both jobs passed `Test-MeasurementEvidence`: status `ok`,
`degraded=false`, zero fatal errors, zero non-fatal errors, classic generator
marker and claim-holder checks passing. Actual G-code hashes were independently
re-read by the reducer and the verifier against committed t58 qualification.

| Job | G-code bytes | Actual SHA-256 = preserved t58 reference | Identity |
| --- | ---: | --- | --- |
| benchy-classic-off | 4,332,737 | `34886ccb56b214d57c28aa58bdcc8d371a41f440200f5b49276b96b53e18cc40` | byte-identical |
| base-classic-off | 22,428,863 | `20e7204cfdbb813f642e7e1669d5ba1e30ed1d0ff7f90635af95d967196975ac` | byte-identical |

Probe executable SHA-256:
`a2e64967b655639abea78da75a3a1146307f7a647de192bd01a21d256f31147f`.
Both jobs used this same executable. The workspace lock resolves
`clipper2-rust` 1.1.0, crates.io checksum
`0fd663fe209e7030c956e3be4c051dcc20cdb73da794f31466762cff12ca11bf`.

### Inventory and sampling

Final corpus: `corpus/t68sites/`, **16,984,733 bytes total**, including its
README and summary, below the 20,000,000-byte ceiling.

| File | Bytes | Engine cases |
| --- | ---: | ---: |
| `benchy-t68sites.corpus` | 2,124,517 | 75 |
| `base-t68sites.corpus` | 8,241,039 | 170 |
| `benchy-region-prep.corpus` | 675,472 | 6 |
| `base-region-prep.corpus` | 5,725,950 | 28 |
| `README.md` | 6,893 | — |
| `summary.json` | 210,862 | — |

Raw geometry/census directories contain 926 files / 9,038,848 bytes for benchy
and 1,993 files / 61,981,830 bytes for base. These sizes exclude sibling logs,
G-code, and job metadata; they measure the recorder's `.case` / `.calls` files.

The classic-priority budget rule chose classic stride **40** and RP stride
**320**. Existing layers 239, 270, 494 are retained additionally in each family.
Complete selected layers:

- Benchy classic: `0, 40, 80, 120, 160, 200, 239`; RP: `0, 239`.
- Base classic: `0, 40, 80, 120, 160, 200, 239, 240, 270, 280, 320, 360, 400, 440, 480, 494`;
  RP: `0, 239, 270, 320, 494`.

The raw geometry sampler was stride 20 plus the same additional layer IDs;
the full classic service census covers every layer. The budget chooses whole
layers, never partial vertex lists or only cheap calls. This is not a complete
all-layer geometry dump and must not be treated as a frequency-weighted timing
sample. The exhaustive service census totals are 2,640 benchy / 5,442 base;
actual engine invocations are 2,614 / 5,439 because the G-site empty-input
shortcut bypasses the engine on 26 / 3 layers. Per-site service, actual engine,
raw geometry, and reduced case counts are in `corpus/t68sites/README.md` and
are machine-readable in `summary.json`.

Important attribution correction: the short service signature on base layer
270 is `A0,A1,E,F,G,H,J,K`: B/C/D are skipped after A1 collapses. Blindly using
host sequence as a site ID would shift later labels and incorrectly place the
missing calls at H/J/K. The reducer validates the complete signature before
resolving labels. No historical t68 artifact was changed.

### Best-effort RP outcome and limitations

RP1 and RP2 were reachable and were captured. In this checkout, every retained
RP invocation is opcode 3 (intersection); no RP offset/closing invocation reached
the instrumented native boundary. Each tag has 661 raw snapshot calls / 3 reduced
cases on benchy and 1,926 raw snapshot calls / 14 reduced cases on base. Thus this
leg supplies real current preparation inputs but does **not** recreate the old
t60 closing/offset cost object. No extra prepass instrumentation was forced in.

RP counts have the retained-per-owner/layer snapshot limitation described above;
they are not an exhaustive census across repeated derivations and are not timing
denominators. The same limitation does not silently weaken the classic-site
gate: its complete service totals are explicitly checked against the preserved
matched-job census, and every classic site is present in both reduced jobs.

### Validation and teardown proof

- Before probe edits: `cargo xtask build-guests --check` exited **0**. The checker
  printed no success text; the observed exit code is retained in raw
  `capture-inputs.json`. No guest rebuild was run.
- `verify-capture.py --raw .local-artifacts/perf-vs-orca/t69-clipper-engine/raw`
  exited **0**, validating all 279 final cases, both G-code reference hashes,
  file hashes/sizes, v1 grammar and per-site counts, plus exact raw-to-corpus
  geometry/parameter bytes after changing only the resolved site label.
- Python/PowerShell syntax checks passed; the parser accepted a valid fixture
  and rejected an invalid argument ID. No `cargo test` invocation was run.
- The complete `probe-t69-dump.patch` was reverse-apply checked, then removed
  with `git apply --reverse`. Forward `git apply --check` passed afterwards,
  proving the saved patch remains re-applicable against the restored tree.
- `git status --short -- crates modules` is empty; the temporary
  `crates/slicer-core/src/t69_probe.rs` is absent. No production source remains
  modified by this probe. Unrelated issue/evidence work was preserved.
- After removal: `cargo xtask build-guests --check` exited **0**, recorded in
  raw `freshness-after.log`. The classic guest hash stayed
  `53d2a4f610f318e8c98cee12f640f506220fdf4e514b7463e99b1dd9fc9ed745`.
- The probe-built `target/release/pnp_cli.exe` was removed and the pre-existing
  executable restored, SHA-256
  `c3584f311605f0ef014898a568eb237d971aef5e962ccf104625ad1cb5760497`.
  There was no matching probe executable under `target/release/deps/`.
  No dist snapshot was built, copied into, or purged. Only the clearly labeled
  gitignored scratch executable under `.local-artifacts/.../build/` remains.
  Intermediate Cargo cache artifacts were not purged; no post-revert rebuild
  of `pnp_cli` or guests was run.
- Global instructions require a recovery stash before removing code. A stash
  was created with `git stash create` and retained with `git stash store`,
  without changing the working tree or staging files. Its object ID is in raw
  `recovery-stash.txt`; no stash was deleted. No `git add` or `git commit` ran.

The probe touches exactly these production paths: `crates/slicer-core/src/lib.rs`,
`crates/slicer-core/src/polygon_ops.rs`, `crates/slicer-core/src/t69_probe.rs`
(temporary new file), `crates/slicer-runtime/src/blackboard.rs`,
`crates/slicer-runtime/src/layer_executor.rs`, and
`crates/slicer-wasm-host/src/host.rs`.

Raw `capture-inputs.json`, the two job JSON files, `diagnose.json`, `build.log`,
`run.log`, `reduce.log`, `verify.log`, `freshness-after.log`, and `teardown.json`
retain detailed local provenance. `corpus/t68sites/summary.json` carries the
portable job metadata and selected raw-source hashes in-repo.
