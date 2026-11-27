# Implementation Plan: 303-elefant-foot-slice-postprocess

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Kernel skeleton — signature, porting header, early-outs

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Land `crates/slicer-core/src/algos/elephant_foot.rs` with the public signatures, the mandatory OrcaSlicer porting header, and canonical's two trivial exits — `compensation <= 0.0` returns the input, and the tiny-contour test (bbox X or Y extent below `min_contour_width + 2 * compensation`, or area below `5 *` that value squared) returns the input. The interesting geometry is a `todo!()`-free but deliberately identity path until Step 2.
- Precondition: `crates/slicer-core/src/algos/mod.rs` declares `bridge_over_infill` ungated; no `elephant_foot` module exists.
- Postcondition: `cargo check -p slicer-core --all-targets` passes with `elephant_foot` compiled ungated, and AC-1 and AC-3 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mod.rs` - whole file (~26 lines)
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - first 40 lines only - the ungated module's header shape
  - `docs/ORCASLICER_ATTRIBUTION.md` - whole file
  - `docs/08_coordinate_system.md` - the mm-to-unit helper section only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/elephant_foot.rs`
  - `crates/slicer-core/src/algos/mod.rs`
  - `crates/slicer-core/tests/algo_elephant_foot_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate only)
  - `crates/slicer-core/src/arachne/**`, `crates/slicer-core/tests/arachne_*.rs`
  - `modules/**` (no module exists yet at this step)
- Blast-radius discipline: not applicable — this step adds no struct field and bumps no schema or version constant. `crates/slicer-core/Cargo.toml` is deliberately **not** edited: adding a `[[test]]` entry with `required-features` for `algo_elephant_foot_tdd` would silently empty every narrow run of it.
- Expected sub-agent dispatches:
  - Question: What is the overall pass shape of canonical `elephant_foot_compensation` — early-outs, stages in order, and the inputs each stage consumes?; scope: `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp`; return: `SUMMARY` (<=200 words, no code)
- Context cost: `S`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read; the header is mandatory
  - `docs/08_coordinate_system.md` - ranged read of the mm-to-unit helpers
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_elephant_foot_tdd zero_compensation_is_identity 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-core --test algo_elephant_foot_tdd tiny_contour_is_returned_unchanged 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `rg -q 'algo_elephant_foot_tdd' crates/slicer-core/Cargo.toml; test $? -ne 0 && echo GATE_OK || echo GATE_FAIL` - FACT: must print `GATE_OK`. **Every one of the five existing `algo_*_tdd` targets in that file carries `required-features = ["host-algos"]`** (29 `[[test]]` entries exist there solely to attach features), so adding this one "for consistency" is the single most likely way to silently blind it — the run would report `ok` with zero tests compiled.
- Exit condition: AC-1 and AC-3 pass, and `rg -q 'cfg\(feature = "host-algos"\)' crates/slicer-core/src/algos/elephant_foot.rs` finds nothing. Falsified if the kernel needs `host-algos` for anything — stop and re-scope, because a gated kernel cannot be reached from a guest.

### Step 2: Kernel body — segment grid, resample, deltas, smoothing, variable offset

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Replace Step 1's identity path with the ported algorithm: build the file-private uniform segment grid at cell size `0.7 * search_radius`, simplify at the file-private epsilon this kernel declares for itself (canonical's `SCALED_EPSILON` is not reusable — see `design.md` §Code Change Surface, Epsilon constant note), resample the contour, compute the per-point distance and derive per-point deltas, apply banded smoothing, then apply the variable inward offset and validate orientation on the way out.
- Precondition: Step 1's exit condition holds; AC-1 and AC-3 pass against the identity path.
- Postcondition: AC-2 and AC-4 pass, and AC-1 and AC-3 still pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/elephant_foot.rs` - whole file (this step's own output)
  - `crates/slicer-core/src/polygon_ops.rs` - locate `offset`, `union`, `difference` signatures with `rg -n`, then +/-20 lines each; do not read in full
  - `docs/13_slicer_helpers_crate.md` - whole file; confirms no existing primitive already does this before adding one
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/elephant_foot.rs`
  - `crates/slicer-core/tests/algo_elephant_foot_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate only)
  - `crates/slicer-core/Cargo.toml` - adding a dependency here would signal the kernel is reaching outside the ungated set; if that seems needed, stop
  - `modules/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`
- Blast-radius discipline: not applicable — no struct field or schema constant is added. The private `SegmentGrid` is file-local and has no literal sites outside this file.
- Expected sub-agent dispatches:
  - Question: Return the contour resample plus per-point delta computation, and `smooth_compensation_banded`, verbatim; scope: `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp`; return: `SNIPPETS` (<=2 snippets, 30 lines each)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - ranged read; every offset and area comparison in this step is a unit boundary
  - `docs/13_slicer_helpers_crate.md` - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_elephant_foot_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail; SNIPPETS <=20 lines on failure
  - `cargo clippy -p slicer-core --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: AC-1 through AC-4 all pass. Falsified if AC-4 passes only because the rib and the body shrink identically — that means a uniform offset was implemented and the width limiter was not; re-open the snippet dispatch rather than adjusting the test tolerance.

### Step 3: Module scaffold and manifest

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Scaffold `modules/core-modules/elefant-foot/` with `pnp_cli module new` on stage `Layer::SlicePostProcess`, then fill `elefant-foot.toml`: module header, `[stage]`, empty `[claims]`, `[ir-access]` reads/writes `["SliceIR"]`, and the seven `[config.schema]` rows (the two owned P86 keys at canonical defaults, plus `support_raft_layers`, `outer_wall_line_width`, `initial_layer_line_width`, `line_width`, `nozzle_diameter` copied verbatim from the existing perimeter manifests). Add the `slicer-core` path dependency.
- Precondition: Step 2's exit condition holds; the kernel is proven natively.
- Postcondition: `cargo check --workspace --all-targets` passes with the new workspace member, and AC-9 and AC-N2 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` - the `[module]` through `[compatibility]` block only
  - `modules/core-modules/gyroid-infill/Cargo.toml` - whole file
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` - locate the five re-declared rows with `rg -n`, then +/-10 lines each; do not read in full
  - `modules/core-modules/gyroid-infill/tests/slicer_module_binding_tdd.rs` - whole file; the binding-test shape
- Files allowed to edit (at most 3):
  - `modules/core-modules/elefant-foot/elefant-foot.toml`
  - `modules/core-modules/elefant-foot/Cargo.toml`
  - `modules/core-modules/elefant-foot/tests/slicer_module_binding_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` - no WIT change; editing here means the design is wrong
  - `crates/slicer-scheduler/**`, `crates/slicer-runtime/**` - no stage is added
  - Other modules' `*.toml` beyond the ranged reads above - copy rows, never edit them
- Blast-radius discipline: not applicable — no struct field or schema constant. `pnp_cli module new` and the root `Cargo.toml` workspace members list are the only registration surfaces; `xtask/src/build_guests.rs` discovers guests by scanning `modules/core-modules`, so no build list needs editing.
- Expected sub-agent dispatches:
  - Question: Confirm the coFloat/coInt declarations, defaults and `min` bounds of `elefant_foot_compensation` and `elefant_foot_compensation_layers`; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (<=5 lines)
  - Question: List every required `[config.schema]` field key and its allowed values for a `float` row and an `int` row; scope: `docs/03_wit_and_manifest.md`; return: `FACT` (<=5 lines)
  - Question: What is the layer-stage ownership model and what triggers the claim-system rule-4 module split?; scope: `docs/01_system_architecture.md`; return: `SUMMARY` (<=200 words)
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated `FACT` (over 300 lines)
  - `docs/01_system_architecture.md` - delegated `SUMMARY` (over 300 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p elefant-foot --test slicer_module_binding_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: AC-9 and AC-N2 pass. Falsified if the manifest needs a `[claims] holds` entry to be dispatched — that would mean rule 4 fires after all; stop and re-scope with the map's rule-4 test.

### Step 4: Gate and taper

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Implement the config-reading half of `run_slice_postprocess` in `src/lib.rs`: read the seven keys, compute `min_contour_width` once via `resolve_role_width(ExtrusionRole::OuterWall, layer_index == 0, false, &ctx)` plus `line_width_to_spacing(width, effective_layer_height)`, and compute the per-layer compensation from canonical's gate and taper. Write no polygons yet — the function returns `Ok(())` after computing the value, which Steps 4's tests observe through a unit-testable helper.
- Precondition: Step 3's exit condition holds; the manifest resolves and the module is dispatchable.
- Postcondition: AC-6, AC-7 and AC-N1 pass; AC-5 passes trivially (no writes exist yet).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - the `RoleWidthContext`, `resolve_role_width`, `line_width_to_spacing`, `NegativeSpacingError` definitions only
  - `crates/slicer-sdk/src/views.rs` - the `SliceRegionView` accessor block only (large file)
  - `modules/core-modules/classic-perimeters/src/lib.rs` - the auto-sentinel width resolution comment block only; locate with `rg -n '1.125'` then +/-25 lines
  - `docs/21_data_defaults_and_fixtures.md` - whole file; governs the `RoleWidthContext` literals in the new tests
- Files allowed to edit (at most 3):
  - `modules/core-modules/elefant-foot/src/lib.rs`
  - `modules/core-modules/elefant-foot/tests/elefant_foot_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/flow.rs` - read-only; this packet changes no flow arithmetic
  - `crates/slicer-sdk/**` - read-only
  - `OrcaSlicerDocumented/**` (delegate only)
- Blast-radius discipline: `RoleWidthContext` is a watched type (a `pub` struct with >=5 named fields under `crates/*/src`). Production `src/` literals stay exhaustive; every literal in `tests/elefant_foot_tdd.rs` must use a `..` rest or carry an `// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`. Both edited files are in this step's edit list, so the fallout is budgeted here rather than discovered by a later `cargo xtask check-literals`.
- Expected sub-agent dispatches:
  - Question: In `PrintObject::slice_volumes`, what exactly gates elephant-foot compensation and how is the per-layer value computed?; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SUMMARY` (<=200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read; the `check-literals` waiver format
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load
- Verification:
  - `cargo test -p elefant-foot --test elefant_foot_tdd taper_matches_canonical_per_layer_formula 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p elefant-foot --test elefant_foot_tdd raft_present_disables_compensation 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p elefant-foot --test elefant_foot_tdd negative_spacing_is_fatal_module_error 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask check-literals` - FACT exit code
- Exit condition: AC-6, AC-7 and AC-N1 pass and `check-literals` exits `0`. Falsified if `outer_wall_line_width` cannot be resolved through the config view's `get_abs_value` — in that case record the blocker against ticket 128 rather than reading the raw magnitude.

### Step 5: Per-region write-back

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Complete `run_slice_postprocess`: for each dispatched region, run the kernel over that region's `polygons` at the Step 4 compensation, `union` the result, and emit exactly one `set_polygons` call with a `RegionKey` carrying that region's own `object_id`, `region_id` and `variant_chain` plus the dispatched `layer_index`. Emit nothing when the compensation is zero.
- Precondition: Step 4's exit condition holds; the compensation value is correct and tested.
- Postcondition: AC-5 and AC-8 pass; AC-6 and AC-7 still pass now that writes exist.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/test-guests/dispatch-layer-slice-postprocess-guest/src/lib.rs` - whole file (~50 lines); the `RegionKey` construction precedent
  - `crates/slicer-sdk/src/builders.rs` - the `impl SlicePostprocessBuilder` block only (large file)
  - `crates/slicer-schema/wit/deps/ir-types.wit` - the `resource slice-region-view` and `resource slice-postprocess-builder` blocks only
