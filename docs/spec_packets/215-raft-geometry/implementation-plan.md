# Implementation Plan: raft-geometry

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-324`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Steps 1 and 2 own their complete migration blast radii before module integration begins.

## Steps

### Step 1: Inventory signed scheduled IR fallout

- Task IDs: `TASK-324`
- Objective: enumerate every definition, struct literal, conversion, fixture, and hard assertion for `GlobalLayer.index`, `ObjectLayerRef` indices, `SliceIR.global_layer_index`, and `SupportIR.global_layer_index`, while classifying preserved `u32::MAX` support-geometry sentinels.
- Precondition: current definitions are those at `crates/slicer-ir/src/slice_ir.rs:1015-1019`, `1030-1035`, `1531-1536`, and `2174-2179`.
- Postcondition: a bounded LOCATIONS inventory identifies all edit sites and all explicitly preserved sentinel/unrelated sites.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` lines 1013-1048, 1223-1234, 1529-1550, 2172-2199
  - every returned file from `rg -l 'GlobalLayer\s*\{|ObjectLayerRef\s*\{|LayerPlanIR\s*\{|SliceIR\s*\{|SupportIR\s*\{|global_layer_index:|global_support_layer_index:|u32::MAX' crates modules`
- Files allowed to edit:
  - `crates/slicer-ir/src/slice_ir.rs`
  - all returned scheduled-field literal/conversion/assertion files
  - `Cargo.toml` if workspace membership is discovered here
- Files explicitly out of bounds: WIT definitions, generated bindings, lockfiles, and preserved support-geometry sentinel consumers.
- Blast-radius discipline: the inventory is the required pre-edit LOCATIONS dispatch. It must list every struct-literal site and every test assertion against migrated fields; `crates/slicer-ir/tests/support_geometry_ir_shape_tdd.rs` is edited only if its asserted field is migrated, otherwise its `u32::MAX` sentinel remains unchanged.
- Expected sub-agent dispatches: Question: enumerate migrated-field literals, conversions, assertions, and preserved sentinels; scope `crates/**/*.rs,modules/**/*.rs`; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` signed-index section; `docs/adr/0009-raft-as-layer-infill-role.md`.
- Verification: `cargo test -p slicer-ir --all-targets`; `cargo metadata --format-version=1 --no-deps`.
- Exit condition: inventory is complete and every hit is classified before any implementation edit.

### Step 2: Migrate signed IR and schedule/capture paths

- Task IDs: `TASK-324`
- Objective: change the five scheduled fields to `i32`, update every inventoried literal/conversion/assertion, and carry signed keys through runtime schedule filtering, slice hydration, host projection, and visual-debug capture without unsigned casts.
- Precondition: Step 1 inventory is complete; WIT `layer-idx` is confirmed `s32`.
- Postcondition: negative schedule entries can be represented and selected; model layers remain non-negative; preserved support-geometry sentinel tests still assert `u32::MAX`.
- Files allowed to read: Step 1 inventory; `crates/slicer-runtime/src/layer_executor.rs` lines 1129-1171 and 623-650; `crates/pnp-cli/src/visual_debug.rs` lines 925-979 and 1371-1394; `crates/slicer-wasm-host/src/marshal/in_.rs` lines 68-95 and 566-593.
- Files allowed to edit: `crates/slicer-ir/src/slice_ir.rs`; returned IR/runtime/host/CLI literal and conversion files; affected tests and fixtures.
- Files explicitly out of bounds: unrelated `u32` scheduler/finalization fields, generated bindings, and support-geometry sentinel fields.
- Blast-radius discipline: edit every inventoried struct literal and hard assertion in this step, including test helpers and `u32::MAX` fallout for migrated `SupportIR.global_layer_index`; do not defer compile discovery to workspace checks.
- Expected sub-agent dispatches: Question: identify signed schedule lookup and capture conversions; scope `crates/slicer-runtime,crates/pnp-cli,crates/slicer-wasm-host`; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md`; `docs/adr/0009-raft-as-layer-infill-role.md`.
- Verification: `cargo test -p slicer-ir --all-targets`; `cargo test -p slicer-runtime --all-targets`; `cargo check --workspace --all-targets`.
- Exit condition: no migrated field remains `u32`, no negative scheduled value is cast to `u32`/`usize`, and signed schedule tests pass.

### Step 3: Migrate Layer::Infill SDK, macro, host, runtime, and guests

