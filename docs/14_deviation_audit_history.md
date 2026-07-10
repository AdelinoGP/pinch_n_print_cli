# Deviation Audit History

Last updated: 2026-05-15 (DEV-053 chronology entry added — Packet 56b spec-review)

## Purpose

This file preserves the minimum historical context from the retired audit artifacts:

- `docs/deviationList.xml`
- `docs/14_deviation_audit_tasks_1.md`
- `docs/14_deviation_audit_tasks_2.md`

Use this file for provenance, chronology, and legacy-reference lookup only.

- Use `docs/DEVIATION_LOG.md` for the live architecture deviation registry.
- Use `docs/07_implementation_status.md` for the active remediation backlog.
- Use `docs/11_operational_governance_and_acceptance_gate.md` and `docs/12_architecture_gate_metrics.md` for release-gate policy and evidence thresholds.

## Artifact Replacement Map

| Retired artifact | Former role | Canonical replacement |
| --- | --- | --- |
| `docs/deviationList.xml` | Working audit registry, blocker rollup, and legacy numbering source | `docs/DEVIATION_LOG.md` for live rows, `docs/07_implementation_status.md` for active tasks, this file for legacy crosswalk and audit chronology |
| `docs/14_deviation_audit_tasks_1.md` | Generated audit instructions and handoff template | This file's audit method summary plus the normalized `docs/DEVIATION_LOG.md` rows |
| `docs/14_deviation_audit_tasks_2.md` | Audit execution results and point-in-time summaries | This file's outcome summary plus the current row state in `docs/DEVIATION_LOG.md` |

## Audit Method Summary

- The 2026-04 audit ran 24 verification-only checks against the implementation and the architecture docs.
- Each audit used the same pattern: read the normative docs first, inspect the decisive implementation path, verify test evidence, then record any still-open drift in `docs/DEVIATION_LOG.md`.
- Fixed or stale legacy claims were closed in the live deviation log instead of being preserved as independent work items.
- Several XML-era topics were merged, split, or retired during that cleanup, so the old `deviation #N` labels are not stable identifiers anymore.

## Outcome Summary

### Key closures and stale legacy claims confirmed during audit

- `DEV-012` closed: the `#[slicer_module]` macro now emits typed WIT export glue for all four worlds.
- `DEV-018` closed: prepass segmentation dispatch is wired on the live host path.
- `DEV-019` closed: paint-annotation execution and warning propagation are wired through the live pipeline.
- `DEV-021` closed: all 17 core modules now ship real component-model `.wasm` artifacts.
- `DEV-022` closed: `ConfigView` immutability and declared-key filtering are enforced on the live path.
- `DEV-028` closed: the layer planner no longer depends on a hand-written duplicate `wit-guest` implementation.
- `DEV-029` closed: blocker-summary drift was merged into the broader planning row `DEV-030`.
- `DEV-025` closed (2026-05-08): all five SDK↔WIT prepass segmentation mismatches resolved. Mismatch 3 (PaintSegmentation output drain non-functional) closed-by-Packet-43 (`43-rev1_macro-prepass-segmentation-output-drain`) via TASK-130a / TASK-130b. Mismatches 4 + 5 closed by Packet 42 (TASK-130c). Mismatches 1 + 2 closed by TASK-128a / TASK-128b. Full TASK-130 cluster (TASK-128a, TASK-128b, TASK-130, TASK-130a, TASK-130b, TASK-130c) retired. See Chronology 2026-05-08 entry for Step 2.5 and Step 2.6 latent-bug details.

### Remaining live architecture themes

This hand-maintained theme list drifted (it predated several closures — e.g. it
listed `DEV-006`/`DEV-025`/`DEV-027` as remaining after they had closed) and has
been retired to remove a second status surface. For the current open set, see the
generated Open Deviation Map in `docs/07_implementation_status.md`, which
`cargo xtask check-deviations` keeps in sync with the authoritative
`docs/DEVIATION_LOG.md`. This file retains only the **Chronology** and crosswalk
below, which it uniquely owns.

## Chronology

- **2026-04-28 — DEV-032 resolved**: Entity-ordering algorithm migrated from `slicer-host::layer_executor::order_entities_by_nearest_neighbor` into `path-optimization-default::run_path_optimization` via the packet-32 `layer-collection-builder` WIT surface (`set-entity-order`/`get-ordered-entities`). The NN heuristic (start at `(0,0)`, Euclidean distance to `start_point`, 0.001mm equality tiebreak, BridgeInfill priority, lower `original_index` stable tiebreak, reversal `false`) is preserved bit-identically. Host helper deleted; fallback behavior is now raw `assemble_ordered_entities` order. Packet 18 (`18_path-optimization-entity-ordering`) marked `superseded`. Tracked by `TASK-152g` (closed) and `TASK-152h` (closed); parent `TASK-152` stays `[~]`.

