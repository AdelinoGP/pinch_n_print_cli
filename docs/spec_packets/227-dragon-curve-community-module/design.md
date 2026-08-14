# Design: 227-dragon-curve-community-module

## Controlling Code Paths

- Primary code path: the dragon tiling + colour mapping in `modules/community-modules/dragon-curve/src/dragon/dragon.mbt` (`tile_dragon_curve`, `map_tiling_index_to_tool`, `pick_tiling_depth`), with emission through the WIT glue template `modules/community-modules/dragon-curve/src/glue/main.mbt.in` (`run`) over the region's sparse-infill area.
  - **Language corrected 2026-08-14.** This design was authored as a Go-vs-Rust branch. `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" supersedes the probes it branched on: Go is **NOT_LOADABLE_OR_CORRECT (terminal)** (wit-bindgen-go v0.7.0 emits imports `go:wasmimport` rejects), MoonBit is **LOADABLE_AND_CORRECT**, and under the locked priority order that section records **"Dragon Curve authoring language: MoonBit"**. The user directed MoonBit explicitly. §Code Change Surface below now describes the MoonBit module as built; the Branch A (Go) / Branch B (Rust fallback) structure it replaced is recorded there as superseded, not silently dropped.
- Neighboring tests/fixtures: `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` (the structural model for sparse-infill unit tests and per-region config resolution); `resources/20mm_cube.obj` (the manual slice-test fixture; `resources/regression_wedge.stl` was the original choice but hits a pre-existing `boostvoronoi` panic that reproduces with core modules only).
- OrcaSlicer comparison: none — this packet carries **no** OrcaSlicer parity and no OrcaSlicer Reference Obligations section.

## Architecture Constraints

- The module is a **community module** and a **labeled example only**: authored under `modules/community-modules/dragon-curve/` but never added to the workspace `Cargo.toml` `members` list. Its natural CI exclusion is structural, not a special case.
- **Coloring is opt-in at the host, not at the module.** The module emits `tool_index = Some(f(tiling_index))` unconditionally and relies on the host marshal-boundary strip for ungranted regions, per ADR-0058 ("Modules never have to guard"). The module must therefore be written so its emitted `ExtrusionPath3D` literals set `tool_index` (when the 226 field exists), never gate emission on the grant.
- The tiling/color logic must be a **pure function** of `(ExPolygon, line_spacing, tiling_depth, color_map, tool_count)`: no `std::time`, no RNG, no `HashMap` iteration, no host-clock calls. Reproducibility is the packet's core invariant (AC-3).
- Config key spellings follow the repo's snake_case convention (`CLAUDE.md` §Config Key Naming Convention); the mirrored keys copy `rectilinear-infill.toml` exactly (`infill_density`, `infill_angle`, `infill_speed`, `line_width`), and the dragon keys are `tiling_depth` / `color_map`.
- Coordinate discipline: the module's geometry uses the same mm↔units helpers as core modules (`slicer_ir::mm_to_units` / `units_to_mm`; 1 unit = 100 nm). The dragon curve has **no** OrcaSlicer source, so no porting header is required.
- WIT closure note (MoonBit): a foreign-language guest has no `slicer-sdk`, so the module carries a **frozen copy of the `Layer::Infill` WIT closure** under `modules/community-modules/dragon-curve/wit/` (`layer-infill.wit` plus `deps/common/common.wit`, `deps/config/config.wit`, `deps/ir-types/ir-types.wit`, `deps/types/types.wit`). `wit-bindgen moonbit` generates the bindings from that snapshot; the canonical source it was snapshotted from remains `crates/slicer-schema/wit/`, and the snapshot is refreshed by hand when the contract moves.
- No-SDK consequences (MoonBit): there is no `#[slicer_module]` macro and no `slicer-sdk`, so two SDK conveniences are reimplemented by hand in the module and unit-tested directly — the `should_emit` gate as `holds_sparse_fill` (`src/glue/main.mbt.in`, deliberately fail-closed: an empty held-claims list suppresses every fill role), and the per-region config override precedence that `slicer_sdk::config_resolution::resolve_float` provides for core modules as `pick_tiling_depth` (`src/dragon/dragon.mbt`).

## Code Change Surface

