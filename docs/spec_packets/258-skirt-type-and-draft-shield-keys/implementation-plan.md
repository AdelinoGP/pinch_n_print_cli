# Implementation Plan: skirt-type-and-draft-shield-keys

Steps are ordered and atomic. Steps 2 and 3 must land in one commit (see `requirements.md` §Step Completion Expectations). No step is rated L.

---

## Step 1 — Declare the seven keys in the `skirt-brim` manifest and guard them

- Task IDs: none (this packet's backlog slice is the wayfinder map's P06, not a `docs/07` `TASK-###`).
- Objective: make all seven keys visible to the module through `bind_module_config_view`, and pin their canonical shape against drift.
- Precondition: `modules/core-modules/skirt-brim/skirt-brim.toml` has six `[config.schema.*]` tables (`skirt_brim_enabled`, `skirt_loops`, `skirt_distance`, `skirt_height`, `brim_width`, `line_width`), plus whatever `257a`/`257b` added if they landed first.
- Postcondition: the manifest additionally declares `skirt_type`, `min_skirt_length`, `skirt_start_angle`, `draft_shield`, `single_loop_draft_shield`, `filament_diameter`, `layer_height` with the exact types/defaults/bounds in AC-1; `tests/skirt_config_schema_tdd.rs` exists and passes.
- Allowed reads: `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/classic-perimeters/classic-perimeters.toml` (host-key declaration precedent), `crates/slicer-scheduler/src/execution_plan.rs` located window around `bind_module_config_view`.
- Files allowed to edit (3): `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/skirt-brim/tests/skirt_config_schema_tdd.rs` (new), `modules/core-modules/skirt-brim/Cargo.toml` (add `toml = "0.8"` dev-dependency — verified absent at authoring, so this add is required).
- Out of bounds: `src/lib.rs` (no wiring in this step), every other module, `ORCA_CONFIG_PADDING`.
- Dispatches: `cargo test -p skirt-brim --test skirt_config_schema_tdd` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence (types/defaults/bounds); `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative).
- Verification: `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-1, AC-N3), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: the guard fails, or `build-guests --check` returns non-zero after a rebuild, or any declared default differs from `requirements.md`'s table.

---

## Step 2 — Export `filament_diameter` from `ResolvedConfig::to_config_map`

- Objective: make the already-resolved filament diameter reach modules that declare it.
- Precondition: `ResolvedConfig::filament_diameter: f32` exists (`crates/slicer-ir/src/resolved_config.rs`, declared via the `cli @filament "filament_diameter" ... extract_float_or_first` macro arm) and `to_config_map` does not export it.
- Postcondition: `to_config_map` emits `("filament_diameter", ConfigValue::Float(f64::from(self.filament_diameter)))`, placed adjacent to the existing `filament_density` export.
- Allowed reads: `crates/slicer-ir/src/resolved_config.rs` — located windows around `to_config_map` and around the `filament_diameter` macro arm only. **The file is far over the 600-line ceiling; never read it in full.**
- Files allowed to edit (1): `crates/slicer-ir/src/resolved_config.rs`.
- Out of bounds: `crates/slicer-gcode/src/serialize.rs` (that is Step 3, but see the commit note below), every packet directory.
- Dispatches: `cargo check --workspace --all-targets` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `design.md` §Which existing mechanism carries the new data; §Architecture Constraints "Blast radius — `to_config_map`".
- Verification: `cargo check --workspace --all-targets` and the existing `to_config_map` round-trip tests in the same file: `cargo test -p slicer-ir --lib resolved_config 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Falsifying exit: any existing `to_config_map` assertion that pins the exported key set now fails and is "fixed" by weakening it rather than by adding the expected key — that is a gate-gaming stop, not a pass.
- **Commit note:** do not commit Step 2 without Step 3. Alone, it emits a bare scalar `; filament_diameter = 1.75` into the CONFIG_BLOCK and breaks OrcaSlicer's filament-count inference (`design.md` R-1).

---

## Step 3 — Build the CONFIG_BLOCK filament array from the resolved scalar

- Objective: preserve the comma-joined per-tool `filament_diameter` array the viewer requires, while sourcing its value from the resolved config instead of a hardcoded literal.
- Precondition: `serialize_config_block` (`crates/slicer-gcode/src/serialize.rs`) synthesizes `vec!["1.75"; filament_count].join(",")` guarded by `if !raw_config.contains_key("filament_diameter")`.
- Postcondition: the branch fires when the key is absent **or** present as a non-`List` scalar, and joins that scalar `filament_count` times; a `ConfigValue::List` in `raw_config` still passes through the generic dump path verbatim. The hardcoded `"1.75"` literal is gone.
- Allowed reads: `crates/slicer-gcode/src/serialize.rs` — located windows around `serialize_config_block`'s synthetic-key section, the `ConfigValue::List` join arm, and `mod tests`. **Never read the file in full; never open `ORCA_CONFIG_PADDING`.**
- Files allowed to edit (1): `crates/slicer-gcode/src/serialize.rs` (implementation + its `mod tests` arm).
- Out of bounds: `ORCA_CONFIG_PADDING` and every padding twin (map Authoring rule 2 — touching it fails the packet); `crates/slicer-runtime/` sources.
- Dispatches: `cargo test -p slicer-gcode --lib serialize::tests` → FACT pass/fail; on failure SNIPPETS ≤ 20 lines.
- Context cost: **S**.
- Authorities: `requirements.md` §Recorded Divergences D-258-4; `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract (delegated SUMMARY).
- Verification: `cargo test -p slicer-gcode --lib serialize::tests::config_block 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-8), then `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Falsifying exit: the emitted line is a bare scalar, or the entry count stops matching `filament_count`, or `ORCA_CONFIG_PADDING` was edited.

