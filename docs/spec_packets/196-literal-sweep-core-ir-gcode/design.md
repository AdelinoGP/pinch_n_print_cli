# Design: 196-literal-sweep-core-ir-gcode

## Controlling Code Paths

- Primary code path: none changed — this packet edits only test-scope construction sites. The watched types' definitions (`Point3WithWidth`, `GlobalLayer`, `LayerCollectionIR`, `PrintEntity`, `WallLoop` in `crates/slicer-ir/src/slice_ir.rs`) are read-only anchors.
- Neighboring tests/fixtures: `slicer_sdk::test_support::fixtures::print_entity_base` (packet 195, `crates/slicer-sdk/src/test_support/fixtures.rs`); derived `Default` on `Point3WithWidth` and `GlobalLayer` and `impl Default for LayerCollectionIR` (all in `crates/slicer-ir/src/slice_ir.rs`, verified 2026-08-07).
- OrcaSlicer comparison: not applicable — no parity surface; construction-syntax refactor only.

## Architecture Constraints

- **Fixture-consumption decision (grounded 2026-08-07; corrected in preflight round 1).** `crates/slicer-sdk/Cargo.toml` depends on `slicer-core` with `features = ["host-algos"]` under `cfg(not(target_arch = "wasm32"))`. Therefore:
  - a `slicer-sdk` dev-dep in **`slicer-core`** would, via feature unification, enable `host-algos` for every `cargo test -p slicer-core` build — silently reversing the documented CLAUDE.md hazard semantics of bare narrow runs and pulling `boostvoronoi`/`rayon` into every core test build;
  - the same dev-dep in **`slicer-ir`** would pull `slicer-sdk` → `slicer-core` (+`host-algos`, `boostvoronoi`, `rayon`) into `slicer-ir`'s dev graph for 2 helper fns.
  Both are rejected. `slicer-ir`'s `fn make_entity` helpers (in tests `entity_id_invariants_tdd.rs`, `ir_validation_tdd.rs`) and `slicer-core`'s `fn make_wall` (in test `wall_sequence_reorder_tdd.rs`) keep exhaustive literals with the waiver reason: `// exhaustive: file-local base; sdk fixture home would pull host-algos into this crate's dev graph (packet 196 [FWD])`.
  **`slicer-gcode`** has NO `slicer-core` dependency today (its `[dependencies]` are `slicer-ir`, `slicer-helpers`, `thiserror`, `log`, `image` — re-verified against `crates/slicer-gcode/Cargo.toml` 2026-08-07; an earlier draft of this packet claimed otherwise and was wrong). The sdk dev-dep therefore NEWLY pulls `slicer-core`+`host-algos` (+`boostvoronoi`, `rayon`, `slicer-macros`, `slicer-schema`) into gcode's dev graph — the same tax rejected for `slicer-ir`. It is accepted here, on different grounds: (1) the tax is confined to gcode's own dev graph — dev-deps do not propagate, so it cannot alter feature resolution of `cargo test -p slicer-core` or any other crate's narrow run (the categorical hazard that rules out the slicer-core dev-dep does not exist here); (2) the fixture surface is 8 `PrintEntity` test files versus slicer-ir's 2 helper fns, which is enough consumption to justify decision 3(b)'s prescribed pattern over 8 files' worth of waivers; (3) in workspace-level builds `slicer-core`+`host-algos` is compiled anyway (`slicer-runtime`, `slicer-sdk`, `slicer-wasm-host` demand it), so the marginal cost lands only on cold isolated `cargo test -p slicer-gcode` builds — real but bounded (impact unmeasured; the [FWD] below covers reversal).
- **Carrier-test waivers.** `slicer-ir` roundtrip tests (`extrusion_line_roundtrip.rs`, `point3_overhang_distance_roundtrip.rs`, `point3_overhang_quartile_roundtrip.rs`) exist to prove every field survives serialization; exhaustiveness IS their intent. Waiver reason: `// exhaustive: carrier/roundtrip test asserts every field travels`.
- **Watched types without a base.** If the Step-1 report lists a watched type with neither `Default` nor a 195 fixture (e.g. a local view struct), use the packet-195 precedent: one file-local `fn <type>_base()` whose single exhaustive literal carries a waiver, and FRU over it at call sites. Never add `Default` to the type in this packet.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The guest-feeding paths in this packet are `crates/slicer-ir/src/slice_ir.rs` and `crates/slicer-core/src/**` (`#[cfg(test)]`-mod edits only; semantically inert for guests but mtime-tripping for `shared_input_paths` in `xtask/src/build_guests.rs`). `crates/slicer-gcode` is NOT a shared guest crate; its `Cargo.toml`/test edits do not trip the gate.

## Code Change Surface

- Selected approach: report-driven sweep. Step 1 runs `check-literals --report` for the area and records the violating file list; Steps 2-4 convert per crate; Step 5 proves invariance and freshness.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-gcode/Cargo.toml`: add `[dev-dependencies]` entry `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }`.
  - `slicer-gcode` tests (8 files with `PrintEntity` literals, measured 2026-08-07 — re-derive from report): route through `print_entity_base(role)` then override meaningful fields; `fn point3`-style helpers → `Point3WithWidth { x, y, z, width, ..Default::default() }`-shaped FRU; `LayerCollectionIR` literals → FRU over its `impl Default`.
  - `slicer-ir` tests: FRU for Default-able types; waivers for `make_entity` helpers and carrier roundtrip tests (reasons above); `#[cfg(test)]` mod in `src/slice_ir.rs` per report.
  - `slicer-core` tests + benches (`benches/polygon_ops`): `fn point3` / `fn junction` helpers → FRU; `fn make_wall` waiver; `#[cfg(test)]` mods in src files per report.
