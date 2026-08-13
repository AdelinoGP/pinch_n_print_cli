# Design: 227-dragon-curve-community-module

## Controlling Code Paths

- Primary code path: the dragon tiling + color mapping in `modules/community-modules/dragon-curve/src/lib.rs` (Rust fallback) or `modules/community-modules/dragon-curve/go/` (Go branch), with emission through `InfillOutputBuilder::push_sparse_path` over `SliceRegionView::sparse_infill_area`.
- Neighboring tests/fixtures: `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` (the structural model for sparse-infill unit tests and per-region config resolution); `resources/regression_wedge.stl` (the manual slice-test fixture).
- OrcaSlicer comparison: none — this packet carries **no** OrcaSlicer parity and no OrcaSlicer Reference Obligations section.

## Architecture Constraints

- The module is a **community module** and a **labeled example only**: authored under `modules/community-modules/dragon-curve/` but never added to the workspace `Cargo.toml` `members` list. Its natural CI exclusion is structural, not a special case.
- **Coloring is opt-in at the host, not at the module.** The module emits `tool_index = Some(f(tiling_index))` unconditionally and relies on the host marshal-boundary strip for ungranted regions, per ADR-0058 ("Modules never have to guard"). The module must therefore be written so its emitted `ExtrusionPath3D` literals set `tool_index` (when the 226 field exists), never gate emission on the grant.
- The tiling/color logic must be a **pure function** of `(ExPolygon, line_spacing, tiling_depth, color_map, tool_count)`: no `std::time`, no RNG, no `HashMap` iteration, no host-clock calls. Reproducibility is the packet's core invariant (AC-3).
- Config key spellings follow the repo's snake_case convention (`CLAUDE.md` §Config Key Naming Convention); the mirrored keys copy `rectilinear-infill.toml` exactly (`infill_density`, `infill_angle`, `infill_speed`, `line_width`), and the dragon keys are `tiling_depth` / `color_map`.
- Coordinate discipline: the module's geometry uses the same mm↔units helpers as core modules (`slicer_ir::mm_to_units` / `units_to_mm`; 1 unit = 100 nm). The dragon curve has **no** OrcaSlicer source, so no porting header is required.
- WIT re-declaration note (per-branch): a Rust module in-tree uses `slicer-sdk` and does **not** re-declare WIT types. The "re-declare WIT types" rule exists for foreign-language repos (the Go branch would re-declare in Go against its generated bindings). State which applies in the branch-specific step.

## Code Change Surface

- Selected approach: **two concrete branches on the 225 verdict.** Branch A fires only if 225's gate records "Go loadable-and-correct"; the expected Branch B is the Rust fallback. Steps 2-4 are branch-independent; only Step 5 diverges.

### Branch A — Go loadable-and-correct (only if 225 passes with that verdict)

- `modules/community-modules/dragon-curve/go/go.mod` - Go module with `wit-bindgen-go` dependency.
- `modules/community-modules/dragon-curve/go/*.go` - re-declared WIT types for the `Layer::Infill` world plus the dragon tiling + `map_tiling_index_to_tool` port.
- `modules/community-modules/dragon-curve/Makefile` - `make` runs `GOOS=wasip1 GOARCH=wasm go build -buildmode=c-shared` → `wasm-tools component embed/new` (with the WASI-preview2 adaptation 225 validated) → writes `dragon-curve.wasm`.
- `modules/community-modules/dragon-curve/dragon-curve.wasm` - the committed Go-built component.

### Branch B — Rust fallback (expected; 225 gate fails or records "not loadable-and-correct")

- `modules/community-modules/dragon-curve/Cargo.toml` - standalone crate (name `dragon-curve`), NOT in the workspace `members`; `[lib] crate-type = ["cdylib", "rlib"]`; deps `slicer-sdk`, `slicer-ir` via relative path `../../../crates/...`; `[workspace]` sentinel (empty table) so the crate is its own workspace and `cargo xtask build-guests` never sees it; dev-dep `slicer-sdk` with `features = ["test"]`; `[target.'cfg(target_arch = "wasm32")'.dependencies] wit-bindgen.workspace = true`.
- `modules/community-modules/dragon-curve/src/lib.rs` - `DragonCurve` struct + `#[slicer_module] impl LayerModule` with `from_config` and `run_infill`, mirroring `rectilinear-infill`'s shape: emit only over `region.sparse_infill_area()`, gated by `region.should_emit(ExtrusionRole::SparseInfill)`, with `map_tiling_index_to_tool` stamped into each path.
- `modules/community-modules/dragon-curve/tests/{dragon_tiling_tdd,dragon_color_map_tdd,dragon_config_override_tdd,dragon_emission_tdd}.rs` - the unit tests (auto-discovered by cargo, no `[[test]]` entries needed).
- `modules/community-modules/dragon-curve/go/` - retained in BOTH branches as a **labeled reference implementation only** (README banner: reference, not the shipped guest). In Branch B its Go source is not built and the committed `.wasm` is Rust-built.
- `modules/community-modules/dragon-curve/Makefile` - Branch B's `make` runs `cargo build --target wasm32-unknown-unknown --release`, then `wasm-tools strip --delete '^component-type:.*:slicer:sdk-'` and `wasm-tools component new` to write `dragon-curve.wasm` (the same sequence `xtask build_guests::build_one_inner` performs for core guests).
- `modules/community-modules/dragon-curve/dragon-curve.wasm` - the committed Rust-built component.

