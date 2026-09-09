---
status: implemented
packet: 227-dragon-curve-community-module
task_ids:
  - TASK-338
---

# 227-dragon-curve-community-module

## Goal

Author the first community module — the dragon-curve sparse-infill tiling with deterministic **per-dragon** tool coloring — at `modules/community-modules/dragon-curve/`. The recorded 225a verdict selects **MoonBit** (see §Authoring-Language Branch), so the module ships as a hand-written MoonBit guest plus a committed `.wasm`.

## Problem Statement

The design spec (`docs/specs/_OLD/community-modules-dragon-curve-infill.md` §1, §4, §5) requires the first `modules/community-modules` entry: a sparse-infill module that tiles the sparse-fill polygon with the dragon curve and tiles the region with four dragons rotated 90 degrees apart and colours each dragon deterministically by its rotation index, wrapped into the host's tool count. The mechanism it depends on — the per-path `tool-index` carrier and the two-sided `claim:authored-coloring` grant — does not exist yet and is authored by draft 226. This packet therefore authors the module as a **consumer** of 226's symbols and branches its authoring language on 225's recorded verdict, exactly as §6 of the spec prescribes.

This is one coherent slice because it is the entire *module artifact* surface: no host code, no canonical WIT edit, no workspace membership. The authoring language is resolved from the recorded 225/225a verdict rather than re-probed here; see the correction note in §In Scope.

## Architecture Constraints

- The module is a **community module** and a **labeled example only**: authored under `modules/community-modules/dragon-curve/` but never added to the workspace `Cargo.toml` `members` list. Its natural CI exclusion is structural, not a special case.
- **Coloring is opt-in at the host, not at the module.** The module emits `tool_index = Some(f(tiling_index))` unconditionally and relies on the host marshal-boundary strip for ungranted regions, per ADR-0058 ("Modules never have to guard"). The module must therefore be written so its emitted `ExtrusionPath3D` literals set `tool_index` (when the 226 field exists), never gate emission on the grant.
- The tiling/color logic must be a **pure function** of `(ExPolygon, line_spacing, tiling_depth, color_map, tool_count)`: no `std::time`, no RNG, no `HashMap` iteration, no host-clock calls. Reproducibility is the packet's core invariant (AC-3).
- Config key spellings follow the repo's snake_case convention (`CLAUDE.md` §Config Key Naming Convention); the mirrored keys copy `rectilinear-infill.toml` exactly (`infill_density`, `infill_angle`, `infill_speed`, `line_width`), and the dragon keys are `tiling_depth` / `color_map`.
- Coordinate discipline: the module's geometry uses the same mm↔units helpers as core modules (`slicer_ir::mm_to_units` / `units_to_mm`; 1 unit = 100 nm). The dragon curve has **no** OrcaSlicer source, so no porting header is required.
- WIT closure note (MoonBit): a foreign-language guest has no `slicer-sdk`, so the module carries a **frozen copy of the `Layer::Infill` WIT closure** under `modules/community-modules/dragon-curve/wit/` (`layer-infill.wit` plus `deps/common/common.wit`, `deps/config/config.wit`, `deps/ir-types/ir-types.wit`, `deps/types/types.wit`). `wit-bindgen moonbit` generates the bindings from that snapshot; the canonical source it was snapshotted from remains `crates/slicer-schema/wit/`, and the snapshot is refreshed by hand when the contract moves.
- No-SDK consequences (MoonBit): there is no `#[slicer_module]` macro and no `slicer-sdk`, so two SDK conveniences are reimplemented by hand in the module and unit-tested directly — the `should_emit` gate as `holds_sparse_fill` (`src/glue/main.mbt.in`, deliberately fail-closed: an empty held-claims list suppresses every fill role), and the per-region config override precedence that `slicer_sdk::config_resolution::resolve_float` provides for core modules as `pick_tiling_depth` (`src/dragon/dragon.mbt`).

## Data and Contract Notes

- IR/manifest contracts: `[module].id = "com.example.dragon-curve"`; `[stage].id = "Layer::Infill"`; `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]`; `[compatibility]` mirrors rectilinear's `min-host-version = "0.1.0"`, `min-ir-schema = "1.0.0"`, `max-ir-schema = "5.0.0"`.
  - **Corrected 2026-08-14.** This line previously read `holds = ["claim:sparse-fill"]` and justified it as "the authored-coloring capability claim was dropped — ADR-0058 §Amendment 2026-08-13". That citation was wrong: the amendment exists but says nothing about manifest claims. Its entire scope is two corrections of *reasoning* (the side-table rejection was re-argued on identity-timing grounds, and a field-residency check against ADR-0032), and it opens by reaffirming that "the two-sided grant ... stand[s]". The ADR header and design spec §2 both require the module to disclose `claim:authored-coloring`, and `authored_coloring_granted` (`crates/slicer-wasm-host/src/marshal/out.rs`) returns `false` unless the disclosure is present — so dropping it would silently strip every `tool_index` the module emits, disabling the exact feature this packet exists to demonstrate. AC-1 was right; this line was wrong.
