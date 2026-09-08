# Implementation Plan: 227-dragon-curve-community-module

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Read the recorded 225 verdict and select the authoring branch

- Task IDs: `TASK-338`
- Objective: Recover the recorded feasibility verdict from the living record and lock the authoring language before writing any language-specific artifact.
  - **Language corrected 2026-08-14.** This step was written as a Go-vs-Rust branch select. `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" supersedes the probes it branched on: Go is NOT_LOADABLE_OR_CORRECT (terminal), MoonBit is LOADABLE_AND_CORRECT and is the recorded selection. There is no `BRANCH_A`/`BRANCH_B`; the module is MoonBit.
- Precondition: `docs/14_submodule_programming_languages.md` §Community-module context carries 225's recorded verdict (or a missing-verdict condition is observed and reported).
- Postcondition: The verdict is recorded verbatim in the step log and the authoring language is locked to MoonBit, with the one-line rationale. The module directory `modules/community-modules/dragon-curve/` is confirmed empty.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/14_submodule_programming_languages.md` - delegated SUMMARY of §Community-module context only (not a full read).
  - `modules/community-modules/` - directory listing (confirm empty parent).
- Files allowed to edit (at most 3):
  - (none — read-only discovery step)
- Files explicitly out of bounds:
  - `docs/spec_packets/225-*/**` (does not exist yet; never attempt to read)
  - `docs/feasibility-probes/*` (225's raw records; the living verdict is authoritative)
- Blast-radius discipline: not applicable (no struct field or schema constant added).
- Expected sub-agent dispatches:
  - Question: what is the current recorded Go/MoonBit verdict in `docs/14_submodule_programming_languages.md` §Community-module context? scope: that section only; return: `SUMMARY` (≤100 words, verdict verbatim).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - direct read of the Central Symbol Contract + Grounding Facts.
- OrcaSlicer refs:
  - none.
- Verification:
  - `ls modules/community-modules/dragon-curve 2>/dev/null` - FACT: directory absent or empty before authoring.
  - `rg -n 'not loadable|loadable-and-correct|not correct' docs/14_submodule_programming_languages.md` - FACT: the verdict line exists and is quoted in the step log.
- Exit condition: branch selector recorded and the empty-directory precondition confirmed.

### Step 2: Author `dragon-curve.toml` (manifest + config schema)

- Task IDs: `TASK-338`
- Objective: Write the complete manifest mirroring `rectilinear-infill.toml` with the dragon's claims, compatibility, stage, and six config keys.
- Precondition: Step 1 branch selected (manifest is branch-independent).
- Postcondition: `dragon-curve.toml` exists with `[module] id = "com.example.dragon-curve"`, `[stage] id = "Layer::Infill"`, `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]` (corrected 2026-08-14 to match AC-1 and ADR-0058; see `design.md` §Data and Contract Notes), `[compatibility]` min-host/min-ir/max-ir mirrored from rectilinear, and `[config.schema]` keys `infill_density`, `infill_angle`, `infill_speed`, `line_width`, `tiling_depth`, `color_map` in snake_case. AC-1 and AC-2 green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full read (manifest model).
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full read (the `float-list` precedent for `color_map`).
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/dragon-curve.toml`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/manifest.rs` (the parser is read-only context; delegate the supported-type question if needed).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what list-valued config types does the manifest schema parser accept (is a tool-index list expressible as `float-list`)? scope: `crates/slicer-scheduler/src/manifest.rs`; return: `FACT` (supported type strings for list fields).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1, §5 - direct read.
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q '"com\.example\.dragon-curve"' modules/community-modules/dragon-curve/dragon-curve.toml` - FACT pass/fail.
  - `rg -q '\[config\.schema\.(infill_density|infill_angle|infill_speed|line_width|tiling_depth|color_map)\]' modules/community-modules/dragon-curve/dragon-curve.toml` - FACT pass/fail (AC-2).
- Exit condition: AC-1 and AC-2 commands pass.

### Step 3: Author the pure tiling + color-mapping logic and its tests (TDD)

- Task IDs: `TASK-338`
- Objective: Implement the deterministic dragon tiling over an `ExPolygon` and the pure `map_tiling_index_to_tool` wrap, TDD-first, with no `tool_count`/`tool_index` dependency.
- Precondition: Step 2 manifest exists (the config keys it declares are the `from_config` source).
- Postcondition: `src/dragon/dragon.mbt` exposes the pure helpers — `tile_dragon_curve(region : Region, line_spacing : Int64, tiling_depth : Int) -> Array[Seg]`, `map_tiling_index_to_tool(tile_index : Int, color_map : Array[Int], tool_count : Int) -> Int?`, plus `dragon_polyline`, `clip_segment`, `point_in_region`, `point_in_ring`, `generation_of` and the `Pt` / `Seg` / `Ring` / `Region` records — and `src/dragon/dragon_test.mbt` is green for AC-3, AC-4, AC-5, AC-N1.
  - **Language corrected 2026-08-14.** This step originally named `src/lib.rs` and three `tests/dragon_*_tdd.rs` files. Packet 225a (`docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)") made Go terminal and selected MoonBit, so there is no Rust crate and no `tests/` directory; all unit tests live in the single file `src/dragon/dragon_test.mbt`.
  - **Colouring driver corrected.** The signature above is `map_tiling_index_to_tool(tile_index, ...)`, not the originally planned `(ordinal, generation, ...)`. The region is filled by many dragon instances on a rep-tile lattice, so the colour must key on `Seg.tile` — the dragon-instance index — giving one dragon one tool. An ordinal-keyed version was built first, coloured *within* each dragon, made the tiling invisible, and is gone. `ordinal` and `generation` are still carried on `Seg` and asserted by the tests as the curve's defining structure; they no longer drive colour.