- **2026-05-03 — DEV-035, DEV-036, DEV-037, DEV-038 registered**: Packet `12-rev1_external-surface-classification-at-slice` wired `is_top_surface`, `is_bottom_surface`, and `is_bridge` fields into `SlicedRegion` and bumped `SliceIR.schema_version` to 1.1.0. Four deviations registered: DEV-035 (any-vertex-in-polygon approximation vs. OrcaSlicer polygon-expansion), DEV-036 (`bridge_regions` empty in production at `mesh_analysis.rs:213`), DEV-037 (WIT contract scope expansion — three flag accessor methods added to `slice-region-view` in `wit/deps/ir-types.wit:72-74`, macro adapter and gyroid-infill wired, WASM rebuild triggered), DEV-038 (latent retract-evidence test pattern fix in `benchy_feature_evidence_failures_name_the_missing_family`). DEV-037 and DEV-038 are closed; DEV-035 and DEV-036 are open pending packet 36. Tracked by `TASK-164`.

- **2026-05-07 — DEV-047 registered** (renumbered 2026-05-13 from DEV-039; the original DEV-039 in `docs/DEVIATION_LOG.md` is the 2026-05-04 packet-35 bbox fallback and predates this entry): Packet `39_stable-entity-ids` bumped `SliceIR.schema_version` from 1.2.0 to 2.0.0 (major bump). Breaking changes: `TravelMove.after_entity_index: u32` replaced by `TravelMove.entity_id: u64` (rename + type change, breaking per docs/02_ir_schemas.md:889 versioning rule). Additive change: `PrintEntity` gained new field `entity_id: u64`. Combined change is breaking → major version bump. Impact: internal-only — no external persisted IR consumers affected, but backward-incompatible with any pre-Packet-39 serialized IR snapshots. Tracked by packet `39_stable-entity-ids`. **WIT-boundary scope expansion (logged retroactively):** the original packet's Step 0 WIT-exposure check was scoped only to `TravelMove`/`entity_idx` and missed `PrintEntity`'s exposure via `print-entity-view`. Three groups of files marked out-of-bounds in `design.md` were edited as structural necessities: (a) `wit/world-finalization.wit` and `crates/slicer-macros/src/lib.rs` to add `entity-id: u64` to `print-entity-view` and the macro-generated PrintEntity construction; (b) `crates/slicer-host/src/wit_host.rs` host marshaller to populate the new field at the boundary; (c) `modules/core-modules/wipe-tower/src/lib.rs` and `modules/core-modules/skirt-brim/src/lib.rs` to stamp `entity_id` at their direct `PrintEntity` construction sites (these modules bypass `FinalizationOutputBuilder`). Items (a) and (b) are closed by Packet 39. Item (c) is provisional — the local `LayerEntityIdGen` instances should be migrated to a builder-issued ID once Packet 40 lands its mutation API; carry-forward tracked by Packet 40 Step 0.

- **2026-05-08 — DEV-025 mismatches 4 + 5 registered and closed**: Packet `42_paint-region-transport-widening` identified two additional structural mismatches in the paint-region transport layer. Mismatch 4 (paint value channel string-coerced): `paint-region-entry.value: string` was parsed by the host via a four-grammar guesser falling back to `ToolIndex(0)`, silently degrading `Custom`-semantic non-numeric values. Mismatch 5 (SDK paint-region polygons hole-blind): `PaintRegionEntry::contour_points: Vec<[f64;2]>` cannot represent interior holes; OrcaSlicer's MMU segmentation natively produces ExPolygons with Clipper-convention holes. Both mismatches closed by Packet 42 (TASK-130c) on 2026-05-08. Mismatch 3 remains open (tracked by Packet 43).

