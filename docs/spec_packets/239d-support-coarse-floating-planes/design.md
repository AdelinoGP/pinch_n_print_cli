# Design: 239d-support-coarse-floating-planes

## Controlling Code Paths

- **Primary code path (Z authority).** `SupportPlanner::plan_for_object`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) and its traditional twin
  (`modules/core-modules/traditional-support-planner/src/lib.rs`). The 239c derivation lives
  in `packet239c_intermediate_planes` (tree `lib.rs` ~3757-3783, traditional `lib.rs`
  ~684-710) and its callers (tree ~3649-3705, traditional ~597-650). The tree caller builds
  `support_rows_by_object` (object_id → `anchor_layer_index` → surviving entries, filter
  `decline_reason.is_none() && skeleton.is_some()`, tree `lib.rs` ~3622-3633) and brackets
  consecutive layer keys; the traditional caller tracks `previous_supported_layer`
  (traditional `lib.rs` ~597-650). `support_pitch_mm` comes from the `support_layer_height_mm`
  read (tree `lib.rs` ~1641-1645, traditional `lib.rs` ~108-112) with the 0.0 sentinel
  falling back to the object's effective layer height (tree `lib.rs` ~3597-3616, traditional
  `lib.rs` ~599-601; the `[FWD]` option (b) comments at tree `lib.rs` ~3600-3608 and
  traditional `lib.rs` ~602-606).
- **Primary code path (decimation).** `build_emit_schedule`
  (`crates/slicer-core/src/algos/support_geometry.rs` ~51-84) decimates the host-side
  `SupportGeometryIR` by `support_layer_height_mm`; it is consumed only inside
  `execute_support_geometry` (same file ~92-127), which the host prepass
  (`commit_support_geometry_builtin`, `support_geometry_producer.rs` ~37-52) calls — there is
  no direct `build_emit_schedule` call in the producer file. Both planners receive
  `SupportGeometryView` as `_support_geometry` (tree `plan_for_object`, `lib.rs` ~1851;
  traditional `run_support_geometry_with_analysis`, `lib.rs` ~174) and ignore it on the
  meshed-object planner path: the tree planner's only read is the mesh-less legacy contact
  fallback — a genuinely mesh-less object with no contacts (gated at tree `lib.rs`
  ~2169-2172, read at ~2173) — and the traditional planner never reads it. The traditional
  planner's own decimation is `support_step =
  round(support_layer_height_mm / model_layer_height).max(1)` (traditional `lib.rs`
  ~363-366) applied at the entry-emission gate (traditional `lib.rs` ~511).
- **Primary code path (emission).** Unchanged from 239c: the tree renderer traverses
  `paint.support_plan()` and the traditional renderer consumes
  `support_plan_entries_for` (`PaintRegionLayerView`) — each obeys the plan-declared
  `anchor_z` per entry, so off-grid rows travel the anchored path (DEV-159..163 seam
  completion) on both families. No transport changes are expected.
- **Neighbouring tests/fixtures.**
  (`crates/slicer-runtime/tests/integration/support_family_closure.rs` (real-slice driver
   `run_slice_for_family` / `run_slice_for_family_with_extra` → `slicer_runtime::run::run_slice`,
   tracked fixture `SupportTest.stl`, tracked config `orca-matched-config.json`; helpers
   `distinct_z_sequence` ~225, `z_followed_by_support_block` ~238, the 239c baseline const
   `DISABLED_INDEPENDENT_HEIGHT_BASELINE_Z` ~261, `assert_no_test_reads_orca_gcode` ~1179;
   the 239c tests at ~284, ~317, ~368, ~1314 — **note**: this packet's AC-N1 const is the
   new `P239D_DISABLED_COARSE_PITCH_BASELINE_Z`, not the 239c const above;
  `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` (the `layer_plan()`
  fixture helper ~167, the 239c test `enabled_independent_height_produces_free_floating_anchor_z`
  ~927);
  `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` (the
  239c test `disabled_independent_height_copies_object_layer_print_z_exactly` ~333);
  `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (the 239c verdict test
  `offgrid_pass_height_delta_matches_recorded_verdict` ~1883);
  `crates/slicer-gcode/tests/gcode_relative_extrusion_tdd.rs` (`extract_e_values` ~50 — the
  E-parsing precedent, not importable across crates).
- **OrcaSlicer comparison:** see `requirements.md` §OrcaSlicer Reference Obligations; do not
  repeat delegation rules.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` = 10 units = 1e-3 mm** is the
  single on-grid/off-grid discriminator used by both the planner (deciding whether a derived
  plane is off-grid) and the renderer (deciding whether to take the anchored route). Do not
  introduce a second epsilon.
- **Config keys are snake_case in Rust, always.** `config.get("support_layer_height_mm")`,
  never `"support-layer-height-mm"`. Manifest section headers are already snake_case.
- **No schema/version constant moves.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`
  (`crates/slicer-ir/src/slice_ir.rs`) is not bumped; no `SupportPlanIR` version moves; no
  field is added to `SupportPlanEntry` — the coarse stack is expressed entirely through
  `anchor_z` values. No version literal is frozen here on purpose: re-derive it from the
  constant at the moment you need it.
