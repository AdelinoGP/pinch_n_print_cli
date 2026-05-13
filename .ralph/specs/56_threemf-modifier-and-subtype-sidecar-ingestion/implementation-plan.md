# Implementation Plan: 56_threemf-modifier-and-subtype-sidecar-ingestion

## Execution Rules

- One atomic step at a time.
- Each step must map back to TASK-190..193.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are not optional — they are the budget contract.
- Aggregate context cost is **L**. Activation Blocker Q1 must be resolved before this plan runs. The implementer dispatches each step to a fresh sub-agent / worker.

## Steps

### Step 0: WIT-mirror gate re-run + sidecar-presence sanity check

- Task IDs:
  - `TASK-190` (precursor)
- Objective: Re-verify the Step-0 gate at implementation time. Author-time gate says clean (host-only). Implementer confirms before touching any code.
- Precondition: Packet activated. Activation Q1-Q4 resolved.
- Postcondition: Either (a) gate confirmed clean → continue; or (b) `ModifierVolume` mirror discovered in `wit/**` or `wit_host.rs` → halt and register DEV-043-style escalation; pause for user authorization.
- Files allowed to read: none directly. Pure dispatch step.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: `crates/slicer-macros/src/lib.rs`, all `wit/**` (dispatch only).
- Expected sub-agent dispatches:
  - Question: "Does any guest-visible WIT type or its host mirror expose `ObjectMesh.modifier_volumes` or a `ModifierVolume` shape? Scope: `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`. Return FACT yes/no with file:line if yes; ≤ 8 lines."
  - Question: "Confirm `resources/benchy_painted.3mf` does NOT contain a `Metadata/model_settings.config` entry. Return FACT yes/no with the archive listing line count; ≤ 3 lines."
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegate SUMMARY only if gate flips.
- OrcaSlicer refs: none in this step.
- Verification: Both FACT dispatches return clean.
- Exit condition: Step 1 may begin.

### Step 1: Sidecar parser + `PartSubtype` enum + TDD-RED

- Task IDs:
  - `TASK-190`
- Objective: Introduce a host-local `PartSubtype` enum and a `parse_3mf_sidecar` helper that reads `Metadata/model_settings.config` from a `zip::ZipArchive`. Author the failing TDD that asserts the parser's API surface against a synthetic minimal sidecar string AND the `resources/benchy_4color.3mf` sidecar.
- Precondition: Step 0 clean.
- Postcondition: New tests fail (compile or assertion) with messages naming the missing `parse_3mf_sidecar` symbol. `model_loader.rs` is otherwise unchanged.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-203, 285-360, 430-587.
  - `crates/slicer-ir/src/slice_ir.rs` — lines 230-265.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — NEW.
  - `crates/slicer-host/src/model_loader.rs` — add empty `parse_3mf_sidecar` stub returning `HashMap::new()` so the test compiles to a RED assertion failure rather than a compile error.
- Files explicitly out-of-bounds: every other file in `crates/slicer-host/src/`, all `wit/**`.
- Expected sub-agent dispatches:
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`. Return LOCATIONS with one-line role each; ≤ 8 entries."
  - Question: "Run `unzip -p resources/benchy_4color.3mf Metadata/model_settings.config | head -80`. Return the raw output verbatim; SNIPPETS."
  - Question: "Run `cargo check -p slicer-host --tests` after Step 1's edits. Return FACT pass/fail and one assertion line on failure."
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — lines 192-211 for `ConfigDelta`/`ModifierVolume`. Read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — delegate (function names only).
- Verification:
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd parses_benchy_4color_sidecar -- --exact --nocapture` — expected RED.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd missing_sidecar_returns_empty_map -- --exact --nocapture` — expected GREEN even pre-impl (stub returns empty).
- Exit condition: TDD-RED assertion message shows the missing parse path.

### Step 2: Implement sidecar parser to make Step 1 tests GREEN

- Task IDs:
  - `TASK-190`
- Objective: Implement `parse_3mf_sidecar` so the Step 1 TDD passes. Handle: (a) present + well-formed sidecar; (b) missing sidecar (silent default); (c) malformed sidecar (log::warn! + fallback); (d) unknown subtype (log::warn! + downgrade).
- Precondition: Step 1 RED.
- Postcondition: All `threemf_sidecar_classification_tdd.rs` tests GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-203, 555-587.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/model_loader.rs` — sidecar parser implementation (or extract to `model_loader_sidecar.rs` if file exceeds 800 lines).
  - `crates/slicer-host/src/model_loader_sidecar.rs` — NEW if needed.