- WIT boundary: the module consumes 226's `extrusion-path3d.tool-index: option<u32>` field and its `tool-count` host service **through the WIT surface directly** — the generated `host-services` and `ir-handles` bindings from the module's frozen `wit/` snapshot. The Rust-side equivalents (`slicer_ir::ExtrusionPath3D.tool_index: Option<u32>`, `slicer_sdk::host::tool_count()`) are the host's view of the same contract, not something this module links against.
- Determinism/scheduler constraints: tiling + color mapping are pure and byte-identical across runs; `layer-parallel-safe = true` in `[hints]` (each region is independent and there is no shared mutable state).

## Locked Assumptions and Invariants

- **L1** — `cargo xtask build-guests` never discovers `modules/community-modules/dragon-curve/` because `discover_guests` (`xtask/src/build_guests.rs`) walks only `modules/core-modules` and `crates/slicer-wasm-host/test-guests` (two hard-coded `ws_root.join(...)` roots). The dragon dir therefore needs no special exclusion. This holds a fortiori for a MoonBit module, which has no Cargo manifest to discover. **Consequence: nothing rebuilds `dragon-curve.wasm`.** It is built by hand with `make` and committed; the guest-staleness rule in `CLAUDE.md` does not cover it.
- **L2** *(superseded)* — originally "the host linker has no WASI, so a Go component importing WASI preview2 is not loadable". The accommodating host removed that blocker, and packet 225a re-measured: Go is now terminal for a different reason (wit-bindgen-go v0.7.0 emits imports `go:wasmimport` rejects, so an export-wired build cannot compile). **Replacement L2** — MoonBit is `LOADABLE_AND_CORRECT` under the accommodating host, and the earlier "strings corrupted" verdict was a fixture packaging error, not a toolchain defect. Both facts must be re-read from `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a", never quoted from this file.
- **L3** *(discharged)* — `ExtrusionPath3D.tool_index` was 226's FORWARD-DEP. 226 has landed; the field and the `tool-count` host service exist, and the glue's `run` (`src/glue/main.mbt.in`) stamps `tool_index` on every emitted path unconditionally, per ADR-0058. See `packet.spec.md` AC-7 for the end-to-end evidence.
- **L4** — MoonBit strings are UTF-16, so `wasm-tools component embed --encoding utf16` is **mandatory**, not stylistic; the host transcodes at the canonical ABI boundary. Dropping the flag reintroduces the string corruption the 2026-08-11 probe misdiagnosed as a MoonBit defect.
- **L5** — the MoonBit build is **not bit-reproducible**: a second clean build of unchanged sources produces a different `.wasm` hash. This is a recorded toolchain property (noted in the module `Makefile`), not a determinism failure of the tiling logic. I2 below is about segment output, not artifact bytes; do not conflate them.
- **I1** — the module never emits top-solid, bottom-solid, or bridge roles; it emits only sparse fill, and only when `holds_sparse_fill` passes on the region's held claims.
- **I2** — two runs over identical input and config produce byte-identical segment output (reproducibility invariant). `tile_dragon_curve` uses no RNG, no clock, and no map iteration; even the sort is hand-rolled (`sort_doubles`) so segment ordering cannot drift with a toolchain update.

## Risks and Tradeoffs

- *(Superseded)* The original risk here was that the Go branch might fire and cost materially more than the Rust fallback. 225a resolved it differently: Go is terminal and MoonBit won the priority order, so the real cost landed on a third path neither branch anticipated — a guest with no SDK, where `should_emit` and per-region override precedence had to be reimplemented and unit-tested by hand (`holds_sparse_fill`, `pick_tiling_depth`).
- **The module is invisible to CI and to every build gate.** No workspace command builds it, tests it, or notices when the frozen `wit/` snapshot drifts from `crates/slicer-schema/wit/`. A WIT contract change will not fail any test; it will produce a component that fails typed instantiation the next time someone slices with it. A host-side marshal-boundary assertion (extending `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs`) would be a stronger regression guard than the manual slice example.
- The `color_map` list shape (`List` of `ConfigValue`) requires the manifest to declare it as a `float-list` or `string-list` type; the exact type is deferred to the manifest parser's supported vocabulary (verified: `float-list` exists in `wipe-tower.toml`). If a tool-sequence list is not cleanly expressible, fall back to a fixed `tiling_depth`-derived mapping and record it in the README — but AC-2's `color_map` key remains required.
