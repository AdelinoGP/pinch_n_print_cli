# Implementation Plan: wipe-tower-bed-exclude-area

Nine atomic steps, strictly ordered. Each names its own falsifying exit condition; a step is not done until that condition is checked and false.

**Ledger-fact discipline:** the core-module count, and whether `254b` has landed, are re-derived from disk inside the step that needs them (`ls -d modules/core-modules/*/ | wc -l`, and the `status:` line of `docs/spec_packets/254b-prime-tower-interface-and-ramming/packet.spec.md`). No number in this plan is authoritative.

---

## Step 1 — Create the `print-validator` module

- **Task IDs:** none (queue packet; recorded against wayfinder ticket 11).
- **Objective:** a new core module at `PrePass::MeshAnalysis` that parses `bed_exclude_area`, pre-filters objects by `object-bounds`, probes the excluded region with `raycast-z-down`, and returns a **fatal** `ModuleError` on a hit.
- **Preconditions:** model the crate on `modules/core-modules/layer-planner-default/` (Cargo.toml, `wit-guest/`, `src/`, `tests/`). Confirm from `crates/slicer-sdk/src/traits.rs` that `PrepassModule::run_mesh_analysis` is the method and what `MeshAnalysisOutput` offers — the module calls neither `push_facet_annotation` nor `push_surface_group`.
- **Postconditions:** AC-2 (module half), AC-4, AC-5, AC-6 and AC-N3 pass. The module is not yet registered anywhere, so no other test changes.
- **Allowed reads:** `modules/core-modules/layer-planner-default/**`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit`, `crates/slicer-schema/wit/deps/common.wit`, `modules/core-modules/wipe-tower/src/lib.rs` (`point_in_polygon`, `float_list_from_config` as the parsing precedent).
- **Files allowed to edit (≤3, counting the new crate as one unit):** `modules/core-modules/print-validator/{Cargo.toml, print-validator.toml, wit-guest/**, src/lib.rs}` (the new crate), `modules/core-modules/print-validator/tests/bed_exclusion_tdd.rs`.
- **Out of bounds:** `crates/slicer-schema/wit/**` (no WIT edit — raise a `[BLOCK]` instead), `crates/slicer-runtime/src/run.rs`, `crates/slicer-core/src/algos/mesh_analysis.rs`.
- **Dispatches:** `SNIPPETS` ≤ 1 × 30 lines for the `PrepassModule` trait shape.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-1, AC-2, AC-4, AC-5, AC-6; `design.md` § Selected Approach, DIV-1.
- **Verification:** `cargo test -p print-validator --test bed_exclusion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** a degenerate `bed_exclude_area` (single point, or an odd float count) produces an error instead of `Ok(())`, or the rejection is constructed without `fatal` set. Either makes the module actively wrong: the first fails valid prints, the second silently never rejects anything.

---

## Step 2 — Register the module

- **Task IDs:** none.
- **Objective:** the module is a workspace member, is present in the integrated registry and the CLI passthrough features, and the scheduler's core-module discovery count matches the tree.
- **Preconditions:** Step 1 landed. **Re-derive** the current directory count and the exact assertion in `core_modules_directory_is_discoverable_and_all_load`; check whether `254b` has already incremented it.
- **Postconditions:** AC-1 passes; `cargo check --workspace --all-targets` is clean; `cargo xtask build-guests --check` returns exit 0 (the new guest is discovered).
- **Allowed reads:** root `Cargo.toml`, `crates/slicer-integrated-modules/{Cargo.toml, src/lib.rs}`, `crates/pnp-cli/Cargo.toml`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- **Files allowed to edit (≤3):** root `Cargo.toml` + `crates/slicer-integrated-modules/{Cargo.toml, src/lib.rs}` (registration unit), `crates/pnp-cli/Cargo.toml`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- **Out of bounds:** module source (Step 1 owns it), `crates/slicer-runtime/src/**`.
- **Dispatches:** `LOCATIONS` ≤ 10 — every place a core module must be registered, as the tree stands now. `FACT` — the re-derived count.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-1; `design.md` § Code Change Surface.
- **Verification:** `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` then `cargo xtask build-guests --check; echo "exit=$?"`
- **Falsifying exit condition:** the count assertion passes against a number copied from this packet rather than re-derived, or `build-guests --check` returns 3 (`wasm-tools` missing) and is read as clean.

---

## Step 3 — Prove the abort path end to end

- **Task IDs:** none.
- **Objective:** a slice whose object occupies the excluded region fails with `PipelineError::Prepass` / `PrepassExecutionError::FatalModule`, a slice whose object is clear is unaffected, and a slice with no `bed_exclude_area` is byte-identical to baseline.
- **Preconditions:** Steps 1–2 landed. The new test file **must** be added to `crates/slicer-runtime/tests/integration/main.rs`'s `mod` list in this same step — an unregistered file compiles to zero tests and reports green.
- **Postconditions:** AC-2 (abort half), AC-3, AC-N2 pass.
- **Allowed reads:** `crates/slicer-runtime/tests/integration/main.rs`, one existing slice-driving integration test as a fixture template, `crates/slicer-runtime/src/prepass.rs` (fatal path — ranged read only).
- **Files allowed to edit (≤3):** `crates/slicer-runtime/tests/integration/bed_exclusion_abort_tdd.rs` (new), `crates/slicer-runtime/tests/integration/main.rs`.
- **Out of bounds:** `crates/slicer-runtime/src/**` — this packet adds no host logic.
- **Dispatches:** `FACT` for the test run.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-2, AC-3, AC-N2; `requirements.md` § In-Tree Grounding (fatal propagation).
- **Verification:** `cargo test -p slicer-runtime --test integration bed_exclusion_abort_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** the run reports `0 passed; 0 failed` — that is the unregistered-file false green, not a pass. Confirm the test count is non-zero before recording the step done.

---

## Step 4 — The wipe-tower half

- **Task IDs:** none.
- **Objective:** `wipe-tower` declares `bed_exclude_area` and rejects a tower footprint corner inside the exclusion polygon at its existing code-3 site.
- **Preconditions:** re-derive the current `wipe-tower.toml` key set from disk (`254a` / `254b` / `255` may have landed) and `rg -n 'bed_exclude_area' crates modules resources` to confirm no existing fixture is about to start failing.
- **Postconditions:** AC-7 and AC-N1 pass; with the key absent, tower behaviour is unchanged.
- **Allowed reads:** `modules/core-modules/wipe-tower/{wipe-tower.toml, src/lib.rs, tests/bed_bounds_tdd.rs}`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`.
- **Files allowed to edit (≤3):** `modules/core-modules/wipe-tower/wipe-tower.toml` + `modules/core-modules/wipe-tower/src/lib.rs` (one unit), `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`.
- **Out of bounds:** `ORCA_CONFIG_PADDING`, the new module's source.
- **Dispatches:** `FACT` — `cargo xtask build-guests --check; echo "exit=$?"` at step exit (the wipe-tower manifest and `src/` are guest-fingerprint inputs).
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-7, AC-N1; `design.md` DIV-2.
- **Verification:** `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` then `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** the tower check fires when `bed_exclude_area` is absent, or a degenerate value makes the tower path fail — the module-side and tower-side degenerate semantics must agree.

---

## Step 5 — Docs

- **Task IDs:** none.
- **Objective:** `docs/15_config_keys_reference.md` is regenerated and current; `docs/04_host_scheduler.md` records that `PrePass::MeshAnalysis` now hosts a guest validator beside its host built-in and that a fatal module error there aborts the slice.
- **Preconditions:** Steps 1–4 landed.
- **Postconditions:** AC-8 passes and the Doc Impact grep returns exit 0.
- **Allowed reads:** `docs/04_host_scheduler.md` (ranged — the prepass/stage-order section only).
- **Files allowed to edit (≤3):** `docs/04_host_scheduler.md`, `docs/15_config_keys_reference.md` (**generated only** — via `cargo xtask gen-config-docs`, never by hand).
- **Out of bounds:** any code file.
- **Dispatches:** `FACT` for each command.
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-8 and § Doc Impact Statement.
- **Verification:** `cargo xtask gen-config-docs --check && rg -q 'bed_exclude_area' docs/15_config_keys_reference.md; echo "exit=$?"` then `rg -q 'print-validator' docs/04_host_scheduler.md; echo "exit=$?"`
- **Falsifying exit condition:** `docs/15` was edited by hand rather than regenerated (`--check` would pass locally and fail in CI after any manifest change).

---

## Step 6 — The travel-path key surface in `machine-gcode-emit`

- **Task IDs:** none.
- **Objective:** `machine-gcode-emit.toml` declares `start_end_points`, `bed_exclude_area` and the six host-injected `travel_point_*` keys. **Manifest-only — no `src/lib.rs` change**: the existing `config.keys()` sweep resolves any declared key, preserving ADR-0050 §2's placeholder domain exactly.
- **Preconditions:** re-derive the current `machine-gcode-emit.toml` key set from disk (packets 253, 267 and the P18/P20 work may have landed), `rg -n 'travel_point' modules crates` to confirm no existing fixture is about to start resolving placeholders that previously passed through verbatim, and re-derive the module's schema-guard tests (any asserting an exact key set must be updated in this step).
- **Postconditions:** AC-9's manifest half holds; with no `travel_point_*` values in the config source (single-tool), the lookup is unchanged and templates referencing them keep the existing verbatim pass-through behavior.
- **Allowed reads:** `modules/core-modules/machine-gcode-emit/{machine-gcode-emit.toml, src/lib.rs, tests/machine_gcode_emit_tdd.rs}`, `docs/adr/0050-custom-gcode-architecture.md` (§2 — the conformance constraint).
- **Files allowed to edit (≤3):** `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (schema-guard re-derivation only).
- **Out of bounds:** `crates/slicer-schema/wit/**` (no WIT edit — raise a `[BLOCK]` instead), `crates/slicer-runtime/src/**` (Step 7 owns the host half), `ORCA_CONFIG_PADDING`.
- **Dispatches:** `SNIPPETS` ≤ 1 × 30 lines for the `config.keys()` sweep in `run_gcode_postprocess` (confirm a declared key resolves with no code change); `FACT` — `cargo xtask build-guests --check; echo "exit=$?"` at step exit (the manifest is a guest-fingerprint input).
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-9; `design.md` § Selected Approach (the travel path), DIV-3, DIV-4.
- **Verification:** `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` then `cargo xtask build-guests --check; echo "exit=$?"`
- **Falsifying exit condition:** the module's substitution resolves a `travel_point_*` placeholder from anything other than a manifest-declared key (ADR-0050 §2 violation), or the schema-guard tests were updated to a key set not re-derived from disk.

---

## Step 7 — The host travel-path computation and injection

- **Task IDs:** none.
- **Objective:** `crates/slicer-runtime/src/travel_path.rs` (new) holds `compute_travel_path` — the canonical `get_path_of_change_filament` algorithm as a pure function — and `run_slice` computes per-object XY bounds from `mesh_ir.objects`, extracts the multi-tool predicate into a shared helper, and injects the six `travel_point_*` values into `config_source`, gated on multi-tool. Single-tool prints get no injection.
- **Preconditions:** Step 6 landed (the module declares the keys). Re-derive the exact `config_source` seeding block and the MMU auto-enable predicate from `crates/slicer-runtime/src/run.rs` as they stand now.
- **Postconditions:** AC-10 and AC-N4 pass (unit arms).
- **Allowed reads:** `crates/slicer-runtime/src/run.rs` (the seeding block, ranged), `crates/slicer-model-io/src/loader.rs` (`compute_z_extent_from_mesh` / `object_world_z_extent_from_mesh_and_transform` as the bounds pattern, ranged), `crates/slicer-ir/src/slice_ir.rs` (`ObjectMesh` / `IndexedTriangleSet` shapes, ranged).
- **Files allowed to edit (≤3):** `crates/slicer-runtime/src/travel_path.rs` (new), `crates/slicer-runtime/src/run.rs` (the injection + the shared multi-tool helper).
- **Out of bounds:** `crates/slicer-schema/wit/**`, `crates/slicer-gcode/src/serialize.rs`, module source (Step 6 owns it), `crates/slicer-runtime/tests/**` (Step 8 owns the integration arms).
- **Dispatches:** `FACT` for the test run.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-10, AC-N4; `design.md` DIV-3, DIV-4.
- **Verification:** `cargo test -p slicer-runtime --lib travel_path 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** the path computation deviates from canonical's algorithm on any traced input (safe default, `+2` margins, `[0, 255]` constant, four-branch `new_path` selection, left/right travel).

---

## Step 8 — The integration arms and the end-to-end substitution

- **Task IDs:** none.
- **Objective:** `crates/slicer-runtime/tests/integration/travel_path_injection_tdd.rs` (new) asserts AC-11 (single-tool slices emit no `travel_point_` line; multi-tool slices emit exactly six `travel_point_*` lines with the computed values) and AC-9's substitution half (a multi-tool slice whose `change_filament_gcode` template references `[travel_point_1_x]` … `[travel_point_3_y]` resolves every placeholder; with `start_end_points` / `bed_exclude_area` absent, the safe-default path resolves). The file **must** be registered in `crates/slicer-runtime/tests/integration/main.rs` in this same step — an unregistered file compiles to zero tests and reports green.
- **Preconditions:** Steps 6–7 landed.
- **Postconditions:** AC-9's substitution half and AC-11 hold through the real pipeline; the six placeholders resolve with no unresolved-key warning naming them.
- **Allowed reads:** one existing multi-tool slice-driving integration test as a fixture template (e.g. `cube_4color_gcode_output_tdd.rs`), `crates/slicer-runtime/tests/integration/main.rs`.
- **Files allowed to edit (≤3):** `crates/slicer-runtime/tests/integration/travel_path_injection_tdd.rs` (new), `crates/slicer-runtime/tests/integration/main.rs` (`mod travel_path_injection_tdd;` registration).
- **Out of bounds:** `crates/slicer-runtime/src/**`, module source.
- **Dispatches:** `FACT` for the test run.
- **Cost:** `M`.
- **Authorities:** `packet.spec.md` AC-9, AC-11; `design.md` § Selected Approach (the travel path).
- **Verification:** `cargo test -p slicer-runtime --test integration travel_path_injection_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Falsifying exit condition:** a configured `start_end_points` / `bed_exclude_area` pair does not change the resolved travel points in the emitted G-code (the key is not live end to end), a single-tool slice emits a `travel_point_` line (the injection gate is wrong), or the run reports `0 passed; 0 failed` (the unregistered-file false green).

---

## Step 9 — Docs (travel path)

- **Task IDs:** none.
- **Objective:** `docs/15_config_keys_reference.md` is regenerated with `start_end_points` and the six `travel_point_*` keys under owner `machine-gcode-emit` and `bed_exclude_area` under all three owners; the Orca-deviations table gains no row.
- **Preconditions:** Steps 6–8 landed.
- **Postconditions:** AC-8's extended grep returns exit 0.
- **Allowed reads:** none beyond the verification commands.
- **Files allowed to edit (≤3):** `docs/15_config_keys_reference.md` (**generated only** — via `cargo xtask gen-config-docs`, never by hand).
- **Out of bounds:** any code file.
- **Dispatches:** `FACT` for each command.
- **Cost:** `S`.
- **Authorities:** `packet.spec.md` AC-8.
- **Verification:** `cargo xtask gen-config-docs --check && rg -q 'bed_exclude_area' docs/15_config_keys_reference.md && rg -q 'start_end_points' docs/15_config_keys_reference.md && rg -q 'travel_point_1_x' docs/15_config_keys_reference.md; echo "exit=$?"`
- **Falsifying exit condition:** `docs/15` was edited by hand rather than regenerated.

---

## Closure Gate

Run in this order, all delegated with `FACT` returns:

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask check-literals 2>&1 | tail -3`
4. `cargo xtask build-guests --check; echo "exit=$?"` — must be `0`
5. Every command in `requirements.md` § Verification Matrix

`cargo test --workspace` runs only if this packet's acceptance ceremony demands it, and then only through `cargo xtask test --summary --workspace`, dispatched to a sub-agent returning `FACT pass/fail`.

## Reporting Obligations (not code changes)

The implementer/reviewer reports these upward; this packet edits neither the map nor the tickets.

- The map's ticket-11 entry describes packet 256 as wiring "only the wipe-tower corner check" with the object-footprint check recorded as a gap. That is now the *secondary* half; the entry needs updating to the module-based validator plus DIV-1's sampled-probe divergence.
- The map's ticket-40 entry returned `start_end_points` to the queue as unimplemented, blocked on packet 256's implementation. Once this revision lands, the tier-table row and the fog entry need updating to "live via packet 256".
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` lists `bed_exclude_area` as Tier A with owner "wipe-tower (bed_shape) + crates/slicer-gcode (printable_height)". The owner is now `print-validator` (primary) + `wipe-tower` (secondary) + `machine-gcode-emit` (travel-path key surface, computation in `slicer-runtime`), and the tier is C. The `start_end_points` row's "blocked, unimplemented" annotation is superseded.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` P04 row: now two keys.
- `docs/specs/orca-feature-gap/issues/key-correction-inventory.md`'s `bed_exclude_area` row (`NOT-YET-BUILT` / "Packet 256 claims WIRED… Verified false") should be updated once this revision lands.
- The two returned canonical consumers (`apply_config`, `construct_printable_area_by_printer`) belong in the map's fog list as future decision points, not as key coverage.
- The `[FWD]` host service for a true per-object footprint polygon (DIV-1) is a WIT-level follow-up worth its own queue entry.