- Files explicitly out-of-bounds: all WIT and macros files; all non-`model_loader*` host source files.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_sidecar_classification_tdd`. Return FACT pass-count vs total."
  - Question: "Return the existing `quick_xml::Reader` parse-loop pattern used in `crates/slicer-host/src/model_loader.rs::parse_3mf_model_xml`. SNIPPETS, ≤ 30 lines."
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` line 5 — versioning rule (informational).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — already named in Step 1 (no fresh dispatch).
- Verification:
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — expected all GREEN.
  - `cargo clippy -p slicer-host --tests -- -D warnings` — expected GREEN.
- Exit condition: Sidecar parser unit suite fully GREEN; clippy clean.

### Step 3: Branch `resolve_object` to route non-`normal_part` geometry into `modifier_volumes` (TDD-RED)

- Task IDs:
  - `TASK-191`
- Objective: Add a failing E2E TDD asserting that `load_model("resources/benchy_4color.3mf")` returns (a) `mesh.indices.len() / 3 == 225_240` (the cube excluded from solid mesh), (b) `modifier_volumes.len() == 1`, (c) the modifier's `config_delta` carries `subtype = "modifier_part"` and `fuzzy_skin = "external"`, (d) `MeshIR.schema_version == { major: 1, minor: 1, patch: 0 }`.
- Precondition: Step 2 GREEN.
- Postcondition: New test compiles and fails on assertion (current `modifier_volumes.is_empty()` for 3MF).
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-552.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — NEW.
- Files explicitly out-of-bounds: everything else.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh`. Return FACT pass/fail."
  - Question: "Confirm `resources/benchy_4color.3mf` exists and is readable. Return FACT yes/no + file size."
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none new.
- Verification: TDD-RED with expected `assertion failed: 225_252 == 225_240` (or analogous).
- Exit condition: TDD-RED present.

### Step 4: Implement `resolve_object` branching + schema bump

- Task IDs:
  - `TASK-191`
- Objective: Thread sidecar classification through `resolve_object`. Non-`normal_part` parts contribute a `ModifierVolume` to the accumulator instead of merging triangles. Drop paint data on non-`normal_part` rows with `log::warn!`. Bump `MeshIR.schema_version` to 1.1.0.
- Precondition: Step 3 RED.
- Postcondition: Step 3 TDD GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-552.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/model_loader.rs` — `resolve_object` signature widens; `load_model` constructs `ModifierVolume` per non-`normal_part` part; `SemVer { 1, 0, 0 }` → `SemVer { 1, 1, 0 }` at lines 195-199.
- Files explicitly out-of-bounds: every other source file.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd`. Return FACT pass/fail per test."
  - Question: "Return the existing `apply_transform_to_mesh` signature from `crates/slicer-host/src/model_loader.rs`. SNIPPETS, ≤ 10 lines."
  - Question: "What is `ConfigKey`'s constructor signature in `crates/slicer-ir/`? Can it accept arbitrary strings?" FACT.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` lines 5, 192-211. Read directly.