- **239c's locked invariants carry over.** `anchor_z` is the declared support print plane and
  the only Z authority a support renderer may consult; the disabled branch is bit-for-bit
  the pre-change behaviour; the on-grid/off-grid discriminator is
  `COORDINATE_TOLERANCE_UNITS`; the `support_layer_height_mm == 0.0` sentinel means "object
  pitch" (239c [FWD] option b). This packet extends the enabled branch only.

## Code Change Surface

**Selected approach — the coarse stack brackets the demanded interface/contact planes.**

The 239c derivation brackets consecutive support rows (dense, object-grid). The coarse
direction must bracket the **demanded planes** — the layers whose entries carry
`TopInterface`/`BaseInterface`/`BottomInterface` roles, which sit at support-region
boundaries and span many object layers (mirroring canonical's sorted `extremes`). Between
consecutive demanded planes the stack is generated at pitch spacing by the canonical rule,
and only the **non-interface rows strictly inside each bracket pair** are replaced by the
stack planes — genuine interface bracket entries always remain. This is a
planner-side derivation change only; the renderer/row path is untouched.

Exact functions, tests, and fixtures:

**Binding decisions (formerly `[FWD]` Q1-Q3; recorded 2026-09-01).** These are part of the
design contract now, not implementer choices:

- **Q1 — bracket selection (binding).** The bracket set is computed per
  `(object_id, region_id)` over each **contiguous run** of demanded support-bearing rows
  (consecutive layers with surviving entries for that object+region). Let I be the run's
  interface-role planes (`TopInterface`/`BaseInterface`/`BottomInterface`). The bracket set
  is:
  - **count(I) >= 2:** the sorted/deduplicated I itself — the run's endpoints are **not**
    added.
  - **count(I) < 2 (zero or one interface plane):** I supplemented with the run's first and
    last surviving support-bearing rows, then sorted/deduplicated by `anchor_z`. A lone
    genuine interface plane therefore stays a bracket, and is never demoted to body or
    removed.
  Within a run the consecutive bracket pairs are ordered by `anchor_z`.
- **Q2 — clone source for synthesized stack planes (binding).** Each synthesized stack
  plane **clones the lower bracket's geometry** (its skeleton and other entry fields) and
  **rewrites the roles to `SupportBody`**; genuine interface bracket entries are **not**
  cloned-over — they survive with their interface roles intact. The source
  `global_layer_index` is captured only
  as part of the local duplicate key and clone-source provenance decision (the
  duplicate-key rule below); the **emitted** entry's final `global_layer_index` is assigned
  from the existing per-plane DEV-163 synthetic identity map (`BTreeMap<i64, i32>`), so all
  entries at one synthesized plane share that plane identity. Other provenance fields are
  preserved. No interface role
  or interface geometry may appear at a body plane.
- **Q3 — `support_step` neutralization form (binding).** The neutralization **sets
  `support_step = 1`** exactly for bracket pairs satisfying the binding coarse predicate —
  configured nonzero pitch >= `local_support_gap`, the maximum positive anchor-Z
  difference between consecutive surviving support-bearing rows of that same
  `(object_id, region_id)` contiguous run covered by the bracket — i.e. the condition is
  evaluated per bracket pair by the same `local_support_gap` predicate as the
  coarse/finer selection, not globally. The finer direction (where `support_step` is
  already 1) is bit-identical, and no global bypass of the decimation gate is added.

1. **Tree planner** (`modules/core-modules/tree-support-planner/src/lib.rs`): in the 239c
   caller (~3649-3705), for each consecutive demanded bracket pair where the binding coarse
   predicate holds — configured nonzero pitch >= `local_support_gap`, the maximum positive
   anchor-Z difference between consecutive surviving support-bearing rows of that same
   `(object_id, region_id)` contiguous run covered by the bracket (these rows are already
   available to both planner callers), compared in exact canonical units with
   `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only tolerance if one is
   needed (no new epsilon) — bracket the demanded planes instead of consecutive support
   rows, using the Q1 bracket rule above. Interface brackets select only genuinely coarse
   replacement ranges; every consecutive surviving-row pair not covered by such a range,
   including pairs below or above the outer interface brackets, keeps the existing 239c
   finer derivation. Between consecutive bracket planes of a
   `(object_id, region_id)` contiguous run, generate the stack by the **tree-family**
   canonical rule of `plan_layer_heights` (`TreeSupport.cpp`): `n = ceil(dist / pitch)`
   (main-body spacing; **no** EPSILON bias), `step = dist / n`, planes at
   `below_z + k * step` for `k = 1..n` with the last aligned to `above_z` (the 239c helper
   returns `1..n` strictly between; the coarse direction additionally emits the aligned last
   plane, which dedups against the upper bracket). Apply the canonical grouping rule of
   `generate_support_layers` (`SupportCommon.cpp`): **group candidate print-Z values within
   `EPSILON` of each other and replace each group by its midpoint**
   `zavg = 0.5 * (first + last)`. Canonical's group **minimum-height** rule is explicitly
   **not** reproduced: `SupportPlanEntry` has no height field, and a row's effective height
   derives from the `anchor_z` of its adjacent rows, so a per-entry "group height" has no
   representation here — this is a recorded deviation-by-inapplicability, not an omission.
   The entries between the brackets are replaced by the stack planes, **except** genuine
   interface bracket entries, which always remain (only **non-interface rows strictly
   inside** each bracket pair are removed). The stack planes are each cloned from
   the lower bracket with roles rewritten to `SupportBody`: the source `global_layer_index`
   is captured into the local duplicate key and clone-source provenance decision only, the
   emitted entry's final `global_layer_index` comes from the per-plane DEV-163 synthetic
   identity map (`BTreeMap<i64, i32>`) so all entries at one synthesized plane share that
   plane identity, and other provenance fields are preserved; the insertion-time identity
   key is the duplicate-key rule's stable
   `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)`), per
   Q2. The sentinel
   (`support_layer_height_mm == 0.0` → object pitch) is separate and takes priority: it
   bypasses the coarse path entirely and preserves the existing 239c object-grid behavior —
   no coarse stack is synthesized, and no resolved pitch is ever claimed to equal the
   varying `local_support_gap` values (AC-N3 pins this).
2. **Traditional planner** (`modules/core-modules/traditional-support-planner/src/lib.rs`):
   the same derivation at the 239c caller (~597-650) using the **traditional-family**
   canonical rule of `raft_and_intermediate_support_layers`
   (`Support/SupportMaterial.cpp`): `n = ceil((dist - EPSILON) / pitch)`,
   `step = dist / n`, planes at `below_z + k * step`, last aligned to `above_z` — **with**
   the EPSILON bias, unlike the tree rule. Apply the same `generate_support_layers`
   grouping/midpoint rule (group within `EPSILON`, midpoint; no group-height representation
   — same inapplicability as above). Plus the `support_step` neutralization per Q3: for each
   bracket pair satisfying the binding coarse predicate (configured nonzero pitch >=
   `local_support_gap`), `support_step` is set to 1 (the floating stack replaces
   the every-Nth-layer grid subset; the decimation gate at ~511 then passes every layer and
   the stack provides the coarse rows). `support_step` stays as-is for the finer direction
   (where it is already 1).
   **Duplicate-key prevention (stable key, no false collisions).** Before inserting a
   synthesized or surviving entry, its identity key
   `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)` is
   deduplicated — the key is the cloned lower bracket's source-entry identity (its
   `global_layer_index`, owning `object_id`/`region_id`, and ordered `body_ids: Vec<String>`)
   plus the plane's `anchor_z`; a second entry with the same key is dropped, so the
   per-object entry map never carries two rows with one identity. There is no entry `id`
   field on `SupportPlanEntry` to key on, and the key deliberately spans the full source
   identity (global layer origin + object + region + full body membership) so it cannot
   collapse two legitimately distinct geometries. **This dedup applies to synthesized
   candidates only: surviving real entries are never deduplicated by emitted
   `global_layer_index`** — two genuine distinct entries may legitimately share an emitted
   plane identity (per-plane DEV-163 assignment), and removing either would destroy real
   geometry. The captured source `global_layer_index`
   lives only in this local key and the clone-source provenance decision; the emitted
   entry's final `global_layer_index` is assigned from the per-plane DEV-163 synthetic
   identity map (`BTreeMap<i64, i32>`), so all entries at one synthesized plane share that
   plane identity.
3. **Tests.** `tree_family_tdd.rs`: `coarse_pitch_produces_free_floating_anchor_z` (AC-2)
   and `zero_pitch_sentinel_stays_object_grid` (AC-N3), extending the `layer_plan()` fixture
   pattern (~167) with a multi-layer demand and pitch 0.3, asserting the exact expected
   bracket planes per the tree `plan_layer_heights` (`TreeSupport.cpp`) formula, the
   original output order, the nondecreasing-per-object / strictly-increasing-distinct
   planes rule, `SupportBody` role replacement, and true-nearest `anchor_layer_index`
   anchoring with the lower-index tie break. **AC-2 also asserts the one-interface case if
   the fixture naturally expresses it: a run with exactly one genuine interface plane keeps
   that plane as a bracket (not demoted to body), per the Q1 sort/dedup refinement.** Plus
   a **finer-direction adaptive regression**
   `adaptive_local_gap_stays_finer` (**NET-NEW test authored by this packet**, present in
   the existing `tree_family_tdd` target and passing): a run whose bracket-pair
   `local_support_gap` (the maximum
   positive anchor-Z difference between consecutive surviving support-bearing rows of the
   same `(object_id, region_id)` contiguous run covered by the bracket) exceeds
   the configured pitch keeps
   the 239c derivation (coarse/finer selection is bracket-local, never decided from the
   first/contact layer height alone). `traditional_family_tdd.rs`:
   `coarse_pitch_produces_free_floating_anchor_z` (AC-3) with the same assertion strength
   under the traditional `raft_and_intermediate_support_layers`
   (`Support/SupportMaterial.cpp`) formula. `support_family_closure.rs`:
   `coarse_support_pitch_emits_free_floating_extruding_rows` (AC-1) and
   `disabled_coarse_pitch_reproduces_baseline_z_sequence` (AC-N1) as `pub fn` checks plus
   bare `#[test]` wrappers in `integration/main.rs` (the wrapper convention), and a new
   E-assertion helper (parse the `;TYPE:Support` block after an off-grid `;Z:` row, extract
   G1 `E` tokens, assert at least one `> 0`; the `extract_e_values` precedent is in a
   different crate's test binary and cannot be imported). `gcode_emit_tdd.rs`:
   `coarse_pass_height_delta_matches_recorded_verdict` (AC-4), mirroring the 239c verdict
   test (~1883) with the Step 5 measured constants (applied-height term, declared plane
   delta, resulting E).
4. **Docs.** `docs/07_implementation_status.md` (Step 1 and Step 5 measurement records under
   `TASK-523`/`TASK-527`; `TASK-523`..`TASK-530` registration at Step 8),
   `docs/specs/support-independent-layer-z-split-plan.md` (queue row 4),
   `docs/specs/support-parity-gap-register.md` (new coarse-direction row),
   `tmp/239d-human-validation.md` (the gate document).

**Rejected alternatives.**

- *Coarse rows from the decimated subset (`build_emit_schedule` / `support_step`).* Rejected:
  the host schedule never reaches the meshed-object planner path (both planners ignore
  `SupportGeometryView` there — the tree's only read is the mesh-less legacy contact
  fallback in `SupportPlanner::plan_for_object` (tree `lib.rs`); measured: family-labeled
  normal(auto) support on 85/299 rows, tree(auto) exploratory run 248 rows, despite it), and
  `support_step` decimates on-grid — neither can produce free-floating rows. The floating
  stack is the source of coarse rows; the decimation mechanisms are reconciled as above.
- *Exact-pitch spacing from the region bottom (`below_z + k * pitch`).* Rejected: the last
  row can land arbitrarily close to the upper bracket and the interface layers; canonical
  `dist/n` adapts the step to the span and aligns the last row. User decision 2026-08-31:
  canonical `dist/n`.
- *Bracket the run boundaries only (first/last demanded layers of each contiguous run),
  regardless of interface entries.* Rejected: ignores the interface structure canonical
  uses; the interface entries are the closer analog of canonical's `extremes`. **Binding
  refinement (Q1):** with count(interface planes) >= 2 the bracket set is the
  sorted/deduplicated interface planes themselves (endpoints not added); with fewer than
  two, that set is supplemented with the run's first/last surviving support-bearing rows and
  sorted/deduped — so a lone genuine interface plane stays a bracket. User decision
  2026-08-31 (interface entries as skeleton); refined
  2026-09-01 (two-row threshold + per-`(object_id, region_id)` run partition); refined
  again (count-conditional supplement so lone interface planes survive as brackets).
- *Remove `support_step` entirely.* Rejected: the neutralization (set to 1 exactly for
  bracket pairs where pitch >= `local_support_gap`) is sufficient and avoids the blast
  radius of deleting a mechanism the traditional
  planner's tests pin. User decision 2026-08-31: floating stack replaces decimation.
- *Transport a `support_layer_height` field on `SupportPlanEntry`.* Rejected (as in 239c):
  it is a WIT-crossing prepass type; the stack is expressed through `anchor_z` deltas.
  Corollary: canonical's `generate_support_layers` group **minimum-height** rule has no
  representation here — `SupportPlanEntry` has no height field and a row's effective height
  derives from adjacent `anchor_z` values — so PnP reproduces the grouping predicate
  (`print_z <= first.print_z + EPSILON`) and the midpoint rule only.

## Files in Scope (read + edit)

Two primary files, justified by the inherent symmetry across the two support families; no
step edits more than three files.

- `modules/core-modules/tree-support-planner/src/lib.rs` - role: tree Z authority; expected
  change: the coarse-direction bracket selection + stack generation in the 239c caller.
  **Very large file — ranged reads only.**
- `modules/core-modules/traditional-support-planner/src/lib.rs` - role: traditional Z
  authority; expected change: the same derivation + the `support_step` neutralization.
  **Very large file — ranged reads only.**

Test and doc files edited by their owning steps: `tree_family_tdd.rs`,
`traditional_family_tdd.rs`, `support_family_closure.rs`, `integration/main.rs`,
`gcode_emit_tdd.rs`, `docs/07_implementation_status.md`,
`docs/specs/support-independent-layer-z-split-plan.md`,
`docs/specs/support-parity-gap-register.md`, `tmp/239d-human-validation.md`.

## Read-Only Context

Include ranges for files over 300 lines.

- `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` - whole file -
  purpose: the ACs this packet extends, the test-naming convention, the gate structure.
- `docs/spec_packets/239c-support-layer-height-producer/design.md` - whole file - purpose:
  the derivation rules, the `[FWD]` sentinel decision, the locked invariants.
- `docs/specs/support-independent-layer-z-split-plan.md` - whole file (short) - purpose: the
  canonical block and the packet queue.
- `docs/DEVIATION_LOG.md` - rows `DEV-159`..`DEV-163` only - purpose: the seam completion
  239d inherits (the E=0 defect class, the aggregation/view/commit seams, the identity
  resolution).
- `docs/specs/support-parity-gap-register.md` - rows `G-02` and the new coarse row only -
  purpose: the gap 239c closed and the gap this packet closes.
- `crates/slicer-core/src/algos/support_geometry.rs` - `build_emit_schedule` and
  `execute_support_geometry` only (~51-127) - purpose: the read-only decimation surface.
- `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` - the prepass call site
  only (~47-52) - purpose: confirming the schedule never reaches the meshed-object planner
  path (the prepass calls `execute_support_geometry`, never `build_emit_schedule` directly).
- `crates/slicer-ir/src/slice_ir.rs` - the `SupportPlanEntry` definition only - purpose:
  `anchor_z`/`anchor_layer_index` semantics (no new fields).
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - the helper block
  (`support_test_path`, `matched_config_path`, `matched_config_for`, `run_slice_for_family`,
  `run_slice_for_family_with_extra`, `distinct_z_sequence`, `z_followed_by_support_block`,
  `assert_no_test_reads_orca_gcode`, the 239c `DISABLED_INDEPENDENT_HEIGHT_BASELINE_Z`, and
  this packet's new `P239D_DISABLED_COARSE_PITCH_BASELINE_Z`) and the 239c
  tests only - purpose: the real-slice driver AC-1/AC-N1 reuse and the wrapper convention.
- `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` - the `layer_plan()`
  fixture helper and the 239c independent-height test only - purpose: the AC-2/AC-N3 fixture
  pattern.
- `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - the 239c verdict test only (~1883-1960) -
  purpose: the AC-4 pattern.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-core/src/algos/support_geometry.rs` and
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` - read-only pre-239c
  surface; never edited here.
- The renderers (`modules/core-modules/tree-support/src/lib.rs`,
  `modules/core-modules/traditional-support/src/lib.rs`), the host seams
  (`crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/pipeline.rs`),
  the WIT/transport (`crates/slicer-schema/wit/**`, `crates/slicer-wasm-host/src/**`,
  `crates/slicer-sdk/src/layer_collection_builder.rs`) - 239a/239b/239c-owned; never edited
  here. If a renderer edit is genuinely needed, that is a scope change to raise, not a local
  fallback.
- `docs/spec_packets/239c-support-layer-height-producer/implementation-plan.md` and
  `requirements.md` - consume the `packet.spec.md` and `design.md` contracts only.
- `docs/15_config_keys_reference.md` - generated by `cargo xtask gen-config-docs`; not
  regenerated by this packet (no new keys).
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: confirm the non-synchronized branch of `raft_and_intermediate_support_layers`
  brackets the sorted `extremes` (top/bottom contact layers) and fills between consecutive
  ones at `n_layers_extra = ceil((dist - EPSILON) / max_suport_layer_height)`,
  `step = dist / n_layers_extra`, `print_z = extr1z + i * step`, last aligned to `extr2z`,
  and that the synchronized branch snaps to object layers; and confirm the tree-family
  `plan_layer_heights` (`TreeSupport.cpp`) main-body loop uses `ceil(dist / pitch)` with no
  EPSILON bias; scope:
  `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` and
  `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY`
  (<= 200 words); purpose: Steps 2-3.
- Question: every site in `modules/core-modules/tree-support-planner/src/lib.rs` and
  `modules/core-modules/traditional-support-planner/src/lib.rs` that assigns `anchor_z`,
  reads `support_layer_height_mm`, or applies `support_step`; scope: those two files; return:
  `LOCATIONS` (<= 20 entries); purpose: Steps 2-3, so neither large file is full-read.
- Question: the interface-role entry structure in the tree planner's emit pass — which
  layers carry `TopInterface`/`BaseInterface`/`BottomInterface` roles and how the roles are
  decided per node (`support_interface_top_layers`, `support_interface_bottom_layers`,
  `num_top_base_interface_layers`); scope: `modules/core-modules/tree-support-planner/src/lib.rs`
  (~3060-3480); return: `SUMMARY` (<= 200 words); purpose: Step 2 bracket selection.
- Question: the three measurement numbers for a minimal coarse off-grid case through
  `DefaultGCodeEmitter::emit_gcode` — applied height term, declared plane delta, resulting E
  — for a 0.3-pitch row; scope: `crates/slicer-gcode/`; return: `FACT` (three numbers plus
  the verdict word); purpose: Step 5. **Highest-risk dispatch.**
- Question: every test in the workspace that hard-asserts the traditional planner's
  `support_step` decimation behaviour (row counts or layer multiples); scope: `crates/`,
  `modules/`; return: `LOCATIONS` (file + test fn, <= 15 entries); purpose: Step 3 blast
  radius.
- Question: the exact family names and config-edit helper used by the 239c real-slice tests
  (`run_slice_for_family_with_extra` call sites) so AC-1 can cover both families; scope:
  `crates/slicer-runtime/tests/integration/support_family_closure.rs`; return: `FACT`
  (<= 5 lines); purpose: Step 4.

## Data and Contract Notes

- **IR/manifest contracts.** No IR shape changes; no manifest changes. `SupportPlanEntry`
  keeps its live field set (body membership carried by `body_ids: Vec<String>`; no entry
  `id` field); the coarse stack is expressed through `anchor_z` values and the
  existing `anchor_layer_index` (the nearest object layer). The keys
  (`independent_support_layer_height`, `support_layer_height_mm`) are already declared on
  both planner manifests with byte-identical `type`/`default` (the
  `ConfigBoundsIndex::from_modules` intersection requires it).
- **WIT boundary.** None crossed. The anchored transport is 239b's; the host seam is 239a's;
  the renderer emission is 239c's. Editing any of them is out of bounds.
- **Determinism/scheduler constraints.** The coarse derivation must be a pure function of
  the entries plus config, never of iteration order or hash-map traversal. Per object, the
  emitted entry sequence must be **nondecreasing in `anchor_z`** in the planner's original
  output order, and the **distinct** `anchor_z` planes must be **strictly increasing** —
  equal-`anchor_z` entries within an object (different `region_id` or different source
  entry, distinguishable by the identity key's `ordered body_ids` component) are the only
  repetition allowed, and duplicate identity keys
  `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)` are
  prevented at insertion (see the
  duplicate-key rule above). A strict per-entry increase is **not** required or asserted.
  The synthetic `global_layer_index` scheme (tree `i32::MIN + ordinal`, deduped per plane via
  `intermediate_plane_indices`) is inherited from 239c and must not introduce a second
  ordering authority.
- **Decimation facts (measured, not assumed).** `build_emit_schedule` gates only the
  host-side `SupportGeometryIR`; both planners ignore `SupportGeometryView` on the
  meshed-object planner path (the tree's only read is the mesh-less legacy contact fallback,
  tree `lib.rs` ~2169-2173; the traditional parameter, `lib.rs` ~174, is never read). The
  traditional `support_step` decimation is on-grid. Neither can produce free-floating rows;
  the floating stack is the coarse-row mechanism.

## Locked Assumptions and Invariants

- **Locked:** `anchor_z` is the declared support print plane, in canonical units, and is the
  only Z authority a support renderer may consult (239c).
- **Locked:** the disabled branch is bit-for-bit the pre-change behaviour. AC-N1 is the
  falsifier; it compares against a baseline captured **before** any planner edit (Step 1).
- **Locked:** the `support_layer_height_mm == 0.0` sentinel means "object pitch" (239c [FWD]
  option b). AC-N3 is the falsifier.
- **Locked:** the finer direction (configured nonzero pitch < `local_support_gap`) is
  unchanged. AC-N2 (the 239c AC-1 test) is
  the falsifier.
- **Locked (coarse/finer selection is bracket-local, with one binding predicate).** For
  each consecutive demanded bracket pair, `local_support_gap` is the maximum positive
  anchor-Z difference between consecutive surviving support-bearing rows of that same
  `(object_id, region_id)` contiguous run covered by the bracket (rows already available
  to both planner callers). The bracket takes the coarse path iff the configured nonzero
  pitch >= `local_support_gap`, compared in exact canonical units with
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only
  tolerance if one is needed (no new epsilon); otherwise the bracket retains the 239c
  finer derivation. The decision is made **per bracket pair**. It is never
  decided from the first or contact layer height alone: two runs of the same object can
  resolve differently (one coarse, one finer) when their covered surviving-row gaps differ,
  and a bracket pair whose `local_support_gap` exceeds the configured pitch (e.g. pitch 0.2
  over covered surviving-row gaps of 0.3, even when the object's first/base layer gap is
  0.2) keeps the
  239c
  finer behaviour even when the global pitch >= the object's base layer pitch.
- **Locked:** the coarse stack follows the canonical `dist/n` stepping between consecutively
  demanded interface/contact planes, with the canonical grouping/midpoint rule applied
  (user decision 2026-08-31).
- **Locked:** the on-grid/off-grid discriminator is
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm), in both
  planner and renderer (239c).
- **Locked (Q1-Q3, binding):** bracket selection is per `(object_id, region_id)` contiguous
  run: with count(interface-role planes) >= 2 the bracket set is the sorted/deduplicated
  interface planes (endpoints not added); with fewer than two, that set is supplemented
  with the run's first/last surviving support-bearing rows and sorted/deduped by `anchor_z`
  (a lone
  genuine interface plane remains a bracket); stack planes clone the lower bracket with roles rewritten to `SupportBody`,
  capturing the source `global_layer_index` into the local duplicate key and
  clone-source provenance decision only, assigning the emitted entry's final
  `global_layer_index` from the per-plane DEV-163 synthetic identity map
  (`BTreeMap<i64, i32>` — one plane identity shared by all entries at a synthesized plane),
  and preserving other provenance fields;
  `support_step` neutralization sets 1 per coarse bracket
  pair only. See §Code Change Surface.
- **Locked (family-specific stepping):** the traditional stack uses
  `raft_and_intermediate_support_layers` (`Support/SupportMaterial.cpp`)
  `ceil((dist - EPSILON) / pitch)` with the EPSILON bias; the tree stack uses
  `plan_layer_heights` (`TreeSupport.cpp`) `ceil(dist / pitch)` without it. One shared
  formula for both families is a spec defect, not a simplification.
- **Locked (binding coarse/finer predicate):** for each consecutive demanded bracket
  pair, `local_support_gap` = the maximum positive anchor-Z difference between consecutive
  surviving support-bearing rows of that same `(object_id, region_id)` contiguous run
  covered by the bracket; coarse path iff configured nonzero pitch >= `local_support_gap`
  (exact canonical units, `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the
  only tolerance if needed — no new epsilon); otherwise the 239c finer derivation is
  retained for that bracket. AC-N4 pins the concrete case: configured pitch 0.2 over
  covered surviving-row gaps 0.3 stays finer even when the object's first/base layer gap is
  0.2.
- **Locked (ordering):** entries are nondecreasing in `anchor_z` per object in original
  output order; distinct planes strictly increasing; identity key
  `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)` unique.
- **Locked (anchoring):** each synthesized plane's `anchor_layer_index` is the true-nearest
  object layer by absolute Z distance to the plane's `anchor_z`, with the lower
  `anchor_layer_index` winning a tie deterministically.
- **Locked by measurement, not by assumption:** whether `DefaultGCodeEmitter::emit_gcode`
  mis-scales a coarse 0.3-pitch off-grid pass. Nothing in this packet may state a flow figure
  or a verdict that the Step 5 record does not contain.

## Risks and Tradeoffs

- **The bracket selection is the design's crux.** The tree planner's interface roles are
  per-node/per-layer (Roof/Floor/Base counters seeded at contact creation), not per
  contiguous run; the demanded planes are the layers whose entries carry interface roles,
  which sit at region boundaries. The Q1 binding decision resolves this: partition by
  `(object_id, region_id)` contiguous run; with count(interface-role planes) >= 2 the
  bracket set is the sorted/deduplicated interface planes (endpoints not added); with
  fewer than two, that set is supplemented with the run's first/last surviving
  support-bearing rows and sorted/deduped. The fallback is now specified, so the stack can no
  longer silently stay grid-bound for interface-less regions, and a lone genuine interface
  plane is never demoted to body.
- **The body-row replacement touches the skeleton.** The Q2 binding decision resolves the
  source: clone the lower bracket, rewrite roles to `SupportBody`, capture the source
  `global_layer_index` into the local duplicate key/clone-source provenance decision, and
  assign the emitted final `global_layer_index` from the per-plane DEV-163 synthetic
  identity map.
  A wrong implementation would produce interface geometry at body planes; AC-2/AC-3 now
  assert the role rewrite directly.
- **The `support_step` neutralization has a blast radius.** The traditional planner's tests
  pin the decimation behaviour; the Step 3 dispatch inventories them before editing. Widening
  a tolerance to make a change pass is gaming the gate and is forbidden.
- **The human gate cannot close today.** Both reference files are verified absent
  (`REFS-ABSENT-GATE-OPEN`), and only a human can produce them (with a support-extruder
  nozzle whose max layer height yields the 0.3 mm pitch — canonical has no
  `support_layer_height_mm` key). The packet reaches "all steps complete, sign-off pending"
  and stops there; that is the designed outcome, not a failure.
- **Guest staleness.** Every step here edits guest-feeding paths, so essentially every
  failure in this packet is a stale-guest suspect until `cargo xtask build-guests --check`
  returns exit `0`.
- **The E=0 defect class is a test assertion, not a gate finding.** AC-1 asserts extrusion
  presence on every off-grid support row; the 239c artifact regression ("it renders
  nothing") was caught only by human-gate inspection, and this packet must not repeat that.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Steps 2, 3, 4, 7, 8)
- Highest-risk dispatch and required return format: the Step 5 flow measurement over
  `crates/slicer-gcode/` — return `FACT` only (three numbers plus the verdict word). A
  dispatch that returns emitter source instead of numbers has failed and must be re-issued;
  the falsifiability of AC-4 rests on that one return.

## Open Questions

- **[FWD] Q4 — human-gate reference nozzle (the only remaining open question; human-reference
  only, non-code, non-blocking).** The human producing
  `tmp/p239d-orca-ref-*-coarse.gcode` must choose a support-extruder nozzle whose
  `max_layer_height_from_nozzle` yields the 0.3 mm pitch. The implementer records the
  recommended nozzle diameter in the gate document at Step 7; the gate stays
  `REFS-ABSENT-GATE-OPEN` until a human produces the references.

## Recorded Decisions (binding; formerly `[FWD]` Q1-Q3)

- **Decision D1 (bracket selection, resolves Q1).** Partition demanded rows by
  `(object_id, region_id)` and contiguous run. Let I be the run's interface-role planes
  (`TopInterface`/`BaseInterface`/`BottomInterface`). With count(I) >= 2 the bracket set is
  the sorted/deduplicated I (endpoints not added); with count(I) < 2 that set is
  supplemented with the run's first and last surviving support-bearing rows, then
  sorted/deduplicated by `anchor_z`. A run with exactly one genuine interface plane keeps it
  as a bracket (never demoted to body).
- **Decision D1a (binding coarse/finer predicate).** For each consecutive demanded bracket
  pair, `local_support_gap` is the maximum positive anchor-Z difference between
  consecutive surviving support-bearing rows of that same `(object_id, region_id)`
  contiguous run covered by the bracket; these rows are already available to both planner
  callers. Take the coarse path iff the configured nonzero pitch >=
  `local_support_gap` (exact canonical units,
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only tolerance if needed —
  no new epsilon), otherwise retain the existing 239c
  finer derivation for that bracket. AC-N4's concrete semantics: configured pitch 0.2 over
  covered surviving-row gaps 0.3 stays finer even if the object's first/base layer gap is
  0.2. The `support_layer_height_mm == 0.0` sentinel is separate: it bypasses the coarse
  path entirely and preserves the existing 239c object-grid behavior; never claim a
  resolved pitch equals the varying `local_support_gap` values.
- **Decision D2 (stack-plane clone source, resolves Q2).** Clone the lower bracket's
  geometry; rewrite the roles to `SupportBody`; capture the source `global_layer_index`
  into the local duplicate key and clone-source provenance decision only, and assign the
  emitted entry's final `global_layer_index` from the per-plane DEV-163 synthetic identity
  map (`BTreeMap<i64, i32>`); preserve other provenance fields.
  Never produce interface roles/geometry at body planes. Genuine interface bracket entries
  survive untouched (only non-interface rows strictly inside each bracket pair are
  replaced). The stable dedup key applies to synthesized candidates only; surviving real
  entries are never deduplicated by emitted `global_layer_index`.
- **Decision D3 (`support_step` neutralization, resolves Q3).** Set `support_step = 1`
  exactly for bracket pairs satisfying the binding coarse predicate (configured nonzero
  pitch >= `local_support_gap`, per D1a's predicate); no
  global bypass of the decimation gate; the finer direction stays bit-identical.
- **Decision D4 (grouping).** Reproduce `generate_support_layers`
  (`Support/SupportCommon.cpp`) EPSILON candidate grouping + midpoint. Do **not**
  reproduce its minimum group-height rule: `SupportPlanEntry` has no height field, so
  canonical's group height is representation-inapplicable in PnP; effective row height
  derives from adjacent `anchor_z`.
- **Decision D5 (family-specific formulas).** Traditional:
  `ceil((dist - EPSILON) / pitch)` per `raft_and_intermediate_support_layers`
  (`Support/SupportMaterial.cpp`). Tree: `ceil(dist / pitch)` per `plan_layer_heights`
  (`TreeSupport.cpp`), no EPSILON bias. Never one shared formula.