- Files allowed to edit (at most 3):
  - `modules/core-modules/elefant-foot/src/lib.rs`
  - `modules/core-modules/elefant-foot/tests/elefant_foot_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` - read-only; a WIT edit here falsifies the design
  - `crates/slicer-runtime/src/layer_executor.rs` - delegate any question about commit-merge behaviour
  - `modules/core-modules/skirt-brim/**` - `brim_use_efc_outline` is out of scope
- Blast-radius discipline: not applicable — no struct field or schema constant is added; `RegionKey` is constructed, not extended.
- Expected sub-agent dispatches:
  - Question: Which consumers read `SliceRegionView`'s `infill_areas`, `top_solid_fill`, `bottom_solid_fill`, `internal_solid_fill` or `sparse_infill_area` at `Layer::Infill`, and are any of them fed from the pre-compensation footprint?; scope: `modules/core-modules/*/src/**`, `crates/slicer-runtime/src/**`; return: `LOCATIONS` (<=20 entries); purpose: resolve the first `[FWD]` question in `design.md`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated `FACT` on whether `SlicedRegion.polygons` is the sole footprint source for the downstream fill areas
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load (the `union_ex` of the kernel result)
- Verification:
  - `cargo test -p elefant-foot --test elefant_foot_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail; SNIPPETS <=20 lines on failure
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: AC-5, AC-6, AC-7 and AC-8 all pass. Falsified if the dispatched `variant_chain` cannot be read from `SliceRegionView` — AC-8 then cannot be satisfied without a WIT change, which is out of scope; stop and re-scope.

### Step 6: Guest build, edition wiring and freshness

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Build the guest component and confirm its embedded WIT world resolves to `slicer:layer-slice-postprocess/slice-postprocess-module`. Edition membership needs **no** edit and is not a decision this step makes — `design.md` §Open Questions records why: modules are discovered dynamically by `discover_guests` filtered to `GuestTree::Core`, `dist/editions.toml` names only the three natively-integrated `hybrid` modules, and `elefant-foot` ships as a WASM guest in all three editions by default. This step's job is to *confirm* that by observing the guest is discovered without touching either file.
- Precondition: Step 5's exit condition holds; the module is complete and natively tested.
- Postcondition: AC-10 passes with exit `0`, and the module's edition membership is explicit rather than incidental.
- Files allowed to read, with ranges when over 300 lines:
  - `dist/editions.toml` - the `[edition.*]` blocks only (~15 lines); confirms `integrated_modules` names only `classic-perimeters`, `arachne-perimeters`, `tree-support-planner`
  - `xtask/src/editions.rs` - locate `load_editions_from` and `validate_edition_names` with `rg -n`, then +/-20 lines each; confirms names are validated against `discover_guests`, not against a hardcoded list
  - `xtask/src/build_guests.rs` - locate the `core-modules` scan with `rg -n`, then +/-20 lines; confirms no build list needs an entry
- Files allowed to edit (at most 3):
  - `modules/core-modules/elefant-foot/Cargo.toml` (only if the guest build reveals a missing `wit-bindgen` target dependency)
  - *(no others — this step is expected to be read-and-verify. Editing `dist/editions.toml` or `xtask/src/editions.rs` is out of bounds: naming the module in `editions.toml` would make it a natively-integrated `hybrid` module, which is a performance decision this packet has not made and did not measure.)*
- Files explicitly out of bounds:
  - `dist/editions.toml`, `xtask/src/editions.rs` - read-only; see the edit-list note above
  - `crates/slicer-wasm-host/test-guests/**` - unrelated guests; never rebuild or edit them by hand
  - `target/**`, `Cargo.lock`
- Blast-radius discipline: not applicable — no struct field or schema constant, and no hardcoded module enumeration exists to extend.
- Expected sub-agent dispatches:
  - Question: After the guest builds, does `discover_guests` list `elefant-foot` under `GuestTree::Core`, and does `cargo xtask dist --edition developer` stage its `.wasm` without any `dist/editions.toml` edit?; scope: `xtask/src/{editions.rs,build_guests.rs,dist.rs}`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - none beyond the code read above
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code; `0` fresh, `1` stale, `3` `wasm-tools` missing. Never grep for `STALE:`
  - `test -f modules/core-modules/elefant-foot/elefant-foot.wasm; echo "exit=$?"` - FACT exit code
- Exit condition: `cargo xtask build-guests --check` exits `0`, the artifact exists, and neither `dist/editions.toml` nor `xtask/src/editions.rs` was modified. Falsified by exit `3` — that is a missing `wasm-tools`, an infrastructure error, not a clean result; install it and re-run before concluding anything. Also falsified if the guest is *not* discovered without an `editions.toml` entry, which would contradict the authoring-time reading of `validate_edition_names` and must be re-scoped rather than patched around.

### Step 7: Wayfinder asset annotation and deviation row

- Task IDs: none (wayfinder queue packet; `task_ids: []`)
- Objective: Annotate the two wayfinder assets with the packet linkage and file the deviation row covering the three divergences in `requirements.md` §Cross-packet impact.
- Precondition: Step 6's exit condition holds; the behaviour is landed and building.
- Postcondition: AC-11 passes and every grep in the packet's Doc Impact Statement resolves.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - locate the `elefant_foot` rows with `rg -n`, then +/-10 lines; never read in full
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - locate the P86 entry with `rg -n`, then +/-10 lines; never read in full
  - `docs/DEVIATION_LOG.md` - the table header and one recent row only; never read in full
- Files allowed to edit (at most 3):
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - `docs/specs/orca-feature-gap/map.md` - the wayfinder session owns the map, not the implementer
  - `docs/specs/orca-feature-gap/issues/93-*.md` - the wayfinder session owns the ticket resolution
  - Every other `docs/spec_packets/*/` directory
- Blast-radius discipline: not applicable — no struct field or schema constant.
- Expected sub-agent dispatches:
  - Question: What is the current highest `DEV-###` across `docs/DEVIATION_LOG.md` and `docs/spec_packets/*/`?; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/*/*.md`; return: `FACT` (<=5 lines). **Re-derive at this moment — the ID recorded at authoring is a ledger fact and will have rotted.**
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - ranged read of the header and one recent row for the row format
- OrcaSlicer refs:
  - none — every canonical citation in the row is by file and function, never line number
