# Clipper engine vendoring and hot-path optimization

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned — added on the human's direction 2026-10-16 (same
session as the t68 resolution: the round-trip bucket measured 97–99 % native
clipper ops). A take requires the human's route decision.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Does bringing the clipper2 engine in-tree (`clipper2-rust` 1.1.0, the
production polygon engine) and optimizing its hot paths yield a byte-exact,
gate-passing reduction on the matched jobs — and if so, where does the
headroom actually live?

Context: the engine is the largest measured cost object in the base cells.
The t60 host:native capture's `polygon_ops` scope totals sum to ≈1,055 s
accumulated on base-off alone (`offset` 482.9 s / `closing_ex` 374.5 s /
`clip_polygons` 197.9 s, all owners — re-derived from
`evidence/t60-host-native/captures/base-classic-off-profile.json`), and the
t68 take measured that 97.55 % / 99.02 % of the classic-perimeters per-layer
round-trip bucket is these calls themselves
([findings](../evidence/t68-host-roundtrips/FINDINGS.md)). Every previous
ticket optimized how the engine is *used* ([clip-universe
hoist](44-clip-universe-hoist.md), [batch-execute](65-infill-linker-batch-execute-experiment.md),
[carve gate](32-emit-carve-gate-representation-and-ab.md)); the engine
*itself* has never been touched.

## Notes

- **Dependency posture (re-derive at take time).** The workspace `Cargo.toml`
  declares `clipper2-rust = "1.1.0"` (crates.io; the lock resolves 1.1.0,
  checksum `0fd663fe…`). Consumers: `crates/slicer-core` (`polygon_ops.rs` —
  host and guest; `triangle_mesh_slicer.rs`) and the guest modules
  transitively — the crate compiles into the wasm guests too, so any engine
  change is a guest rebuild + both freshness gates + byte-identity
  qualification on both legs. Version posture is settled: stay on the 1.1.0
  code base ([clipper2 1.2.0 delta](31-clipper2-1-2-0-geometry-delta.md); the
  bump is parked; mind [t17](17-clipper2-cost-output-verdict.md)'s harness
  trap — pin the dep and assert the resolved version from the harness's own
  lock).
- **Vendoring mechanics have precedent.** The t57 take vendored the crate
  locally
  (`.local-artifacts/perimeter-reference-preparation/t57-topx-cache/source/.local-libs/clipper2-rust`,
  with `clipper2-original` as the oracle twin) and drove both through an
  independent exactness harness. The in-tree version would be a workspace
  member or a `[patch.crates-io]` redirect; the evidence must carry the
  upstream commit + the local diff list.
- **Headroom first.** The bundled canonical Clipper2 C++ lives at
  `OrcaSlicerDocumented/deps_src/clipper2` (the parity reference; re-derive
  its version at take time). First measurement: profile the Rust port
  against the canonical on representative matched-workload inputs (t68's
  per-site input shapes; the t55 body-union corpus) to size the optimization
  headroom. If the port is already at parity, resolve as a measured
  no-candidate ([t59](59-prep-slice-flat-bridge-closing-reduction.md) /
  [t68](68-classic-perimeters-host-round-trips.md) precedent) rather than
  forcing an experiment. **Not a replacement candidate:** exposing the C++
  to the host leg while the wasm guests keep the Rust port risks host/guest
  output divergence (the t33/t47 oracle contract) — the C++ is the baseline,
  not the target.
- **Exactness contract.** Output geometry must stay byte-identical: any
  coordinate-level change is a parity event and out of this map's scope (see
  the map's Out of scope). Candidate optimizations must be value-exact
  against the pristine crate on identical inputs (t57's dual-engine oracle
  pattern), then pass the fresh 16-cell output matrix before any timing.
- **Candidate space (not pre-approved, to be measured):** hot-path
  allocation/aliasing, integer-op bottlenecks, data layout, redundant
  intersection sweeps, the `closing_ex` / `offset` inner loops (the two
  largest scopes), and wasm-side codegen effects — both legs must be
  qualified, since an optimization can help one and hurt the other.
- **Acceptance:** the headroom measurement first (or a measured no-candidate
  verdict); for any candidate — independent exactness (dual-engine
  value-identity + full 16-cell output identity), then the standing paired
  ordinary + accelerated A/B on pre-declared cells (6 retained pairs per
  mode, zero starvation exclusions), keep/drop to the human, no auto-commit.
  Probe reverted; freshness green; evidence under `evidence/t69-clipper-engine/`
  (reducer + verifier in-repo; raw tree gitignored); candidate-built binaries
  purged from the standing dirs after the take (t36 audit precedent).
- **Recipe constraints:** instrumented runs never mixed into wall claims;
  `--module-dir modules/core-modules` mandatory; `cargo xtask build-guests
  --check` exit 0 before any capture; outputs disclosed per the fairness
  contract.