- Files allowed to read, with ranges when over 300 lines:
  - `ConfigValue` (`crates/slicer-ir/src/slice_ir.rs`, near line 700) - the config variants the pure logic must eventually accept.
  - `modules/community-modules/dragon-curve/wit/deps/types/types.wit` - the geometry and `extrusion-path3d` shapes the pure records adapt to.
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/src/dragon/dragon.mbt`
  - `modules/community-modules/dragon-curve/src/dragon/dragon_test.mbt`
  - `modules/community-modules/dragon-curve/src/dragon/moon.pkg.json`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/**` - the guest has no SDK; nothing in this step may link against it.
  - `modules/community-modules/dragon-curve/{gen,interface,world,_build}/` - generated by `make`, gitignored, destroyed by the `bindings` target.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what `ConfigValue` variants can cross the WIT `config-types` boundary, so the depth/colour inputs are representable? scope: `ConfigValue` (`crates/slicer-ir/src/slice_ir.rs`) and `modules/community-modules/dragon-curve/wit/deps/config/config.wit`; return: `LOCATIONS` (≤20 entries).
- Design note: `src/dragon/moon.pkg.json` declares an **empty `import` list**. That is load-bearing, not incidental — the pure package depends on no generated binding, so `moon test` runs it without any host, WIT closure, or component. Adding an import here would make the tiling logic untestable off-component.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §4 - direct read (the tiling semantics + edge cases).
- OrcaSlicer refs:
  - none.
- Verification (`cargo test` is unrunnable in this directory — there is no `Cargo.toml`):
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'tiling_is_deterministic_across_runs'` - FACT pass/fail (AC-3).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'holes_are_excluded_from_tiling'` - FACT pass/fail (AC-5).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'color_map_wraps_into_tool_count'` - FACT pass/fail (AC-4).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'tool_count_zero_returns_none'` - FACT pass/fail (AC-N1).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon` - FACT: whole pure suite green (equivalently `make test`). `-f` takes a name glob, so a bare run is the way to get the full count.
- Exit condition: AC-3, AC-4, AC-5, AC-N1 green; the `per_region_tiling_depth_override` test is authored and passing in Step 4.

### Step 4: Wire the WIT glue `run` and the per-region override

- Task IDs: `TASK-338`
- Objective: Wrap the pure helpers in the exported WIT `run`, reading module and per-region config through the generated `config-types` bindings, applying the override precedence via `pick_tiling_depth`, and emitting sparse paths gated by `holds_sparse_fill`.
  - **Language corrected 2026-08-14.** This step originally wrapped the helpers in `#[slicer_module] impl LayerModule` (Branch B) or a Go port (Branch A). Packet 225a selected MoonBit (`docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)"): Go is `NOT_LOADABLE_OR_CORRECT (terminal)`, MoonBit is `LOADABLE_AND_CORRECT`. There is no `Cargo.toml`, no `#[slicer_module]`, and no `slicer-sdk`, so **two SDK conveniences are reimplemented by hand and unit-tested directly**: `should_emit` becomes `holds_sparse_fill` (`src/glue/main.mbt.in`, fail-closed — an empty held-claims list suppresses every fill role, because emitting anyway would duplicate whichever module actually holds the region), and `slicer_sdk::config_resolution::resolve_float`'s precedence becomes `pick_tiling_depth` (`src/dragon/dragon.mbt`).