---

## Step 4 — Wire `draft_shield` and `single_loop_draft_shield` (span + per-layer loop count)

- Objective: make the two shield keys change emitted geometry.
- Precondition: Step 1 landed. `SkirtBrim` computes `max_layer = (self.skirt_height as usize).min(layers.len())` identically in `process` and `run_finalization`.
- Postcondition: `SkirtBrim` carries `draft_shield_enabled: bool` and `single_loop_draft_shield: bool` from `from_config`; the span becomes `if draft_shield_enabled && skirt_loops > 0 { layers.len() } else { min(skirt_height, layers.len()) }` in **both** functions; `generate_skirt_entities` emits only ring index 0 when `single_loop_draft_shield` is set and the layer is not the first of the span.
- Allowed reads: `modules/core-modules/skirt-brim/src/lib.rs` (in full — 421 lines at authoring, under the ceiling), `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`.
- Out of bounds: every crate under `crates/`, every other module, the manifest (Step 1 owns it).
- Dispatches: `cargo test -p skirt-brim --test finalization_live_tdd` → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence rows for `draft_shield` / `single_loop_draft_shield`; `design.md` §Selected Approach step 3.
- Verification: `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-2, AC-3), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: the legacy `process()` path and `run_finalization` disagree on the span (both must be updated), or the default path's push count changes.

---

## Step 5 — Wire `skirt_start_angle` (start-corner rotation)

- Objective: make the start angle change the first-layer innermost ring's start vertex.
- Precondition: Step 4 landed.
- Postcondition: `SkirtBrim` carries `skirt_start_angle: f32`; a private `rotate_rect_start` re-orders the rect's four distinct vertices to begin at the corner nearest `bbox_center + r·(cos θ, sin θ)` (`r` = half-diagonal of that ring's rect) and re-closes the loop at 5 points; the rotation is applied only to the first ring of the first layer of the span, matching canonical's `first_layer && i == loops.first`.
- Allowed reads: `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`.
- Out of bounds: as Step 4.
- Dispatches: `cargo test -p skirt-brim --test skirt_brim_tdd` → FACT pass/fail. If the corner-selection formula is disputed, dispatch a SUMMARY read of `Skirt::find_start_point` in `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — never read it in-context.
- Context cost: **S**.
- Authorities: `requirements.md` §Recorded Divergences D-258-2 (innermost- vs outermost-first) and D-258-5 (corner, not mid-edge).
- Verification: `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-4), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: at the default `-135.0` any emitted vertex differs from the pre-packet sequence, or a rotated loop has other than 5 points or is not closed (INV-2).

---

## Step 6 — Build `skirt_type` per-object grouping with envelope merging

- Objective: make `"perobject"` emit one ring set per non-touching object cluster.
- Precondition: Step 5 landed.
- Postcondition: a private `skirt_groups` partitions span entities by `region_key.object_id` into per-object `BBox2D`s, runs the grow-by-`grouping_offset` union-find fixed point (`grouping_offset = skirt_distance + skirt_loops * line_width`), returns the surviving groups' un-grown union bboxes sorted by `(x_min, y_min)`; both `process` and `run_finalization` iterate groups. `"combined"` returns exactly one group equal to today's `compute_bbox` result.
- Allowed reads: `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`, `crates/slicer-sdk/src/traits.rs` located window around `LayerCollectionView::ordered_entities`.
- Files allowed to edit (2): `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`.
- Out of bounds: as Step 4; also `crates/slicer-sdk/src/traits.rs` (read-only — no SDK change is needed or permitted here).
- Dispatches: `cargo test -p skirt-brim --test skirt_brim_tdd` → FACT pass/fail.
- Context cost: **M** (the union-find fixed point plus the two-case AC).
- Authorities: `requirements.md` §Per-Key Canonical Evidence `skirt_type` row; `design.md` §Selected Approach step 1, INV-4, INV-5.
- Verification: `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-5), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: group order varies between runs (INV-4), a skirt entity's own `RegionKey.object_id` stops being `"__skirt__"` (INV-5), or `"combined"` output differs from pre-packet.

---

## Step 7 — Build `min_skirt_length` loop expansion