- **2026-05-08 — DEV-025 mismatch 3 closed; TASK-130 / TASK-130a / TASK-130b cluster closed; DEV-025 fully closed**: Packet `43-rev1_macro-prepass-segmentation-output-drain` closed the last open mismatch in DEV-025 and fully retired the TASK-130 cluster (TASK-128a, TASK-128b, TASK-130, TASK-130a, TASK-130b, TASK-130c). **Mismatch 3 — PaintSegmentation output drain (closed-by-Packet-43):** `build_prepass_world_glue` in `crates/slicer-macros/src/lib.rs` now iterates `sdk_output.regions()` and calls `_output.push_paint_region` for each region, replacing the previously hollow arm body that discarded the SDK output entirely. Two latent macro/host bugs were surfaced and bounded during implementation: **Step 2.5 — macro paint_seg_arm scope fix:** the inline-WIT block at line 1317 was extended from `use geometry.{ex-polygon};` to `use geometry.{ex-polygon, polygon, point2};`, and two explicit Rust `use` statements added to the `segmentation_helpers` quote block (`use self::slicer::world_prepass::geometry::Polygon;` and `use self::slicer::world_prepass::geometry::Point2;`) mirroring the finalization-world pattern — required because wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose `modes_of()` returns empty. **Step 2.6 — host layer-idx alignment with canonical `wit/deps/ir-types.wit`:** host inline WIT in `crates/slicer-host/src/wit_host.rs` now declares `type layer-idx = s32` matching the canonical `s32`; the four non-paint view records explicitly retain `u32` to match the macros crate WIT; negative-index rejection added in the `push_paint_region` validator; i32→u32 cast added in `dispatch.rs` harvest at the IR boundary. All five DEV-025 mismatches are now closed; `docs/DEVIATION_LOG.md` DEV-025 status updated to `Closed`. TASK-130, TASK-130a, TASK-130b marked `[x]` in `docs/07_implementation_status.md`; blocker list updated to remove TASK-130a and TASK-130b.