- Precondition: Step 3 pure helpers green.
- Postcondition: `run` (`src/glue/main.mbt.in`) emits sparse-fill paths over the region's sparse area only, honours a per-region `tiling_depth` override, stamps `tool_index` from `map_tiling_index_to_tool(seg.tile, color_map, tool_count)` unconditionally (ADR-0058: never guard on the grant), and the `per_region_tiling_depth_override` test is green.
- Files allowed to read, with ranges when over 300 lines:
  - `run_infill` (`modules/core-modules/rectilinear-infill/src/lib.rs`) - structural model for role/region discipline **only**; it is a Rust SDK guest and is not a template here.
  - `resolve_float` (`crates/slicer-sdk/src/config_resolution.rs`) - the precedence rule `pick_tiling_depth` reimplements.
  - `SliceRegionView::sparse_infill_area` / `SliceRegionView::should_emit` (`crates/slicer-sdk/src/views.rs`) - the gate semantics `holds_sparse_fill` reimplements.
  - `modules/community-modules/dragon-curve/wit/layer-infill.wit` and `wit/deps/config/config.wit` - the exported `run` signature and `ConfigView` accessors as the guest actually sees them.
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/src/glue/main.mbt.in`
  - `modules/community-modules/dragon-curve/src/glue/moon.pkg.json.in`
  - `modules/community-modules/dragon-curve/src/dragon/dragon.mbt` (`pick_tiling_depth` + its tests in `dragon_test.mbt`)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/traits.rs` - the guest implements the WIT export directly and never sees `LayerModule`; do not load the file.
  - `modules/community-modules/dragon-curve/{gen,interface,world}/` - the copy targets, not the sources. Edit the `.in` templates; `make bindings` overwrites everything there.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what is the exact exported `run` signature and the sparse-path builder shape in the frozen WIT closure? scope: `modules/community-modules/dragon-curve/wit/layer-infill.wit`, `wit/deps/ir-types/ir-types.wit`; return: `LOCATIONS` (≤10 entries).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §5 - direct read (minimal scope + per-region overrides).
- OrcaSlicer refs:
  - none.
- Verification (`cargo test` is unrunnable in this directory — there is no `Cargo.toml`):
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'per_region_tiling_depth_override'` - FACT pass/fail (AC-6).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'each_dragon_gets_exactly_one_tool'` - FACT pass/fail (the tile-keyed colouring driver).
  - `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon` - FACT: whole pure suite green.
  - `rg -q 'tool_index: tool' modules/community-modules/dragon-curve/src/glue/main.mbt.in` - FACT: the glue stamps `tool_index` unconditionally.
  - `cd modules/community-modules/dragon-curve && make` - FACT: the glue compiles and componentizes (the only check that the `.in` templates are valid MoonBit against the generated bindings; `moon test` covers the pure package only).
- Exit condition: AC-6 green, the pure suite green, and `make` produces a component.

### Step 5: WIT snapshot, build script, committed `.wasm`, and banner README

- Task IDs: `TASK-338`
- Objective: Freeze the `Layer::Infill` WIT closure under the module's `wit/`, author the `Makefile` (bindgen + glue-template copy + `moon build` + `wasm-tools` componentize), produce the committed `dragon-curve.wasm`, and write the banner `README.md` with the manual slice example.
  - **Language corrected 2026-08-14.** This step originally described a Rust/Go build (`cargo build --target wasm32-unknown-unknown` + a `wasm-tools strip` of the `slicer-sdk` custom sections, or a `GOOS=wasip1` Go build) and a `tests/dragon_emission_tdd.rs` file deferred behind 226. Packet 225a selected MoonBit; **none of that exists.** The build is `wit-bindgen moonbit` → `moon build --target wasm --release` → `wasm-tools component embed --encoding utf16` → `wasm-tools component new`. There is no strip step (MoonBit emits a bare core module with no SDK custom sections) and no emission test file — 226 has landed and AC-7 is verified end-to-end against a real slice instead (see `packet.spec.md` AC-7).