- Selected approach: **a single MoonBit guest**, selected by the packet 225a re-measurement recorded in `docs/14_submodule_programming_languages.md` (see §Controlling Code Paths). The module is not a Cargo crate at all: no `Cargo.toml`, no `src/lib.rs`, no `tests/*.rs`, no `#[slicer_module]`, no `slicer-sdk`, no `go/`, and no `wit-guest/` shim.

### Superseded — Branch A (Go) / Branch B (Rust fallback)

The original design offered two branches on 225's verdict: Branch A (Go bindings + `wit-bindgen-go` + a Go-built `.wasm`) if the gate recorded "Go loadable-and-correct", and Branch B (a standalone `dragon-curve` Cargo crate with `#[slicer_module] impl LayerModule` in `src/lib.rs`, four `tests/dragon_*_tdd.rs` files, and a `[workspace]` sentinel) otherwise. **Neither shipped.** Packet 225a made Go terminal and selected MoonBit, which the two-branch structure did not contemplate. Nothing under `go/`, `Cargo.toml`, `src/lib.rs`, or `tests/` was ever created; do not look for it.

### MoonBit module — the surface as built

- `modules/community-modules/dragon-curve/dragon-curve.toml` - the manifest: `[module]`, `[stage] id = "Layer::Infill"`, `[claims].holds`, `[compatibility]`, `[config.schema]` (seven keys: the six the ACs pin, plus `filament_density`, which the host's `derive_tool_count` requires a module to declare before it will report a truthful tool count), `[hints]`.
- `modules/community-modules/dragon-curve/moon.mod.json` - the MoonBit module root (`name = "slicer/layer-infill"`, `preferred-target = "wasm"`). This is the manifest `moon` resolves package paths against, which is why the test selector is `-p slicer/layer-infill/src/dragon`.
- `modules/community-modules/dragon-curve/wit/layer-infill.wit` + `wit/deps/{common,config,ir-types,types}/*.wit` - the frozen `Layer::Infill` WIT closure the bindings are generated from.
- `modules/community-modules/dragon-curve/src/dragon/dragon.mbt` - the pure tiling + colour logic, importing nothing (`moon.pkg.json` has an empty `import` list) so it is testable without the WIT bindings: `tile_dragon_curve`, `dragon_polyline`, `clip_segment`, `point_in_region`, `point_in_ring`, `generation_of`, `pick_tiling_depth`, `map_tiling_index_to_tool`, and the `Pt` / `Seg` / `Ring` / `Region` records.
- `modules/community-modules/dragon-curve/src/dragon/dragon_test.mbt` - the unit suite over that logic (determinism, holes, colour wrap, zero-`tool_count`, empty `color_map`, per-region override, one-tool-per-dragon, adjacent-dragon distinctness). Take the count from a run, not from this line.
- `modules/community-modules/dragon-curve/src/dragon/moon.pkg.json` - the pure package's manifest.
- `modules/community-modules/dragon-curve/src/glue/main.mbt.in` - the WIT glue **template**: `run`, the config readers (`cfg_float`, `cfg_int`, `cfg_color_map`), the hand-rolled `holds_sparse_fill` gate, and the geometry adapters. It is a `.in` template because `wit-bindgen moonbit` forward-declares `run` in the generated exported-interface package, so the definition must be copied *there*; a definition placed in `gen/` builds and then traps on dispatch.
- `modules/community-modules/dragon-curve/src/glue/moon.pkg.json.in` - the matching package manifest template (the generated one imports only what the bindings need; the glue additionally needs `geometry`, `host-services`, `config-types`, `ir-handles`, `module-errors`, and the pure `dragon` package).
- `modules/community-modules/dragon-curve/Makefile` - `make` runs `wit-bindgen moonbit` over the WIT closure, copies both glue templates into the generated interface package, `moon fmt` + `moon build --target wasm --release`, then `wasm-tools component embed --encoding utf16` and `wasm-tools component new` to write the artifact. `make test` runs the unit suite.
- `modules/community-modules/dragon-curve/README.md` - the labeled-example banner, the build/test commands, and the manual `pnp_cli slice` example.
- `modules/community-modules/dragon-curve/.gitignore` - excludes the generated `gen/`, `interface/`, `world/`, `_build/`, and `embedded.wasm` trees, which are build outputs and carry no hand-written source.
- `modules/community-modules/dragon-curve/dragon-curve.wasm` - the committed component, built by hand and never rebuilt by any workspace command.

- Rejected alternatives and reasons:
  - Adding the module to the workspace `members` (rejected: violates spec §1's natural CI exclusion, and the module is not a Cargo crate in the first place).
  - A `wit-guest/` shim directory mirroring core modules (rejected: that shim exists to give a Rust guest its own Cargo workspace; a MoonBit guest has no Cargo graph to isolate).
  - Putting the glue `run` definition in the generated `gen/` package instead of the exported-interface package (rejected on measured evidence: it builds and then traps on dispatch — recorded in the `Makefile`'s `bindings` target).
  - Making the module gate its own emission on the grant (rejected: contradicts ADR-0058's "Modules never have to guard").
  - A parallel per-path tool side-list (rejected by ADR-0058: the field survives the infill linker's clone/re-emit; a side-list does not).
  - Keying the colour on the per-segment ordinal (rejected on inspection of the printed result: the region is filled by *many* dragon instances on a rep-tile lattice, so an ordinal-keyed colour varies *within* each dragon and makes the tiling invisible. The driver is `Seg.tile`, the dragon-instance index — one dragon, one tool. `map_tiling_index_to_tool(tile_index, color_map, tool_count)`).

## Files in Scope (read + edit)

All paths below are under `modules/community-modules/dragon-curve/` and all exist on disk.

- `dragon-curve.toml` - role: manifest; expected change: module + stage + claims + compatibility + config schema + hints.
- `moon.mod.json` - role: MoonBit module root; expected change: name + preferred target.
- `wit/layer-infill.wit`, `wit/deps/common/common.wit`, `wit/deps/config/config.wit`, `wit/deps/ir-types/ir-types.wit`, `wit/deps/types/types.wit` - role: frozen `Layer::Infill` WIT closure; expected change: snapshot of `crates/slicer-schema/wit/`, refreshed by hand only when the contract moves.
- `src/dragon/dragon.mbt` - role: pure tiling + colour + override-precedence logic; expected change: the full algorithm body.
- `src/dragon/dragon_test.mbt` - role: unit tests backing AC-3/4/5/6/N1, plus coverage-uniformity and tile-stability guards. Re-derive the test count from a run; do not quote it here (it is a ledger fact and rots).
- `src/dragon/moon.pkg.json` - role: pure package manifest; expected change: empty `import` list (the no-dependency property that keeps the logic testable without bindings).
- `src/glue/main.mbt.in` - role: WIT glue template (`run`, config readers, `holds_sparse_fill`, geometry adapters, `tool_index` stamping); expected change: the full glue body.
- `src/glue/moon.pkg.json.in` - role: glue package manifest template; expected change: the import list the glue needs beyond the generated bindings.
- `Makefile` - role: build script; expected change: bindgen + template copy + `moon build` + `wasm-tools embed/new`, plus the `test` target.
- `README.md` - role: labeled-example banner + build/test + manual slice test; expected change: social-rule text + commands.
- `.gitignore` - role: exclude generated `gen/`, `interface/`, `world/`, `_build/`, `embedded.wasm`.
- `dragon-curve.wasm` - role: committed component; expected change: built artifact.

## Read-Only Context

Line numbers below are navigation hints only; the symbol name is the identifier.

- `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" - ranged read of that section - purpose: the binding language verdict (Go terminal, MoonBit selected).
- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full read - purpose: manifest structure and exact config-key spellings (the grounding model for the four mirrored keys).
- `modules/core-modules/wipe-tower/wipe-tower.toml` - full read - purpose: the `float-list` precedent for `color_map`.
- `crates/slicer-schema/wit/` (the `Layer::Infill` closure) - purpose: the canonical WIT the module's `wit/` snapshot is taken from; read to confirm the snapshot has not drifted, never edited by this packet.
- `SliceRegionView::sparse_infill_area` and `SliceRegionView::should_emit` (`crates/slicer-sdk/src/views.rs`) - purpose: the semantics the MoonBit guest reimplements by hand as `holds_sparse_fill`; read as a spec, not as a dependency (a foreign guest cannot call the SDK).
- `InfillOutputBuilder::push_sparse_path` (`crates/slicer-sdk/src/builders.rs`) - purpose: the sparse-path emission shape the glue's `run` mirrors over the WIT builder.
- `resolve_float` (`crates/slicer-sdk/src/config_resolution.rs`) - purpose: the per-region override precedence `pick_tiling_depth` reimplements.
- `ConfigValue` (`crates/slicer-ir/src/slice_ir.rs`, near line 700) - purpose: the `Int` / `Float` / `List` variants behind `cfg_int`, `cfg_float`, and `cfg_color_map`.
- `modules/core-modules/rectilinear-infill/src/lib.rs` (`run_infill`) - purpose: structural model for role/region discipline only. **Not a template here** — it is a Rust `#[slicer_module]` guest and this module is not.

## Out-of-Bounds Files

- `crates/slicer-sdk/src/host.rs` (long) - never load wholesale; `tool_count` is 226's surface and the MoonBit guest reaches the host service through its generated `host-services` bindings, not through the SDK.
- `crates/slicer-sdk/src/traits.rs` (long) - delegate a `SUMMARY` at most; the guest implements the WIT export directly and never sees `LayerModule`.
- `crates/slicer-wasm-host/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**` - delegate symbol lookups; do not browse (226's surface). The module's `wit/` snapshot is edited here; the canonical `crates/slicer-schema/wit/` is not.
- `crates/slicer-ir/src/slice_ir.rs` beyond `ConfigValue` and `ExtrusionPath3D` - ranged reads only.
- `modules/community-modules/dragon-curve/{gen,interface,world,_build}/` and `embedded.wasm` - **generated by `make`, gitignored, and blown away by the `bindings` target.** Never read them as a source of truth and never hand-edit them; the only hand-written glue lives in `src/glue/*.in`.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `OrcaSlicerDocumented/` - never load (no parity in this packet).

## Expected Sub-Agent Dispatches

- Question: does `cargo xtask build-guests`'s discovery walk `modules/core-modules` and `crates/slicer-wasm-host/test-guests` only (i.e. never `modules/community-modules`)? scope: `xtask/src/build_guests.rs::discover_guests`; return: `FACT`; purpose: locked assumption L1.
- Question: what is the current recorded language verdict? scope: `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a" only; return: `SUMMARY`; purpose: language select (answered: MoonBit).
- Question: what `ConfigValue` variants can reach a guest through the WIT `config-types` interface, so `cfg_int` / `cfg_float` / `cfg_color_map` cover them? scope: `ConfigValue` (`crates/slicer-ir/src/slice_ir.rs`) and the module's `wit/deps/config/config.wit` snapshot; return: `LOCATIONS`; purpose: glue config readers.
- Question: which list-valued types does the manifest schema parser accept for `color_map`? scope: the manifest parser; return: `FACT` (answered: `float-list`).
- *(Dropped)* the `build_one_inner` componentize-sequence dispatch. That sequence (`wasm-tools strip` of the `component-type:*:slicer:sdk-*` custom sections, then `component new`) is specific to a Rust `slicer-sdk` guest. The MoonBit build emits a bare core module with no SDK custom sections and instead needs `wasm-tools component embed --encoding utf16` before `component new`; see the `Makefile`.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5, the build + componentize + emission wiring)
- Highest-risk dispatch and required return format: the 225-verdict read — `SUMMARY` of `docs/14` §Community-module context (verbatim verdict + the one-line branch selector).

## Open Questions

- *(Resolved — moot.)* The first question asked whether `slicer_sdk::host::tool_count()` has a non-wasm fallback so a Rust `dragon_emission_tdd` could run off-wasm. There is no Rust test and no SDK: `tool_count` is reached through the guest's generated `host-services` bindings, and the pure package (`src/dragon/`) is deliberately binding-free so its tests never need a host at all. AC-7 is instead verified end-to-end against a real slice (see `packet.spec.md`).
- *(Resolved.)* The manifest schema parser accepts `float-list`; `color_map` is declared `type = "float-list"` in `dragon-curve.toml` (the wipe-tower precedent held). Values arrive as doubles and `cfg_color_map` (`src/glue/main.mbt.in`) rounds them to non-negative integers; an absent or wrongly-typed key yields an empty list, which disables authored colouring rather than failing the slice.
- **Open** — the frozen `wit/` snapshot has no drift check against `crates/slicer-schema/wit/`. See §Risks.
- None `[BLOCK]` — both original activation blockers (225, 226) have landed.