- **2026-05-10 — DEV-044 + DEV-045 registered**: Spec-review of Packet `43-rev1_macro-prepass-segmentation-output-drain` (closed 2026-05-08) surfaced two distinct gaps in the paint-segmentation surface that the Acceptance Gate (`docs/11_operational_governance_and_acceptance_gate.md:77-86`) was too lenient to catch. **DEV-044** (Paint input surface stubbed): `crates/slicer-host/src/model_loader.rs:280-352` (`parse_3mf_model_xml`) parses only `<vertex>`/`<triangle>` XML; every Bambu/Orca paint namespace (`custom_supports`, `paint_color`, `support_blocker`, `seam_painting`) is silently discarded, and `ObjectMesh::paint_data` is unconditionally `None` at line 150. Neither `slicer-host` nor the dev `slicer-cli` exposes a paint flag. Every code path downstream of `paint_data` — `paint_segmentation.rs:70-130`, `wit_host.rs:2498/2653`, the layer-world `paint-region-layer-view` at `wit/deps/ir-types.wit:194-218` — operates on always-None input on the live binary path. PaintSegmentation is contract-green (DEV-025 closed 2026-05-08) but unfalsifiable end-to-end. **DEV-045** (RegionMap paint-blind): the host built-in `crates/slicer-host/src/region_mapping.rs:103-248` contains zero "paint*"/"semantic" tokens; `RegionPlan` (`crates/slicer-ir/src/slice_ir.rs:1028-1033`) has no paint-semantic dimension; `crates/slicer-host/src/config_resolution.rs` recognises only `object_config:` (not `paint_config:`). `PaintSemantic::Custom("fuzzy_skin")` and similar values cross IR via `PaintRegionLayerView` but cannot bind to per-region `ResolvedConfig` overrides — paint is useful today only for tool/material differentiation via `ActiveRegion.tool_index`. Failing TDD-RED tests committed in `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (DEV-044, 2 tests) and `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` (DEV-045, 1 test). Closures planned via Packet 50 (`paint-input-3mf-ingestion`) and Packet 51 (`paint-semantic-region-overrides`); Packet 51 depends on Packet 50 for end-to-end testability. Scope intentionally `.3mf` only (STL+sidecar JSON ingestion rejected as YAGNI per user direction).

- **2026-05-11 — DEV-044 closed**: Packet `50_paint-input-3mf-ingestion` extended `parse_3mf_model_xml` to recognize the `paint_fuzzy_skin` attribute on `<triangle>` elements, decode it as `PaintValue::Flag(true)` when the value is `"4"`, and populate `ObjectMesh::paint_data` with a `PaintLayer` carrying `PaintSemantic::FuzzySkin`. Whole-facet only; subdivision deferred. DEV-044 status flipped to `Closed — Packet 50, 2026-05-11` in `docs/DEVIATION_LOG.md`; TASK-180 added to `docs/07_implementation_status.md`.

- **2026-05-13 — DEV-009 cooling subset closed**: Packet `53_gcode-cooling-fan-emission` closes the cooling subset of DEV-009. Cooling fan control (M106/M107) is now emitted from a live `PostPass::LayerFinalization` cooling module, replacing the previously documented rejection on `Layer::PathOptimization` (TASK-152c superseded by TASK-152d). TASK-152c marked superseded in `docs/07_implementation_status.md`; rejection wording removed from `docs/05_module_sdk.md`.

- **2026-05-14 — DEV-050 + DEV-051 registered and closed**: Packet `56_threemf-sidecar-parser` introduced a host-internal parser for the OrcaSlicer / Bambu Studio sidecar `Metadata/model_settings.config`. **DEV-050** (Partial subtype coverage): `parse_part_subtype` in `crates/slicer-host/src/model_loader_sidecar.rs` recognises exactly five subtypes (`normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`); any unrecognized value is downgraded to `PartSubtype::NormalPart` with `log::warn!`. Registered and closed immediately by Packet 56 — the downgrade is a deliberate design choice to keep the loader forward-compatible. **DEV-051** (Missing or malformed sidecar non-fatal): `parse_3mf_sidecar` returns `HashMap::new()` silently for a missing sidecar, and returns `HashMap::new()` plus `log::warn!` ("treating all parts as normal_part") for malformed XML. `load_model` does not return `Err`. Registered and closed immediately by Packet 56. Both deviations were originally planned as DEV-047 and DEV-049 but were renumbered to DEV-050/DEV-051 because DEV-047 (packet 39 / stable-entity-ids) and DEV-049 (packet 53 / cooling) were already claimed. TASK-190 added to `docs/07_implementation_status.md`. 7 TDD tests in `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs`; all regression suites pass.

- **DEV-052** (2026-05-14): Paint data dropped on non-`NormalPart` rows. Registered and closed by Packet 56b (`56b_threemf-modifier-part-ir-routing`).

- **DEV-053** (2026-05-15): Config key normalization (kebab→snake) in `modules/core-modules/fuzzy-skin/src/lib.rs`. Discovered during spec-review of Packet 56b: the module was reading `"apply-to-all"` / `"point-distance"` (kebab) while the host stamps `"apply_to_all"` / `"point_distance"` (snake_case per CLAUDE.md convention) and the manifest declares the same snake_case names. Without the fix the modifier-part `apply_to_all = true` stamp would be silently ignored. Two-line change; registered and closed by Packet 56b.

- **2026-07-09 — D-104-OVERHANG-QUARTILE-NONE scope refinement** (Packet 148): Packet 148 refined scope from pipeline-wide to arachne-path-only per-vertex overhang/flag/seam/boundary parity.

- **2026-07-09 — D-104b-OVERHANG-FLOW-NONE registered and closed** (Packet 149): per-vertex `flow_factor` was never reduced for bridge segments (always 1.0). Closed by `slicer_core::flow::bridging_flow(bridge_flow_ratio, thick_bridges)`, applied at `is_bridge` vertices in both `classic-perimeters` and `arachne-perimeters`.

- **2026-07-09 — D-104c-OVERHANG-REVERSE-NONE registered and closed (registration only)** (Packet 149): `detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `extra_perimeters_on_overhangs` were absent from the pipeline entirely. Closed for the registration gap — all four keys now registered on both perimeter manifests with OrcaSlicer defaults (`1`/`0`/`0`/`0`). The reversal BEHAVIOR itself remains unimplemented and is tracked as a follow-up concern, not closed by this packet.