- Precondition: Step 4 green; locked assumption L1 (xtask discovery) re-verified via dispatch.
- Postcondition: `wit/` + `Makefile` + `dragon-curve.wasm` + `README.md` + `.gitignore` exist; `make` reproduces the artifact from a clean tree; the manual slice command is documented.
- Files allowed to read, with ranges when over 300 lines:
  - `discover_guests` (`xtask/src/build_guests.rs`) - the two hard-coded roots, for L1.
  - `crates/slicer-schema/wit/` - the canonical `Layer::Infill` closure the module's `wit/` snapshot is taken from (read only; this packet never edits it).
  - `modules/community-modules/dragon-curve/Makefile` - the recorded toolchain versions the build was verified against.
- Files allowed to edit (at most 3, plus the `wit/` snapshot which is a mechanical copy):
  - `modules/community-modules/dragon-curve/Makefile`
  - `modules/community-modules/dragon-curve/README.md`
  - `modules/community-modules/dragon-curve/.gitignore`
  - `modules/community-modules/dragon-curve/wit/layer-infill.wit` and `wit/deps/{common,config,ir-types,types}/*.wit` - snapshot copy
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` - the canonical WIT; snapshot from it, never edit it.
  - `crates/slicer-sdk/src/host.rs` - the guest reaches `tool-count` through its generated `host-services` bindings, not the SDK.
  - `docs/*.md` (packet 228 owns docs).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does `discover_guests` walk only `modules/core-modules` and `crates/slicer-wasm-host/test-guests`, never `modules/community-modules`? scope: `discover_guests` (`xtask/src/build_guests.rs`); return: `FACT`.
- Build notes that are load-bearing, not stylistic:
  - `--encoding utf16` on `component embed` is **mandatory**: MoonBit strings are UTF-16 and the host transcodes at the canonical ABI boundary. Omitting it reproduces the string corruption the 2026-08-11 probe misdiagnosed as a MoonBit defect.
  - The glue definition of `run` must be copied into the **generated exported-interface package**, not into `gen/`. `wit-bindgen moonbit` forward-declares `run` there; a definition in `gen/` builds cleanly and then traps on dispatch. This is why the glue is a `.in` template.
  - The build is **not bit-reproducible** — a second clean build yields a different `.wasm` hash. Recorded MoonBit property; do not treat a hash change as a diff.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1 - direct read (packaging + build/artifact + banner README).
- OrcaSlicer refs:
  - none.
- Verification:
  - `ls modules/community-modules/dragon-curve/{Makefile,README.md,.gitignore,dragon-curve.wasm,moon.mod.json,dragon-curve.toml}` - FACT: all present.
  - `ls modules/community-modules/dragon-curve/wit/layer-infill.wit modules/community-modules/dragon-curve/wit/deps/{common/common.wit,config/config.wit,ir-types/ir-types.wit,types/types.wit}` - FACT: the frozen closure is complete.
  - `rg -q 'module-dir modules/community-modules/dragon-curve' modules/community-modules/dragon-curve/README.md` - FACT: the manual slice command is documented (note the flag is `--model`, not `--input`).
  - `cd modules/community-modules/dragon-curve && make && wasm-tools component wit dragon-curve.wasm > /dev/null` - FACT: the artifact rebuilds and is a valid component.
  - `wasm-tools component wit modules/community-modules/dragon-curve/dragon-curve.wasm | rg -q 'tool-index: option<u32>'` - FACT: 226's carrier is present in the component's world (AC-7).
- Exit condition: the module directory is complete, `make` reproduces the component, and the manual slice command is documented. AC-7 is no longer deferred — 226 has landed and `packet.spec.md` records the end-to-end evidence.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | read-only discovery; delegated verdict read |
| Step 2 | S | manifest + one dispatch |
| Step 3 | M | pure tiling + colour logic in `src/dragon/`, TDD |
| Step 4 | M | WIT glue `run` + per-region override + `tool_index` stamping |
| Step 5 | M | WIT snapshot + build/componentize + banner |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS. AC-7's 226 FORWARD-DEP has cleared; it is verified end-to-end rather than by a unit test (see `packet.spec.md` AC-7).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (packet 228 owns the backlog rows; this packet only touches TASK-338's status if 228 has not yet landed).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (the two `[FWD]` questions in `design.md`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile. Those two gates only prove this packet did not break the workspace — the module is outside the Cargo graph, so no `cargo` command builds or tests it. Its own suite is `moon test --target wasm -p slicer/layer-infill/src/dragon` (or `make test`), run from the module directory; `cargo test` there fails outright, there being no `Cargo.toml`.
