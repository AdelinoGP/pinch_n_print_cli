---
status: implemented
packet: 196-literal-sweep-core-ir-gcode
task_ids:
  - TASK-318
---

# 196-literal-sweep-core-ir-gcode

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-ir`, `slicer-core`, and `slicer-gcode` test code to FRU-over-a-base (or a reasoned `// exhaustive:` waiver where exhaustiveness is the test's intent), so the area gate `cargo xtask check-literals crates/slicer-ir crates/slicer-core crates/slicer-gcode` exits 0 with every suite green and test counts unchanged.

## Problem Statement

Queue row #3 of `docs/specs/_OLD/struct-literal-churn-gate-plan.md`. The plan's measured churn (165-file sweep in `a579fc18` after `Point3WithWidth` gained a field) comes from exhaustive watched-type literals in test code. Packets 194/195 built the gate and the FRU bases; this packet is the first of three area sweeps: drive `cargo xtask check-literals` to zero violations for `slicer-ir`, `slicer-core`, and `slicer-gcode` without changing what any test asserts. It is one coherent slice because the three crates form the IR-to-G-code data spine, share the same fixture decisions (see `design.md`), and their combined violation surface is the smallest of the three sweep areas.

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
