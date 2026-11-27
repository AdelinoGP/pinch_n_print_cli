# Implementation Plan: support-support-keys

Steps are ordered. Steps 1–3 are the `slicer-core` / `slicer-runtime` slice; Steps 4–5 are the traditional planner; Steps 6–7 are the tree planner; Step 8 closes. Steps 4–5 and 6–7 are independent of each other and of Steps 1–3 after Step 1 lands.

Every `cargo` command below tees to `target/test-output.log`. When a run fails, **read the log**; never re-run to see more output (`CLAUDE.md` § Test output must always tee).

---

## Step 1 — Add the two `SupportContactParams` fields and absorb the literal blast radius

- **Task IDs:** none (queue packet, `task_ids: []`).
- **Objective:** `SupportContactParams` carries `critical_regions_only: bool` and `remove_small_overhang: bool` with canonical defaults, and the whole workspace still compiles. No behaviour changes yet.
- **Preconditions:** working tree clean; `cargo check --workspace --all-targets` green.
- **Postconditions:** both fields exist; `Default` yields `false` and `true` respectively; every struct literal compiles; `cargo xtask check-literals` green.
- **Allowed reads:** `crates/slicer-core/src/algos/overhang_annotation.rs` (the params declaration and its `Default` impl only), `docs/21_data_defaults_and_fixtures.md`.
- **Edits (3):** `crates/slicer-core/src/algos/overhang_annotation.rs`, `crates/slicer-core/tests/support_overhang_detection_tdd.rs`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`.
- **Out of bounds:** `crates/slicer-gcode/src/serialize.rs`; every planner module; `docs/config/host-keys.toml`.
- **Dispatch:** re-derive the literal sites before editing — `LOCATIONS` ≤ 20, scope `crates/**` and `modules/**`, Rust only, pattern `SupportContactParams {`. The authoring-time count was 2 / 11 / 3 across the three files above; it is a ledger fact, so re-derive it, do not trust it.
- **Blast-radius note:** production `src/` literals stay exhaustive. Test literals in `support_overhang_detection_tdd.rs` take a `..Default::default()` rest or an `// exhaustive: <reason>` waiver. The producer's per-layer literal already uses `..base_params.clone()` and needs no edit.
- **Cost:** M.
- **Authorities:** `docs/21_data_defaults_and_fixtures.md`; `requirements.md` § Per-Key Canonical Evidence for the two defaults.
- **Verification:** `cargo check --workspace --all-targets` and `cargo xtask check-literals`, both green in this step.
- **Falsifying exit:** if `cargo check --workspace --all-targets` reports a literal site outside the three files named above, stop — the blast radius was mis-derived and the step's edit list is wrong.

---

## Step 2 — Build the small-overhang filter and the critical-regions restriction

- **Task IDs:** none.
- **Objective:** `detect_support_contacts_with_annotations` filters small overhang clusters when `remove_small_overhang` is set, then restricts `contacts` to the cantilever and sharp-tail sets when `critical_regions_only` is set.
- **Preconditions:** Step 1 complete.
- **Postconditions:** AC-2 and AC-3 pass; AC-N4 still passes.
- **Allowed reads:** the body of `detect_support_contacts_with_annotations` and the cantilever-annotation helper in `crates/slicer-core/src/algos/overhang_annotation.rs`; `docs/08_coordinate_system.md`.
- **Edits (3):** `crates/slicer-core/src/algos/overhang_annotation.rs`, `crates/slicer-core/tests/support_critical_and_small_overhang_tdd.rs` (net-new), `crates/slicer-core/Cargo.toml` (the net-new `[[test]]` entry).
- **Out of bounds:** `crates/slicer-runtime/**`; every planner module.
- **Dispatches:** (a) `SUMMARY` ≤ 200 words on canonical `PrintObjectSupportMaterial::top_contact_layers` — the exact erosion distance, the bounding-box comparison, and the exemption predicate; (b) `SUMMARY` ≤ 200 words on canonical `TreeSupport::detect_overhangs` — the order of clear, re-append cantilevers, append sharp tails.
- **Cost:** M.
- **Authorities:** `requirements.md` § Canonical semantics the port borrows exactly; `design.md` DIV-3 and § Architecture Constraints (mm-not-scaled, square join).
- **Implementation notes, binding:** the erosion is `-external_perimeter_width_mm` in **millimetres** (`polygon_ops::offset` scales internally); the smallness test is `bbox.x < 2 * fw || bbox.y < 2 * fw` on the eroded cluster — an extent test, **not** an area test; the join is `SUPPORT_SURFACES_JOIN`. Ordering is filter, then restrict. Enforcers are not visible in this function and must stay that way.
- **Verification:** `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_critical_and_small_overhang_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** if the new test binary reports `0 passed; 0 failed`, the `[[test]]` entry is missing `required-features = ["host-algos"]` or the file is cfg-gated out — the run is blind, not green. Fix before proceeding.

---

## Step 3 — Source the three params from config in `resolve_contact_params`

- **Task IDs:** none.
- **Objective:** `enforce_support_layers`, `critical_regions_only` and `remove_small_overhang` come from `ResolvedConfig` (with the per-region `extension_int` override path for the first, matching its sibling keys) instead of hardcoded neutrals.
- **Preconditions:** Steps 1–2 complete.
- **Postconditions:** AC-1 and AC-9 pass; the stale "no production config source yet" comment is corrected to name the three keys that now have one and the three that still do not.
- **Allowed reads:** `resolve_contact_params` and the per-layer params literal in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`; the five field declarations in `crates/slicer-ir/src/resolved_config.rs` (read-only, ranged).
- **Edits (1):** `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (including its `tests` module).
- **Out of bounds:** `crates/slicer-ir/src/resolved_config.rs` — the typed fields already exist; editing it means the packet was mis-scoped, stop and report. Also `docs/config/host-keys.toml`.
- **Dispatch:** none.
- **Cost:** S.
- **Authorities:** `design.md` § Selected Approach; `requirements.md` § In Scope item 3.
- **Implementation note:** `bridge_no_support`, `bridge_polygons` and `support_sharp_tails` keep their current neutral sourcing. They are out of scope and are reported to the map as a separate gap.
- **Verification:** `mkdir -p target && cargo test -p slicer-runtime --lib support_analysis_producer 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** if wiring `enforce_support_layers` makes contacts appear on *every* layer rather than the lowest N, `layer_id` is not reaching the per-layer literal — stop and re-read that literal before changing the detector.

---

## Step 4 — Traditional planner: declare and read the two new keys

- **Task IDs:** none.
- **Objective:** `traditional-support-planner` declares `support_bottom_z_distance` and `support_object_first_layer_gap` in its manifest and holds both as struct fields read in `from_config`, alongside the existing `support_top_z_distance` / `support_object_xy_distance` reads.
- **Preconditions:** none beyond a clean tree (independent of Steps 1–3).
- **Postconditions:** both keys are declared with canonical type, default and bounds, and both are read; no key is declared without being read.
- **Allowed reads:** the struct declaration and `from_config` in `modules/core-modules/traditional-support-planner/src/lib.rs`; the existing `[config.schema.support_top_z_distance]` table in its manifest.
- **Edits (2):** `modules/core-modules/traditional-support-planner/src/lib.rs`, `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`.
- **Out of bounds:** `modules/core-modules/traditional-support/**`; `modules/core-modules/tree-support-planner/**`.
- **Dispatch:** none.
- **Cost:** S.
- **Authorities:** `requirements.md` § In Scope item 6 for the exact types, defaults and bounds; `design.md` DIV-1 for the `description` text.
- **Verification:** `cargo check --workspace --all-targets`
- **Falsifying exit:** the manifest gains any table other than the two named — in particular `raft_first_layer_expansion` or `support_style` (AC-N2). If it does, stop; that is the disposition rule 1 prohibits.

---

## Step 5 — Traditional planner: the bottom-Z gap and the first-layer XY override

- **Task IDs:** none.
- **Objective:** the emit floor is raised by `support_bottom_z_distance` on model-terminated columns only, and the per-layer trim offset uses `support_object_first_layer_gap` on layer 0.
- **Preconditions:** Step 4 complete.
- **Postconditions:** AC-4, AC-5 pass; AC-10 and AC-N4 still pass.
- **Allowed reads:** the `target_top_z` computation, the `model_termination_layer` resolution, and the per-layer trim loop in `modules/core-modules/traditional-support-planner/src/lib.rs` — three ranged windows, not a full-file read.
- **Edits (3):** `modules/core-modules/traditional-support-planner/src/lib.rs`, `modules/core-modules/traditional-support-planner/tests/support_gap_keys_tdd.rs` (net-new), `modules/core-modules/traditional-support-planner/Cargo.toml` (the net-new `[[test]]` entry).
- **Out of bounds:** `crates/slicer-core/**`; `modules/core-modules/tree-support-planner/**`.
- **Dispatch:** `SUMMARY` ≤ 200 words on canonical `TreeSupport::draw_circles` — confirm `gap_xy_first_layer` **substitutes for** `m_xy_distance` on object layer 0 rather than adding to it.
- **Cost:** M.
- **Authorities:** `design.md` DIV-1, DIV-2, § Architecture Constraints (do not divide by `effective_layer_height`), § Invariants.
- **Implementation note, binding:** the bottom gap is measured by walking actual layer Z upward from the termination layer until the accumulated gap reaches the configured value — the mirror of the existing `target_top_z` walk. A build-plate termination (`model_termination_layer` is `None`) gets no gap. The two clearance keys are never summed.
- **Verification:** `mkdir -p target && cargo test -p traditional-support-planner --test support_gap_keys_tdd 2>&1 | tee target/test-output.log && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee -a target/test-output.log; grep -c "test result: ok" target/test-output.log`
- **Falsifying exit:** a plate-terminated column's floor moves at any value of `support_bottom_z_distance` — DIV-2 is violated and the `Some`/`None` distinction was collapsed.

---

## Step 6 — Tree planner: declare, read, and apply the first-layer XY override

- **Task IDs:** none.
- **Objective:** `tree-support-planner` declares and reads both keys, and both `inflate_model_occupancy` call sites select `support_object_first_layer_gap` when the object layer index is 0.
- **Preconditions:** none beyond a clean tree.
- **Postconditions:** AC-6 passes.
- **Allowed reads:** `inflate_model_occupancy` and its two call sites, and the `from_config` reads of `support_top_z_distance` / `support_object_xy_distance`, in `modules/core-modules/tree-support-planner/src/lib.rs` — **ranged reads only**, the file is long.
- **Edits (3):** `modules/core-modules/tree-support-planner/src/lib.rs`, `modules/core-modules/tree-support-planner/tree-support-planner.toml`, `modules/core-modules/tree-support-planner/tests/support_gap_keys_tdd.rs` (net-new).
- **Out of bounds:** `modules/core-modules/traditional-support-planner/**`; `crates/slicer-core/**`. Note the manifest already declares `support_style` — leave that table exactly as it is (AC-N2).
- **Dispatch:** `LOCATIONS` ≤ 20 — every `inflate_model_occupancy` call site and the object-layer-index expression in scope at each, so the selection lands at both without a full-file read.
- **Cost:** M.
- **Authorities:** `design.md` § Selected Approach; `requirements.md` § Canonical semantics the port borrows exactly.
- **Verification:** `mkdir -p target && cargo test -p tree-support-planner --test support_gap_keys_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** only one of the two call sites is changed. Both are in the collision path; changing one produces a planner that disagrees with itself.

---

## Step 7 — Tree planner: the bottom-Z gap, or a named deferral

- **Task IDs:** none.
- **Objective:** resolve `design.md`'s `[FWD]` question and either apply the bottom gap at the tree planner's descent-termination site or defer it with a written reason.
- **Preconditions:** Step 6 complete.
- **Postconditions:** either the tree planner honours `support_bottom_z_distance` on model-terminated columns, or `design.md` § Open Questions records the concrete reason it cannot (the tree family reaching the plate through node propagation with no per-column model-termination signal) and `requirements.md`'s scope note is amended to say the key is carried by the traditional planner only.
- **Allowed reads:** the descent-termination and node-drop region of `modules/core-modules/tree-support-planner/src/lib.rs` — ranged, anchored on the `to_buildplate` / termination logic.
- **Edits (2):** `modules/core-modules/tree-support-planner/src/lib.rs`, `modules/core-modules/tree-support-planner/Cargo.toml` (the net-new `[[test]]` entry, if not added in Step 6).
- **Out of bounds:** everything outside `modules/core-modules/tree-support-planner/`.
- **Dispatch:** `LOCATIONS` ≤ 20 — where a tree column's descent terminates and whether that site distinguishes a model surface from the build plate.
- **Cost:** M.
- **Authorities:** `design.md` § Open Questions, DIV-2.
- **Falsifying exit:** if no per-column model-termination signal exists, **do not synthesise one** — that would be a new decision point outside this packet's scope. Record the deferral and move to Step 8. A faked signal is worse than a named gap.

---

## Step 8 — Bounds arm, gates, and docs regeneration

- **Task IDs:** none.
- **Objective:** the two new manifest keys are bounds-enforced, all gates are green, and the generated key reference reflects five newly live keys.
- **Preconditions:** Steps 1–7 complete or explicitly deferred per Step 7.
- **Postconditions:** AC-N1, AC-N2, AC-N3 pass; the three gate commands are green; `docs/15_config_keys_reference.md` is regenerated.
- **Allowed reads:** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (the existing arms, for shape).
- **Edits (2):** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `docs/15_config_keys_reference.md` (regenerated, not hand-edited).
- **Out of bounds:** `crates/slicer-gcode/src/serialize.rs`; `docs/specs/orca-feature-gap/**`; `docs/specs/support-parity-gap-register.md`.
- **Dispatch:** `FACT` pass/fail for each gate command.
- **Cost:** S.
- **Authorities:** `packet.spec.md` § Gate Commands; `requirements.md` § Verification Matrix.
- **Verification:** `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check` (inspect the exit code — 0 fresh, 1 stale, 3 `wasm-tools` missing; never grep for `STALE:`); then `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** `git diff --stat -- crates/slicer-gcode/src/serialize.rs` is non-empty (AC-N3), or either returned key appears in a planner manifest (AC-N2).

---

## Aggregate Context Cost

S + M + M + S + M + M + M + S = **L aggregate, no single step above M.** The packet does not require a split: the two planner slices (Steps 4–5, 6–7) are independent of the host slice (Steps 1–3) and of each other, so a budget-constrained run may land them in separate sessions.

## Closing Obligations

- Report the five items in `design.md` § Map and Ticket Updates Required. Do **not** apply them; the map, the tickets and the parity register are out of bounds.
- Re-derive every ledger fact at point of use: the `SupportContactParams` literal count, the next free packet number for the organic-tree-engine row, and the `docs/07` module inventory.
- Run `/spec-review 265-support-support-keys --preflight` before requesting activation, and re-check the two map gates by hand: (a) zero declaration-only keys in the disposition table; (b) a non-default-value AC for every in-packet key.