- Verification:
  - `cargo xtask check-deviations` - FACT exit code
  - `rg -q 'elefant_foot_compensation.*303-elefant-foot' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md && rg -q '303-elefant-foot' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md && rg -q 'elefant_foot_compensation' docs/DEVIATION_LOG.md; echo "exit=$?"` - FACT exit code
- Exit condition: `check-deviations` exits `0` and every Doc Impact grep resolves. Falsified if the re-derived `DEV-###` collides with a row filed by a parallel packet — take the next free ID and re-run rather than reusing the authoring-time value.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Signatures, porting header, two early-outs; one bounded SUMMARY dispatch |
| Step 2 | M | The ported algorithm; the packet's heaviest step — split at the smoothing boundary if it overruns |
| Step 3 | S | Scaffold plus seven manifest rows, five copied verbatim |
| Step 4 | M | Config reads, width derivation, gate and taper; carries the `check-literals` fallout |
| Step 5 | M | Per-region write-back plus the downstream-fill-areas `LOCATIONS` dispatch |
| Step 6 | S | Guest build and freshness; edition membership confirmed, not edited |
| Step 7 | S | Two asset annotations and one deviation row |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` needs **no** update: this is a wayfinder queue packet with `task_ids: []` and no `TASK-###` slice. Confirm by dispatching a worker to check that no `TASK-###` row references elephant-foot; never read the backlog in full.
- No reopened or superseded packet to reconcile.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo test -p slicer-core --features host-algos --no-fail-fast` and reconcile the binary count against the previous known-good count before trusting any narrow `slicer-core` result — a count drop means the narrow run was blind, never the reverse (`CLAUDE.md` §"Feature-gated test files report green when they don't compile").
- Run the whole-suite gate through `cargo xtask test --summary --workspace` so the guest-freshness preflight fires; never plain `cargo test --workspace`.
- Record remaining packet-local risk: first production occupancy of `Layer::SlicePostProcess`, and the substituted acceleration structure's tolerance versus canonical.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