- Task IDs: `TASK-324`
- Objective: change `LayerModule::run_infill` to `i32`, remove the macro's `layer_index as u32` conversion, update host/runtime boundaries and every implementation/call site/test fixture, and preserve WIT `s32`.
- Precondition: Step 2 is green and the infill inventory is complete.
- Postcondition: SDK trait, generated guest glue, host invocation, runtime dispatch, SDK guest, macro tests, and boundary tests all use signed layer indices.
- Files allowed to read: `crates/slicer-sdk/src/traits.rs` lines 345-365; `crates/slicer-macros/src/lib.rs` lines 3073-3115; all `run_infill` inventory results; WIT layer-infill definition.
- Files allowed to edit: `crates/slicer-sdk/src/traits.rs`; `crates/slicer-macros/src/lib.rs`; all affected SDK/macro/host/runtime guest and test files returned by the inventory.
- Files explicitly out of bounds: other Layer stage signatures, generated bindings, and unrelated guest layer arguments.
- Blast-radius discipline: include every `run_infill` implementation, direct call, macro snapshot/assertion, WIT boundary fixture, and SDK guest signature returned by `rg -n 'run_infill|call_run_infill|run-infill|layer_index as u32|layer_index: u32' ...`; no follow-up workspace check may discover an omitted call site.
- Expected sub-agent dispatches: Question: enumerate exact infill contract implementations, conversions, guests, tests, and call sites; scope `crates/slicer-sdk,crates/slicer-macros,crates/slicer-wasm-host,crates/slicer-runtime,modules`; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` Layer::Infill signature; WIT `layer-infill.wit`.
- Verification: `cargo test -p slicer-macros --all-targets`; `cargo test -p slicer-wasm-host --test wit_boundary_tdd --all-targets`; `cargo test -p slicer-sdk --all-targets`.
- Exit condition: `LayerModule::run_infill` and all affected implementations use `i32`; macro glue passes WIT `i32` unchanged; WIT remains `s32`.

### Step 4: Add raft carrier and claim-holder behavior

- Task IDs: `TASK-324`
- Objective: add the `raft-default` module/tests and deterministic expanded-footprint carrier synthesis, then make rectilinear-infill hold and render `claim:raft-fill` as `RaftInfill`.
- Precondition: signed IR and infill contract steps are green.
- Postcondition: focused red tests become green for negative prefix count, clipping, determinism, no-op inputs, and claim dispatch.
- Files allowed to read: rectilinear manifest/source; relevant IR carrier and polygon helpers; ADR-0009.
- Files allowed to edit: `modules/core-modules/raft-default/`; `modules/core-modules/rectilinear-infill/`; focused SDK claim test.
- Files explicitly out of bounds: Layer::Support modules, final G-code, and support planner.
- Expected sub-agent dispatches: Question: verify polygon offset/clip helpers and existing `RaftInfill` dispatch; scope `crates/slicer-core/src,crates/slicer-sdk/src,modules/core-modules/rectilinear-infill`; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: ADR-0009; `docs/08_coordinate_system.md` delegated conversion section.
- Verification: `cargo test -p raft-default --test raft_geometry_tdd --all-targets`; `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd --all-targets`.
- Exit condition: all geometry and claim tests pass with deterministic carrier and rendered raft paths.

### Step 5: Wire fixtures, typed visual gate, and documentation

- Task IDs: `TASK-324`
- Objective: invoke negative-prefix Layer::Infill scheduling, author both visual-debug requests, document the signed contracts, and establish the typed-capture/conditional-PNG gate.
- Precondition: Steps 2-4 are green and typed capture exposes `raft_paths`.
- Postcondition: real dispatch produces non-empty typed raft output; negative PNG support is either proven or explicitly reported unsupported.
- Files allowed to read: current Layer::Infill scheduling and visual-debug request/manifest schema; docs sections named in `packet.spec.md`.
- Files allowed to edit: `crates/slicer-runtime/`, `crates/pnp-cli/`, `crates/slicer-wasm-host/` same dispatch path; `tmp/visual-debug-raft.json`; `tmp/visual-debug-raft-typed.json`; named docs.
- Files explicitly out of bounds: WIT generated output, Layer::Support dispatch, unrelated scheduler stages, and Orca source.
- Expected sub-agent dispatches: Question: determine negative selector support and typed-capture manifest path; scope `crates/pnp-cli,crates/slicer-runtime,tmp`; return `FACT`.
- Context cost: `M`
- Authoritative docs: ADR-0009; `docs/19_visual_debug.md`; architecture/schema/manifest sections.
- Verification: `cargo xtask build-guests --check`; the AC-4 conditional visual-debug command; `rg -q 'i32|raft_paths|claim:raft-fill' docs/02_ir_schemas.md`.
- Exit condition: typed `raft_paths` is non-empty for the fixture; any unsupported negative PNG selection is printed explicitly and does not count as a PNG pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Complete signed-field and assertion inventory |
| Step 2 | M | Scheduled IR and capture migration with blast radius |
| Step 3 | M | Full Layer::Infill SDK/macro/host/runtime contract migration |
| Step 4 | M | Carrier and claim-holder behavior |
| Step 5 | M | Runtime fixture, visual gate, and docs |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS, with AC-4's explicitly reported unsupported PNG branch treated as a documented limitation only when typed capture passes.
- `docs/07_implementation_status.md` is updated through a worker dispatch.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record the negative-layer PNG limitation if encountered; typed capture remains decisive.
- Confirm context stayed at or below the standard band.

All cargo commands use `--all-targets` where applicable.