- OrcaSlicer refs: none new.
- Verification:
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_volume_carries_typed_metadata -- --exact --nocapture`
  - `cargo test -p slicer-host --test threemf_transform_tdd` — must stay GREEN (regression).
- Exit condition: Both `modifier_part_excluded_*` and `modifier_volume_carries_*` GREEN; `threemf_transform_tdd` stays GREEN.

### Step 5: Fuzzy-skin manifest schema confirmation (gate)

- Task IDs:
  - `TASK-192`
- Objective: Verify `apply-to-all` is declared in `modules/core-modules/fuzzy-skin/manifest.toml`'s `[config.schema]`. If absent, add it (additive; no SemVer ripple). If present, this step is a no-op.
- Precondition: Step 4 GREEN.
- Postcondition: `fuzzy-skin` manifest includes `apply-to-all` config schema entry.
- Files allowed to read:
  - `modules/core-modules/fuzzy-skin/manifest.toml`.
  - `modules/core-modules/fuzzy-skin/src/lib.rs` — lines 1-120 (already seen at packet-author time; re-check only if necessary).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/fuzzy-skin/manifest.toml` — additive only.
- Files explicitly out-of-bounds: all other files.
- Expected sub-agent dispatches:
  - Question: "Does `modules/core-modules/fuzzy-skin/manifest.toml`'s `[config.schema]` block declare an entry for `apply-to-all`? FACT yes/no with file:line if yes."
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — module manifest TOML schema. Delegate SUMMARY if needed.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` — expected GREEN.
  - `cargo build --workspace` — expected GREEN.
- Exit condition: `apply-to-all` present in manifest; build clean.

### Step 6: Region-mapping overlap stamp for `modifier_part` (TDD-RED + GREEN)

- Task IDs:
  - `TASK-192`
- Objective: Extend `execute_region_mapping` to accept per-object modifier volumes (or read them off `ExecutionPlan`); compute per-layer 2D projection of each modifier; for each `(layer, region)`, run `slicer_core::polygon_ops::intersection`; on non-empty overlap stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`. TDD-RED first.
- Precondition: Step 4 GREEN.
- Postcondition: `region_overlap_stamps_only_in_cube_zband` GREEN; `fuzzy_region_restricted_to_cube_and_painted_facets` GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` — lines 1-260.
  - `crates/slicer-host/src/config_resolution.rs` — lines 80-220 (for `ConfigKey`/`ConfigValue` helpers).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/region_mapping.rs` — overlap stamp loop.
  - `crates/slicer-host/src/pipeline.rs` — thread `modifier_volumes` into the region-mapping call.
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — extend (already created at Step 3).
- Files explicitly out-of-bounds: `crates/slicer-macros/`, `wit/**`, `crates/slicer-sdk/`.
- Expected sub-agent dispatches:
  - Question: "Which function in `slicer-core` slices an `IndexedTriangleSet` at a given Z plane and returns 2D polygons in scaled integer units? Return FACT with function path." (Used to project each modifier per layer.)
  - Question: "Return `slicer_core::polygon_ops::intersection` signature. SNIPPETS, ≤ 6 lines."
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd region_overlap_stamps_only_in_cube_zband fuzzy_region_restricted_to_cube_and_painted_facets`. Return FACT pass/fail per test."
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility. Delegate SUMMARY.
  - `docs/08_coordinate_system.md` — coordinate hazards. Read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `modifier_part`-overlap fuzzy_skin application function name. Delegate LOCATIONS.
- Verification:
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — all four tests GREEN.
  - `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — must stay GREEN (regression).
- Exit condition: All four `benchy_4color_modifier_part_e2e_tdd` tests GREEN; regression suite green.

### Step 7: New host stage `apply_negative_part_subtract` (TDD-RED + GREEN)

- Task IDs:
  - `TASK-192`