- Objective: make a non-zero `min_skirt_length` add outward rings until the extruded-length target is met.
- Precondition: Steps 1, 2, 3 and 6 landed (the module must be able to read `filament_diameter` and `layer_height`, and must already iterate groups).
- Postcondition: `SkirtBrim` carries `min_skirt_length`, `layer_height`, `filament_diameter`; private `e_per_mm()` returns `((line_width - layer_height) * layer_height + π * (layer_height/2)²) / (π * (filament_diameter/2)²)`; private `ring_count(bbox)` returns `skirt_loops` when `min_skirt_length <= 0.0`, else the smallest ring count whose accumulated `perimeter_i * e_per_mm` reaches the target, capped by a module constant `MAX_MIN_LENGTH_LOOPS`; rings beyond `skirt_loops` are offset further **outward**.
- Allowed reads: `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`.
- Files allowed to edit (2): `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`.
- Out of bounds: as Step 4.
- Dispatches: `cargo test -p skirt-brim --test skirt_brim_tdd` → FACT pass/fail. If the accumulation or the outward direction is disputed, dispatch a SUMMARY read of `append_skirt_loops_for_hull` inside `Print::_make_skirt` (`OrcaSlicerDocumented/src/libslic3r/Print.cpp`).
- Context cost: **M**.
- Authorities: `requirements.md` §Per-Key Canonical Evidence `min_skirt_length` row and D-258-3; `design.md` INV-3, R-5.
- Verification: `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (AC-6, AC-N2), then `cargo xtask build-guests --check; echo "exit=$?"` → 0.
- Falsifying exit: `min_skirt_length = 1.0e9` does not terminate at the cap (INV-3), rings are added inward, or `min_skirt_length = 0.0` changes the ring count.

---

## Step 8 — Bounds/enum rejection and CONFIG_BLOCK reachability arms

- Objective: prove the manifest bounds are enforced before values reach `ConfigView`, and that an explicit `skirt_type` reaches the emitted CONFIG_BLOCK without any padding twin.
- Precondition: Steps 1–7 landed.
- Postcondition: `config_bounds_enforcement_tdd` carries the three rejection cases in AC-7; `gcode_header_thumbnail_config_blocks_tdd` carries the AC-9 presence/absence pair.
- Allowed reads: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`.
- Files allowed to edit (2): those two files.
- Out of bounds: `crates/slicer-gcode/src/serialize.rs` (Step 3 owns it), `ORCA_CONFIG_PADDING`.
- Dispatches: both test commands → FACT pass/fail.
- Context cost: **S**.
- Authorities: `requirements.md` §Acceptance Summary; `design.md` INV-6.
- Verification: `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`.
- Falsifying exit: an out-of-range value passes through to `ConfigView`, or an unset skirt key appears in the CONFIG_BLOCK.

---

## Step 9 — Register deviations, regenerate config docs, run the closure gates

- Objective: register the output-affecting divergences, bring the generated key reference in line, and prove the tree is green.
- Precondition: Steps 1–8 landed.
- Postcondition: `docs/DEVIATION_LOG.md` carries one row each for `requirements.md`'s D-258-3, D-258-4 and D-258-5, using `DEV-###` IDs **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next) — never an ID frozen at authoring; `docs/15_config_keys_reference.md` regenerated; `check-literals`, `clippy`, `build-guests --check` and the packet's narrow tests all pass.
- Allowed reads: none beyond command output.
- Files allowed to edit (2): `docs/DEVIATION_LOG.md` (three appended rows), `docs/15_config_keys_reference.md` — the latter **only** as the output of `cargo xtask gen-config-docs`, never by hand.
- Out of bounds: every source file (this is a gate step).
- Dispatches: each gate command → FACT exit code / pass-fail.
- Context cost: **S**.
- Authorities: CLAUDE.md §Build & Test Commands, §Test Discipline, §Guest WASM Staleness.
- Verification: `cargo xtask gen-config-docs --check` (AC-10) · `cargo xtask check-literals` · `cargo check --workspace --all-targets` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo xtask build-guests --check; echo "exit=$?"`.
- Falsifying exit: any gate non-zero, or `gen-config-docs --check` reports drift after the regeneration.

---

## Blast-radius discipline

- Step 2 adds a map entry, not a struct field, so there is no struct-literal blast radius in `slicer-ir`. Its blast radius is the two `resolved_config_to_map` delegators (`crates/slicer-gcode/src/serialize.rs`, `crates/slicer-wasm-host/src/dispatch.rs`); Step 3 owns the first and the second is unaffected because module visibility is filtered by declaration.
- Steps 4–7 add fields to `SkirtBrim`. Every test constructs it through `SkirtBrim::from_config`, so no `SkirtBrim { .. }` literal exists to update — the step that adds the fields confirms this with `cargo xtask check-literals` rather than assuming it. New test fixtures for watched types must carry a `..` rest or an `// exhaustive: <reason>` waiver.
- No public schema constant or version is bumped by this packet, so there is no test-assertion fallout of the "hard-asserts the old constant value" kind.
