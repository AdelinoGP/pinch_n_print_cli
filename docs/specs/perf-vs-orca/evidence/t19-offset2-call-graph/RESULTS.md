# Ticket 19 — narrow-island opening-emptiness short-circuit

Route ticket: [offset2_ex call reduction](../../issues/19-offset2-ex-call-reduction.md).
Call-graph re-derivation: [FINDINGS.md](FINDINGS.md).

Status: timing campaign complete; see the verdict table below.

## What was measured

The ticket's three named candidate shapes were dead or not output-preserving
(see the re-derivation note). The take evaluated the one surviving measured
shape: `classify_narrow_island`'s two-pass `opening_ex`, whose **result geometry
is discarded** — the function reads only `opened.is_empty()`. Replacing it with
the shared single-erosion emptiness predicate halves that call site's
`offset2_ex` traffic (720 → 480 calls on the route job) and cuts module fuel by
≈ 2.35% in the attribution probe.

## Change and exactness

`opening_is_empty` (`crates/slicer-core/src/polygon_ops.rs`) answers
`opening_ex(polys, d, ..).is_empty()` without materialising the dilation. It
shares `offset2_ex`'s first pass verbatim through `offset2_ex_first_pass`, the
same helper the two-pass entry point now calls, so the erosion it observes is
byte-identical to the one the full opening would have started from — the same
primitive, not a re-parameterised copy.

For `delta_mm > 0` the identity is `opening(X, d) = ∅ ⟺ erode(X, d) = ∅`: a
positive polygon offset maps `∅ → ∅` and contains its input on non-empty
inputs, so `dilate(Y) = ∅ ⟺ Y = ∅`. For `delta_mm <= 0` or non-finite the
predicate delegates to the unchanged `opening_ex`. Two measured boundaries make
that delegation load-bearing rather than defensive:

- At `delta_mm == 0`, `inflate_paths_64` short-circuits to a **clone**, so the
  first pass skips the union cleanup the real second pass still runs; a
  degenerate (zero-area) ring survives the clone (non-empty) yet is dropped by
  the opening (empty). `smaller_perimeter_threshold_mm` admits 0.0 (manifest
  minimum), so this is reachable.
- At `delta_mm < 0` the passes have opposite signs and the second can empty a
  non-empty first pass on the same degenerate ring.

`classify_narrow_island`
(`modules/core-modules/classic-perimeters/src/lib.rs`) calls the predicate with
exactly its old arguments (`threshold_mm / 2` as the distance, `CoreJoin::Miter`,
`perimeter_arc_tolerance` as the miter limit — the callers' historical
parameterisation). No other call site changed; `opening_ex` still routes through
`offset2_ex`, and the gap-fill band and thin-wall sites are untouched.

## Independent exactness

`crates/slicer-core/tests/opening_emptiness_tdd.rs` (4 tests):

- the predicate agrees with the unchanged two-pass `opening_ex` on a 14-fixture
  family carrying **both** polarities (a constant predicate fails);
- for positive distance it equals independently extracted erosion emptiness
  (`offset_with_miter_limit`, tree-based extraction) — not the refactored
  function as its own oracle;
- a non-empty erosion at positive distance never opens to nothing;
- the `delta <= 0` delegation is pinned with the degenerate fixtures where raw
  erosion emptiness would give the wrong answer.

`narrow_island_classification_selects_outer_wall_width_in_both_polarities`
(`modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`)
exercises the live `run_perimeters` wiring: a 0.3 mm rib must take
`smaller_perimeter_line_width`, a 4 mm island must keep `outer_wall_line_width`.
Inverting the predicate in the guest makes it red (checked, then reverted), so
it is falsifiable rather than decorative.

## Standing paired A/B

Settings are the map's matched classic supports-off Benchy job
(`evidence/matched-pair/configs/pnp-classic-supports-off.json`): 0.4 mm nozzle,
0.2 mm layers, 2 walls, classic, 20% gyroid, supports off. Twelve threads, one
excluded warmup per arm, six alternating AB/BA measured pairs per mode.
External-only, probe-free, uninstrumented subprocesses; `GetProcessTimes`
charges total creation-to-exit wall and kernel+user CPU.

Starvation policy was declared before measurement in [`protocol.json`](protocol.json):
below 0.75 × best CPU/wall **within the same mode and arm**, exclude the whole
pair; require at least five retained pairs per mode; no automatic resampling.
Separate arm references avoid mistaking a changed work mix for starvation. **No
pair was excluded in either mode.**

| mode | baseline median wall | candidate median wall | wall reduction | baseline median CPU | candidate median CPU | CPU reduction | retained |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary | 16.980 s | 16.870 s | **0.65%** | 120.086 s | 117.586 s | **2.08%** | 6 / 6 |
| accelerated | 15.708 s | 15.478 s | **1.47%** | 99.500 s | 98.469 s | **1.04%** | 6 / 6 |