- **2026-07-09 — D-104d-MIN-WIDTH-TOP-SURFACE-NONE registered and closed** (Packet 149): `min_width_top_surface` was absent from the pipeline. Closed by registering it on both perimeter manifests (float mm, default `1.2` ≈ OrcaSlicer's 300% of line width) plus read-and-validate reads in both modules' `lib.rs`. The `only_one_wall_top` threshold behavior this key is meant to gate is deferred.

- **2026-07-09 — D-104e-ALTERNATE-EXTRA-WALL-NONE registered and closed** (Packet 149): `alternate_extra_wall` was absent from the pipeline. Closed by registering it on `arachne-perimeters` and implementing it as a `+2` bump to `ArachneParams.max_bead_count` on odd layers (PnP's beading stack emits `max_bead_count / 2` walls, so `+2` is the PnP equivalent of OrcaSlicer's `loop_number++`), gated on `!spiral_vase && sparse_infill_density > 0`.

- **2026-07-09 — D-104f-CONCENTRIC-INFILL-NO-ARACHNE registered (open, deferred)** (Packet 149): concentric infill does not route through Arachne (OrcaSlicer `FillConcentric.cpp:80-118` / `FillConcentricInternal.cpp:29-55` wiring missing). Not closed by packet 149 — the red test `arachne_parity_pipeline_concentric_infill_uses_arachne` stays red as the explicit fingerprint of this gap. Deferred to a follow-up workstream; no packet is currently scoped to close it.

- **2026-07-09 — D-104g-FLOW-FACTOR-PERVERTEX-DIVERGENCE registered (open)** (Packet 149): PnP models flow as a per-vertex `flow_factor`; OrcaSlicer models a per-path `Flow` (height/width/thread-diameter). The `bridge_flow` ratio is correctly modelable per-vertex; the divergence is realized in the `thick_bridges=true` branch of `bridging_flow()` returning `1.0` instead of OrcaSlicer's height/nozzle-diameter `Flow` computation.

- **2026-07-09 — D-104h-NO-PERCENT-CONFIG-TYPE registered (open, post-implementation audit)** (Packet 149): OrcaSlicer's `coFloatOrPercent`/`coPercent` config types have no `[config.schema]` representation in this codebase; percent-typed keys must be registered as pre-resolved floats. Packet 149 registered `min_width_top_surface` (Orca: 300% of line width) as resolved mm `1.2` (0.4 mm reference line width) and `sparse_infill_density` (Orca: 20%) as raw float `20.0`. The resolved values do not track the user's actual line width; a future implementer should add a percent-or-absolute config type and re-register these keys.

## Legacy Backlog Crosswalk

The status backlog previously referenced XML-era labels such as `deviation #14b` and `deviation #23`. Those labels were retired because the audit cleanup merged several historical entries and rewrote the live registry around stable `DEV-###` rows.

Use the topic-based mapping below when reading older notes or commit history.

| Legacy backlog topic                                                  | Retired XML-era references            | Current canonical tracker                                |
|-----------------------------------------------------------------------|---------------------------------------|----------------------------------------------------------|
| Manifest `ir-access` completeness                                     | `#1`                                  | `DEV-002`, `TASK-121`                                    |
| Runtime access-audit feeding and undeclared-access rejection          | `#2`, `#8`, `#17`                     | `DEV-003`, `TASK-123`, `TASK-124`                        |
| Claim-transition enforcement and related scheduler conflict semantics | `#3`, `#11`, `#18`                    | `DEV-004`, `TASK-125`, `TASK-126`                        |
| Non-planar Z-envelope enforcement                                     | `#4`, `#20`                           | `DEV-005`, `TASK-127`                                    |
| Prepass and layer boundary correctness                                | `#5`, `#6`, `#14`                     | `DEV-006`, `DEV-025`, `TASK-128`, `TASK-129`, `TASK-130`, `TASK-130c` |
| Manifest config-schema completeness                                   | `#7`                                  | `DEV-008`, `TASK-122`                                    |
| Benchy feature parity and regression coverage                         | `#14a`, `#14b`, `#14c`, `#14d`, `#25` | `DEV-009`, `TASK-120a` to `TASK-120d`, `TASK-135`        |
| Progress-event evidence and Python bridge follow-up                   | `#23`, `#24`                          | `DEV-010`, `DEV-024`, `TASK-136`, `TASK-137`             |
| Phase G status drift and dead `Noop*Runner` cleanup                   | `#12`                                 | `DEV-020`, `TASK-139`                                    |
| Acceptance-gate closure and deviation-registry hygiene                | `#15`, `#16`                          | `DEV-010`, `DEV-026`, `DEV-030`, `TASK-140`, `TASK-141`  |
| Entity-ordering algorithm relocation (host helper → path-optimization-default) | —                                 | `DEV-032` (packet 18 → packet 33 via packet 32 WIT surface) |

## Deletion Rationale

The retired XML and audit-task files were useful as temporary working surfaces while the deviation audit was in progress. They became liabilities once the audit results were synchronized into the live docs because they preserved stale numbering, stale blocker summaries, and duplicate status claims that could drift away from `docs/DEVIATION_LOG.md` and `docs/07_implementation_status.md`.

This file exists so the repository retains the audit story without keeping those temporary working files as live reference material.