- Objective: Add a new host stage that runs between prepass and region-mapping, performing per-layer 2D subtract via `slicer_core::polygon_ops::difference`. TDD-RED first; the test builds a synthetic in-memory 3MF archive with `subtype="negative_part"` and asserts post-subtract per-layer polygon area is strictly less than pre-subtract.
- Precondition: Step 6 GREEN.
- Postcondition: `negative_part_removes_layer_polygon_area` GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/pipeline.rs` — full.
  - `crates/slicer-host/src/prepass.rs` — entry-point line range (delegate FACT for line range).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW (stage implementation).
  - `crates/slicer-host/src/pipeline.rs` — insert stage call between prepass and region-mapping.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW (test).
- Files explicitly out-of-bounds: all other files.
- Expected sub-agent dispatches:
  - Question: "Return the exact insertion point in `crates/slicer-host/src/pipeline.rs` between `execute_prepass_*` and `execute_region_mapping`. FACT with file:line."
  - Question: "Show how `crates/slicer-host/tests/threemf_transform_tdd.rs` builds an in-memory `zip` archive for synthetic 3MF fixtures. SNIPPETS, ≤ 30 lines."
  - Question: "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area`. FACT pass/fail."
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — prepass ordering. Delegate SUMMARY.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — negative-part subtract function name. Delegate LOCATIONS.
- Verification:
  - `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area -- --exact --nocapture` — GREEN.
  - `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — must stay GREEN (regression).
- Exit condition: Negative-part synthetic test GREEN; existing tests unbroken.

### Step 8: Support enforcer / blocker paint-segmentation piggyback (TDD-RED + GREEN)

- Task IDs:
  - `TASK-192`
- Objective: Project each `support_enforcer` / `support_blocker` modifier volume per layer and emit synthetic `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. Use the existing paint-segmentation pipeline (Packet 50b / 51) without modifying its core logic.
- Precondition: Step 7 GREEN.
- Postcondition: `support_enforcer_emits_paint_region` and `support_blocker_emits_paint_region` GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/paint_segmentation.rs` — full.
  - `crates/slicer-host/src/region_mapping.rs` — overlap pattern (already touched at Step 6).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/paint_segmentation.rs` — augment to accept and emit modifier-derived paint regions.
  - `crates/slicer-host/src/pipeline.rs` — thread `modifier_volumes` of support subtypes into the paint-segmentation call.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — extend.
- Files explicitly out-of-bounds: WIT, SDK, macros.
- Expected sub-agent dispatches:
  - Question: "Return the existing `harvest_paint_segmentation_ir` entry point and the place where `PaintRegionIR` is assembled. SNIPPETS, ≤ 30 lines."
  - Question: "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region support_blocker_emits_paint_region`. FACT pass/fail per test."
  - Question: "Run `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation`. FACT pass/fail (regression check)."
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `PaintSemantic::SupportEnforcer` / `Blocker`. Already read at packet-author time.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — support enforcer/blocker geometry function names. Delegate LOCATIONS.
- Verification:
  - `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` — all subtype tests GREEN.
  - Packet 50b/51 regression tests GREEN.
- Exit condition: Support enforcer/blocker tests GREEN; regressions clean.

### Step 9: No-regression sweep (no-sidecar + transform + gcode)

- Task IDs:
  - `TASK-193`