Paired median candidate-minus-baseline deltas: ordinary wall −0.343 s / CPU
−2.305 s; accelerated wall −0.248 s / CPU −1.063 s. Both metrics favor the
candidate in the medians in every mode, which is the protocol's keep clause.
Wall improves in 4 of 6 ordinary pairs and 6 of 6 accelerated pairs; CPU in
6 of 6 ordinary and 5 of 6 accelerated pairs. Per-sample CPU/wall ranges:
ordinary baseline 6.183–7.201, candidate 6.720–7.178; accelerated baseline
6.257–6.375, candidate 6.281–6.417. Only accelerated wall shows strict range
separation; the ordinary ranges overlap. The effect is small — it cannot close
this destination's 6.28–26.24x gap — but the standing gate is directional and it
passes in both modes. Source: [`summary.json`](summary.json),
[`evidence.json`](evidence.json).

### Output disclosure

Every proof, warmup and measured output in both arms/modes:

- SHA-256 `34886ccb56b214d57c28aa58bdcc8d371a41f440200f5b49276b96b53e18cc40`;
  **4,332,737 bytes**, byte-identical to the preserved job reference and to the
  pre-candidate baseline.
- Validated classic marker; `status=ok`, `degraded=false`, zero fatal/non-fatal
  errors; zero integrated-shadow warnings and no instrumentation events/probes;
  zero existing open-loop warnings on this job.
- TYPE counts: Skirt 1, Brim 1, Outer wall 240, Inner wall 240, Gap infill 110,
  Bottom surface 24, Bridge 22, Sparse infill 204, Internal solid infill 13,
  Top surface 70, Internal Bridge 12.

### Verdict

**KEEP — approved by the human 2026-10-08 and committed with this ticket.** Both
modes cleared the protocol's keep clause with agreeing wall/CPU medians and no
starvation exclusions. This is a call-volume reduction with a verified (small)
wall win, not a gap-closer. No default-mode switch and no adoption-campaign
retry is part of this keep.

## Source-form provenance

The candidate source fails `clippy::neg_cmp_op_on_partial_ord` on its deliberate
`!(delta_mm > 0.0)` NaN-inclusive test (NaN must take the delegate; writing only
`delta_mm <= 0.0` would let NaN reach the fast path). Measured on this tree with
[`source-form-provenance.py`](source-form-provenance.py):

| shipped form | clippy `-D warnings` | rebuilt ordinary wasm | rebuilt accelerated wasm |
| --- | --- | --- | --- |
| measured source exactly | **fails** | `f67d0856…` == timed | `332c4191…` == timed |
| + `#[allow]` on the function | passes | `f67d0856…` == timed | `0e5679aa…` ≠ timed |
| positive-first rewrite | passes | `18c6a387…` ≠ timed | `9fac4788…` ≠ timed |
| + manifest lint allow (**shipped**) | passes | `f67d0856…` == timed | `332c4191…` == timed |

**Any text change to `polygon_ops.rs` shifts the accelerated guest's artifact
hash**, including a comment insertion, because the module carries
`.llvm.<hash>` symbol suffixes in its name section; the ordinary artifact is
text-insensitive. Preserving the timed artifact identity therefore requires
shipping the measured source bytes unchanged, which is why the lint is allowed
at `Cargo.toml [workspace.lints.clippy]` rather than rewritten in place. Both
timed artifacts reproduce byte-for-byte from the shipped tree under forced
rebuilds, and full shipped builds (host + guest) of both modes slice the job to
the same `34886ccb…` reference.

## Validation and reproduction

| command | result |
| --- | --- |
| `cargo test -p slicer-core --test opening_emptiness_tdd --test offset2_ex_tdd --test offset2_ex_collapse_tdd` | 8 passed |
| `cargo test -p classic-perimeters --test classic_perimeters_tdd --test classic_perimeters_concave_tdd --test wall_closure_tdd --test boundary_paint_tdd --lib` | 33 passed (20 + 7 + 3 + 2 + 1) |
| `cargo build --workspace` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 (dependency future-incompatibility advisory only) |
| `cargo xtask check-literals` | 0 violations |
| `cargo xtask check-test-quality --report` (touched files) | 0 findings |
| `cargo xtask build-guests --check` and `--accelerated --check` | both exit 0 |
| full shipped builds, both modes, matched job | byte-identical `34886ccb…` |

The falsification check for the guest test inverts the predicate in
`classify_narrow_island` and observes it go red; the source was restored
afterwards (no mutation is present in the shipped tree). Verify the headline
claims using only tracked files:

```bash
python docs/specs/perf-vs-orca/evidence/t19-offset2-call-graph/verify-t19.py
```

Optional raw captures live under the gitignored `target/t19-ab/campaign/`. To
re-reduce them without launching any slices:

```bash
python docs/specs/perf-vs-orca/evidence/t19-offset2-call-graph/verify-t19.py --reduce target/t19-ab/campaign --write-summary
```

Five negative controls were run against the verifier (not saved as fixtures):
a candidate-wall tamper flips the gate, a CPU-only win is rejected, starvation
pair loss is rejected, output drift hard-fails, and a fabricated reference
hard-fails.