- Rejected alternatives and reasons:
  - sdk dev-dep for `slicer-ir`/`slicer-core` — rejected (feature-unification evidence above; gcode's dev-graph-only tax is accepted, theirs is not — see the corrected decision bullet).
  - Adding `Default` to `PrintEntity`/`WallLoop` — locked out by packet 195 (`ExtrusionRole`/`LoopType` have no safe default variant).
  - Spell-all-fields + FRU — rejected; defeats churn reduction and trips `clippy::needless_update`.

## Files in Scope (read + edit)

Sweep packet: per-crate bounded globs replace the 3-file list; each step still edits one crate's test tree only.

- `crates/slicer-ir/tests/**/*.rs` + `#[cfg(test)]` mod(s) in `crates/slicer-ir/src/slice_ir.rs` - role: area 1 sweep; expected change: FRU conversions + waivers.
- `crates/slicer-gcode/tests/**/*.rs` + `crates/slicer-gcode/Cargo.toml` + `#[cfg(test)]` mod in `crates/slicer-gcode/src/emit.rs` - role: area 2 sweep; expected change: dev-dep + fixture/FRU conversions.
- `crates/slicer-core/tests/**/*.rs` + `crates/slicer-core/benches/**/*.rs` + `#[cfg(test)]` mods in `crates/slicer-core/src/**` (only files named by the Step-1 report) - role: area 3 sweep; expected change: FRU conversions + 1 waiver.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - the struct definitions and `Default` impls only, ranged reads around `pub struct Point3WithWidth`, `pub struct GlobalLayer`, `pub struct LayerCollectionIR`, `pub struct PrintEntity`, `pub struct WallLoop` - purpose: confirm base availability per type.
- `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only - purpose: call `print_entity_base` correctly.
- `docs/21_data_defaults_and_fixtures.md` - conversion rule + waiver format - purpose: normative wording.

## Out-of-Bounds Files

- `xtask/src/check_literals.rs` and `xtask` tests - packet 194 owns the tool; never patch it here.
- `docs/spec_packets/194-check-literals-gate/**`, `docs/spec_packets/195-defaults-and-fixture-bases/**` except their `packet.spec.md` - inspect via SUMMARY dispatch only.
- Production (non-`cfg(test)`) code in all three crates - exhaustive literals there are intentional propagation checkpoints.
- `OrcaSlicerDocumented/` - not applicable; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load (scratch baseline files under `target/sweep-196-*` are the sole exception, created/greped by this packet).

## Expected Sub-Agent Dispatches

- Question: run the Step-1 report + baselines and return the per-crate violating-file list and summary line; scope: commands in `requirements.md` matrix; return: `LOCATIONS` (≤20 file entries per crate, violations-per-file count as context); purpose: Step 1.
- Question: does `cargo test -p <crate> [flags]` pass with summary multiset equal to `target/sweep-196-<crate>-baseline.txt`?; scope: one crate per dispatch; return: `FACT` PASS/FAIL + failing-test names ≤5 lines; purpose: Steps 2-5.
- Question: run `cargo clippy --workspace --all-targets -- -D warnings` / `cargo check --workspace --all-targets` / `cargo xtask build-guests --check`; scope: workspace; return: `FACT` pass/fail + first error ≤10 lines; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: untouched. Conversions must be value-identical: omitted fields equal the base's value by construction, so every struct a test builds is bit-identical pre/post.
- WIT boundary: untouched.
- Determinism/scheduler constraints: none.

## Locked Assumptions and Invariants

- Test counts, assert counts, and every constructed value are invariant; only construction syntax changes.
- `PrintEntity`/`WallLoop` remain `Default`-less (packet-195 lock, re-guarded by AC-N1).
- Waiver format is frozen by packet 194; reasons are mandatory (AC-N2).

## Risks and Tradeoffs

- The `slicer-gcode` sdk dev-dep grows gcode's dev graph (newly compiles `slicer-core`+`host-algos`, `boostvoronoi`, `rayon` for isolated `-p slicer-gcode` test builds) — accepted per the corrected fixture-consumption decision; reversal path is the [FWD] rule.
- The 194 checker may have blind spots (macro-token range expressions) or false positives unknown until first real-tree run at this scale. A checker defect is a deviation against packet 194, not a local patch; record it and waiver-or-skip the affected site with a reason naming the defect.
- Baseline suite runs (esp. `slicer-core --features host-algos`) are slow; both baseline and post runs are mandatory — budget two full area suite runs, no more (read `target/test-output.log` instead of re-running).
- `--check` will report `STALE:` after any `slicer-ir`/`slicer-core` src edit even though `cfg(test)` code never reaches guests; the rebuild is mandatory anyway to leave the tree clean (AC-6).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (slicer-core sweep: most files, feature-gated suite)
- Highest-risk dispatch and required return format: the Step-1 report enumeration — must return `LOCATIONS` capped at 20 entries per crate with counts, never the raw report body.

## Open Questions

- `[FWD]` Dev-dep reconsideration rule, covering all three crates: (a) if the Step-1 report shows the `slicer-ir`/`slicer-core` class-b waiver count exceeding ~6 sites (sizing estimate from 2026-08-07 greps: 3 helper fns), reconsider the sdk dev-dep for that crate — for `slicer-core` the feature-unification flip on its own bare runs remains the categorical bar to beat; (b) conversely, if the `slicer-gcode` dev-dep proves unacceptable at implementation time (e.g. `boostvoronoi` build failure or an unacceptable cold-build cost in gcode's dev graph), drop it and fall back to file-local waivered base fns exactly as `slicer-ir` does, recording the reversal in the close notes.
- `[FWD]` Exact FRU shape per site (which fields stay spelled) is implementer judgment under the "meaningful fields stay spelled" rule; no per-site enumeration is frozen here.