- Rejected alternatives and reasons:
  - Adding the module to the workspace `members` (rejected: violates spec §1's natural CI exclusion and would make `cargo xtask build-guests` discover it only if a `wit-guest/` shim were added, which is not the community-module shape).
  - A `wit-guest/` shim directory mirroring core modules (rejected: core modules need it for the separate guest workspace; a community module is one standalone crate with `#[slicer_module]` in `src/lib.rs`, so no shim).
  - Making the module gate its own emission on the grant (rejected: contradicts ADR-0058's "Modules never have to guard").
  - A parallel per-path tool side-list (rejected by ADR-0058: the field survives the infill linker's clone/re-emit; a side-list does not).

## Files in Scope (read + edit)

- `modules/community-modules/dragon-curve/dragon-curve.toml` - role: new manifest; expected change: full module + stage + claims + compatibility + config schema.
- `modules/community-modules/dragon-curve/Cargo.toml` - role: standalone crate manifest (Branch B; present in both branches for the Rust build path); expected change: package/deps/features/workspace sentinel.
- `modules/community-modules/dragon-curve/src/lib.rs` - role: dragon tiling + color mapping + module impl; expected change: the full module body.
- `modules/community-modules/dragon-curve/tests/*.rs` - role: unit tests; expected change: four test files.
- `modules/community-modules/dragon-curve/Makefile` - role: build script; expected change: branch-appropriate build + componentize sequence.
- `modules/community-modules/dragon-curve/README.md` - role: labeled-example banner + manual slice test; expected change: social-rule text + command.
- `modules/community-modules/dragon-curve/go/*` - role: Go reference implementation (both branches) or the shipped guest (Branch A); expected change: Go source.
- `modules/community-modules/dragon-curve/dragon-curve.wasm` - role: committed component; expected change: built artifact.

## Read-Only Context

- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full read - purpose: manifest structure and exact config-key spellings (the grounding model).
- `modules/core-modules/rectilinear-infill/Cargo.toml` - full read - purpose: module crate dependency/feature/lints shape.
- `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `48-287` (struct + `#[slicer_module] impl LayerModule` + `run_infill`) - purpose: module shape to mirror.
- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` - lines `1-80` - purpose: `test_prelude` fixture + `should_emit` pattern.
- `modules/core-modules/rectilinear-infill/wit-guest/Cargo.toml` - full read - purpose: the `[workspace]` sentinel convention (noting the dragon crate needs no `wit-guest/`).
- `crates/slicer-sdk/src/views.rs` - lines `440-572` - purpose: `sparse_infill_area` / `should_emit` semantics.
- `crates/slicer-sdk/src/builders.rs` - lines `26-136` - purpose: `InfillOutputBuilder::push_sparse_path`.
- `crates/slicer-sdk/src/config_resolution.rs` - full read - purpose: `resolve_float` for per-region override.
- `crates/slicer-ir/src/slice_ir.rs` - lines `700-727` - purpose: `ConfigValue` variants (`Int`, `Float`, `List`).

## Out-of-Bounds Files

- `crates/slicer-sdk/src/host.rs` (1108 lines) - never load wholesale; the 226 `tool_count` wrapper is a FORWARD-DEP and its body is 226's.
- `crates/slicer-sdk/src/traits.rs` (1821 lines) - delegate `SUMMARY` only for the `LayerModule::run_infill` signature; never read whole.
- `crates/slicer-wasm-host/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**` - delegate symbol lookups; do not browse (226's surface).
- `crates/slicer-ir/src/slice_ir.rs` beyond the `ConfigValue` range and `ExtrusionPath3D` (line ~1941) - ranged reads only.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `OrcaSlicerDocumented/` - never load (no parity in this packet).

## Expected Sub-Agent Dispatches

- Question: does `cargo xtask build-guests`'s discovery walk `modules/core-modules` and `crates/slicer-wasm-host/test-guests` only (i.e. never `modules/community-modules`)? scope: `xtask/src/build_guests.rs::discover_guests`; return: `FACT`; purpose: locked assumption L1.
- Question: what is the exact `wasm-tools` componentize sequence `build_one_inner` runs for a core guest (strip pattern + `component new`), so the dragon Makefile mirrors it? scope: `xtask/src/build_guests.rs::build_one_inner`; return: `SUMMARY` (≤100 words); purpose: build script.
- Question: what is the current recorded Go/MoonBit verdict in `docs/14_submodule_programming_languages.md` §Community-module context? scope: that section only; return: `SUMMARY`; purpose: Step 1 branch select.
- Question: what are the exact `ConfigView` value variants and `test_prelude` builders available for authoring a per-region override fixture? scope: `crates/slicer-ir/src/slice_ir.rs::ConfigValue`, `crates/slicer-sdk/src/test_prelude.rs`; return: `LOCATIONS`; purpose: test authoring.

## Data and Contract Notes

- IR/manifest contracts: `[module].id = "com.example.dragon-curve"`; `[stage].id = "Layer::Infill"`; `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]`; `[compatibility]` mirrors rectilinear's `min-host-version = "0.1.0"`, `min-ir-schema = "1.0.0"`, `max-ir-schema = "5.0.0"`.
- WIT boundary: the module consumes the 226 `extrusion-path3d.tool-index: option<u32>` field and 226's `tool-count` host service. These are FORWARD-DEPs; the module's own Rust surface is `slicer_ir::ExtrusionPath3D.tool_index: Option<u32>` and `slicer_sdk::host::tool_count() -> u32`.
- Determinism/scheduler constraints: tiling + color mapping are pure and byte-identical across runs; `layer-parallel-safe = true` in `[hints]` (each region is independent and there is no shared mutable state).

## Locked Assumptions and Invariants

- **L1** — `cargo xtask build-guests` never discovers `modules/community-modules/dragon-curve/` because `discover_guests` walks only `modules/core-modules` (looking for a `wit-guest/Cargo.toml`) and `crates/slicer-wasm-host/test-guests`. Verified by reading `xtask/src/build_guests.rs::discover_guests` (the two hard-coded `ws_root.join(...)` roots). The dragon dir therefore needs no special exclusion.
- **L2** — the host linker has no WASI, so a Go component that imports WASI preview2 is not loadable (this is the existing 225 evidence; the verdict must still be re-read from `docs/14` at Step 1, not quoted from this file).
- **L3** — `ExtrusionPath3D` today (pre-226) has exactly `{points, role, speed_factor}`; the `tool_index` field is added by 226. The dragon module's Step 5 emission wiring is the only surface that references it and is deferred by AC-7.
- **I1** — the module never touches `region.top_solid_fill()` / `bottom_solid_fill()` / `bridge_areas()`; it emits only `SparseInfill` over `sparse_infill_area()`.
- **I2** — two runs over identical input and config produce byte-identical segment output (reproducibility invariant).

## Risks and Tradeoffs

- The Go branch is a genuine two-way branch, not a stub: if 225 unexpectedly passes, the implementer must produce real Go bindings + a committed Go-built component, which is materially more work than the Rust fallback. The packet accepts this because §6 of the spec demands the branch, and 225's verdict is the only honest selector.
- The `color_map` list shape (`List` of `ConfigValue`) requires the manifest to declare it as a `float-list` or `string-list` type; the exact type is deferred to the manifest parser's supported vocabulary (verified: `float-list` exists in `wipe-tower.toml`). If a tool-sequence list is not cleanly expressible, fall back to a fixed `tiling_depth`-derived mapping and record it in the README — but AC-2's `color_map` key remains required.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5, the build + componentize + emission wiring)
- Highest-risk dispatch and required return format: the 225-verdict read — `SUMMARY` of `docs/14` §Community-module context (verbatim verdict + the one-line branch selector).

## Open Questions

- `[FWD]` Does 226's `slicer_sdk::host::tool_count()` return a native-fallback value (e.g. `1`) on non-wasm targets so `dragon_emission_tdd` can run without a WASM boundary, or must the emission test be `#[cfg(target_arch = "wasm32")]`? Resolved by reading 226's packet at activation; the emission test is deferred by AC-7 regardless.
- `[FWD]` Does the manifest schema parser accept a `color_map` list of tool indices as `float-list` (wipe-tower precedent) or only `string-list`? Resolved at Step 2 against the manifest parser; AC-2 only pins the key spelling.
- None `[BLOCK]` — both activation blockers are the explicit FORWARD-DEPs on 225/226.