- Objective: Re-run all regression-defense test suites and assert clean.
- Precondition: Steps 1-8 GREEN.
- Postcondition: All regression suites GREEN.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`. Return FACT pass/fail per file with totals."
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All four FACTs GREEN.
- Exit condition: No regressions.

### Step 10: Clippy + check sweep

- Task IDs:
  - `TASK-193`
- Objective: Confirm lint + build cleanliness.
- Precondition: Step 9 GREEN.
- Postcondition: `cargo clippy --workspace -- -D warnings` GREEN; `cargo check --workspace` GREEN.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): any source file the lint pass demands (sticking to files-in-scope from earlier steps).
- Files explicitly out-of-bounds: macros, WIT, SDK.
- Expected sub-agent dispatches:
  - Question: "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail."
  - Question: "Run `cargo check --workspace`. FACT pass/fail with first error if fail."
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: Both FACTs GREEN.
- Exit condition: Clean workspace.

### Step 11: Doc + deviation registration

- Task IDs:
  - `TASK-193`
- Objective: Update `docs/02_ir_schemas.md` (IR 0 schema_version header), `docs/07_implementation_status.md` (append TASK-190..193 rows), `docs/DEVIATION_LOG.md` (register DEV-047/048/049 as Closed by Packet 56), `docs/14_deviation_audit_history.md` (chronology entries).
- Precondition: Step 10 clean.
- Postcondition: Docs reflect packet outcome.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — lines 62-244 only.
  - `docs/07_implementation_status.md` — append-only (delegate read of full file).
  - `docs/DEVIATION_LOG.md` — append.
  - `docs/14_deviation_audit_history.md` — append.
- Files allowed to edit (≤ 3 per dispatch; 4 files spread across two dispatches):
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - Question: "Append four `[x] TASK-19N` rows after the TASK-181 row in `docs/07_implementation_status.md`, each naming packet 56_threemf-modifier-and-subtype-sidecar-ingestion. Return the resulting four lines verbatim. SNIPPETS."
  - Question: "Confirm next free DEV-### slot in `docs/DEVIATION_LOG.md`. Return FACT (highest existing DEV-### + 1)."
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` line 5, 250, 506 — patterns for the "Current schema_version" header.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'schema_version: 1\.1\.0.*packet 56' docs/02_ir_schemas.md`
  - `rg -q '\[x\] TASK-190.*56_threemf' docs/07_implementation_status.md`
  - `rg -c '^\| DEV-04[789].*Closed.*Packet 56' docs/DEVIATION_LOG.md` (expected: 3)
- Exit condition: All `rg` checks return 0 exit code with the expected match counts.

### Step 12: Packet acceptance ceremony

- Task IDs:
  - All packet TASK ids.
- Objective: Dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` to a worker and record FACT pass/fail. If any criterion fails, return to the relevant step; do not flip status.
- Precondition: Steps 0-11 GREEN.
- Postcondition: All AC commands GREEN; `packet.spec.md` ready to flip to `status: implemented`.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): `packet.spec.md` (status flip on success only).
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command, each returning FACT pass/fail.
  - Final dispatch: "Run `cargo test --workspace` per CLAUDE.md Test Discipline acceptance-ceremony allowance. Return FACT pass-count vs total."
- Context cost: `M`
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands return PASS.
  - `cargo test --workspace` returns all-pass.
- Exit condition: Status flippable to `implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 0 | S | Pure dispatch. |
| Step 1 | M | New test file + parser stub. |
| Step 2 | M | Sidecar parser implementation. |
| Step 3 | S | New E2E test file (RED). |
| Step 4 | M | `resolve_object` widening + schema bump. |
| Step 5 | S | Manifest-only edit. |
| Step 6 | M | Region-mapping overlap stamp + pipeline thread. |
| Step 7 | M | New host stage + new synthetic-3MF tests. |
| Step 8 | M | Paint-segmentation piggyback for support subtypes. |
| Step 9 | S | Regression sweep. |
| Step 10 | S | Clippy + check sweep. |
| Step 11 | S | Doc + deviation registration. |
| Step 12 | M | Acceptance ceremony. |

Aggregate: **L** (7 M + 6 S). Per skill rule, this packet must be split before activation OR the user explicitly authorizes the L-aggregate. See `packet.spec.md` Activation Blocker Q1.

If split into Packet 56 (Steps 1-4, 9-11; M aggregate) + Packet 57 (Steps 5-6, 9-11; M aggregate) + Packet 58 (Step 7 + regression; M) + Packet 59 (Step 8 + regression; M), each child packet meets the M-aggregate rule.

## Packet Completion Gate

- All 13 steps complete.
- Every step exit condition met.
- Packet acceptance criteria GREEN (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-190..193 (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- DEV-047/048/049 registered in `docs/DEVIATION_LOG.md` and chronology in `docs/14_deviation_audit_history.md`.
- WIT-mirror gate confirmed clean (Step 0 + Step 12 final check).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (Step 12).
- Confirm packet-level verification commands are GREEN (Steps 9, 10, 12).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson and consider tightening Step 7 / Step 8 dispatch boundaries.
- Run `cargo test --workspace` exactly once at ceremony close via worker FACT dispatch. Never absorb the output directly.
