# Requirements: 56_threemf-modifier-and-subtype-sidecar-ingestion

## Problem Statement

Two correlated bugs were discovered while validating DEV-046 (3MF `<build>/<item>` transform application) against `resources/benchy_4color.3mf`:

1. **Bug A — Root cause (sidecar ignored).** The 3MF loader at `crates/slicer-host/src/model_loader.rs::resolve_object` (lines 430-552) and `find_model_path` (lines 575-587) reads only `3D/3dmodel.model`. It never opens the OrcaSlicer / Bambu Studio sidecar `Metadata/model_settings.config`, which classifies each `<part>` by `subtype=`. As a consequence, every `<component>` of a parent `<object>` is merged into a single `IndexedTriangleSet` (lines 463-530). For `resources/benchy_4color.3mf` this incorrectly extrudes the 12-triangle modifier cube as solid plastic in the merged 225,252-triangle mesh.

2. **Bug B — Visible symptom (fuzzy applied globally).** The modifier cube in the same fixture carries `<metadata key="fuzzy_skin" value="external"/>` on its `<part>` row in the sidecar. The expected behavior is "fuzzy skin only where the modifier intersects the model, plus any explicitly painted `paint_fuzzy_skin` triangles" (~14,920 of the 225,240 body triangles). The current slice applies fuzzy globally to the whole benchy because (a) Bug A means the modifier marker never reaches the IR, and (b) `crates/slicer-host/src/region_mapping.rs` has no overlap-driven config-stamping path for modifier volumes (Packet 51 wired only paint-semantic overlays; no path exists today for mesh-volume-driven overlays).

The two bugs are tightly coupled — A is the root cause; B is the user-visible symptom. The user has approved a single-packet vertical slice covering all five OrcaSlicer `<part subtype>` values (`normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) end-to-end. The activation gate documents the resulting L-aggregate context cost and the split alternatives.

WIT scope is clean — confirmed by sub-agent Explore against `wit/**`, `wit_host.rs`, and `dispatch.rs`. `ObjectMesh.modifier_volumes` and `ModifierVolume` are host-only types; no DEV-043-style escalation is required. The `MeshIR.schema_version` bump is the only IR-level change and is additive.

Three deviations are registered by this packet and closed at packet close:
- **DEV-NN-A** (recommended ID DEV-047): partial subtype coverage. Unknown subtype values silently downgrade to `normal_part` with a `log::warn!`.
- **DEV-NN-B** (recommended ID DEV-048): paint data on non-`normal_part` parts is dropped at load time with a `log::warn!`.
- **DEV-NN-C** (recommended ID DEV-049): missing or malformed `Metadata/model_settings.config` is non-fatal; loader logs a warning and treats every part as `normal_part`.

This packet does not absorb any prior packet's directory; the Cross-Packet Mutation Rule does not apply. Predecessor packets 50, 50a, 50b (paint ingestion), and 51 (paint-semantic config overlay) are referenced only for pattern reuse — their files are not modified.

## Task IDs (new, registered by this packet)

- **TASK-190** — Parse 3MF sidecar `Metadata/model_settings.config`; classify `<object>`/`<part>` by `subtype=`; surface typed per-part metadata. Covers DEV-047/048/049.
- **TASK-191** — Branch `resolve_object` to route `modifier_part`, `negative_part`, `support_enforcer`, and `support_blocker` geometry into `ObjectMesh.modifier_volumes` instead of merging into the solid mesh. Drops paint data carried on non-`normal_part` rows. Bumps `MeshIR.schema_version` 1.0.0 → 1.1.0 additively.
- **TASK-192** — Wire each subtype's downstream consumer:
  - `modifier_part`: region-mapping direct stamp (user-selected Option 1) — `slicer_core::polygon_ops::intersection` between `RegionPlan` polygons and per-layer modifier projection, stamping `RegionPlan.config["fuzzy_skin.apply-to-all"]=true` on overlapping regions only.
  - `negative_part`: new host stage (per Activation Blocker Q3 lock = Option 1) running between prepass and region-mapping, performing per-layer 2D `slicer_core::polygon_ops::difference` against the parent's slice polygons.
  - `support_enforcer` / `support_blocker`: piggyback on the paint-semantic system from Packet 51 — project the modifier volume per layer and emit synthetic `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`.
- **TASK-193** — TDD coverage: sidecar parser unit tests, fixture-backed E2E (`benchy_4color.3mf`), synthetic-fixture E2E for the three secondary subtypes, no-regression validation for `benchy_painted.3mf`.

## In Scope

- Files-in-scope (write):
  - `crates/slicer-host/src/model_loader.rs` (primary; sidecar parser + `resolve_object` branching + schema bump).
  - `crates/slicer-host/src/model_loader_sidecar.rs` (new file; optional split if `model_loader.rs` exceeds 800 lines after the addition).
  - `crates/slicer-host/src/region_mapping.rs` (region-overlap config stamp for `modifier_part`).
  - `crates/slicer-host/src/pipeline.rs` (insert new negative-part-subtract stage call; forward `modifier_volumes` to region-mapping).
  - `crates/slicer-host/src/paint_segmentation.rs` (or sibling helper; synthetic-`PaintRegionIR` emission for `support_enforcer`/`blocker`).
  - New host stage file (Step 7) implementing `apply_negative_part_subtract`. Recommended path: `crates/slicer-host/src/negative_part_subtract.rs`.
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` (new).
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` (new).
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` (new).
  - `docs/02_ir_schemas.md` (additive: IR 0 `MeshIR` schema header + `ModifierVolume` `config_delta` typed-key documentation).
  - `docs/07_implementation_status.md` (append TASK-190..193).
  - `docs/DEVIATION_LOG.md` (register DEV-047/048/049).
  - `docs/14_deviation_audit_history.md` (chronology entries).

## Out of Scope

- `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs` — confirmed clean by Step 0 sub-agent. Any escalation discovered during implementation triggers DEV-043 / DEV-046-style scope escalation (user-authorized; rare).
- `crates/slicer-macros/src/lib.rs` (>2300 lines; explicit ban — delegate any read).
- `crates/slicer-sdk/` — no trait, `ConfigView`, or builder change. The `fuzzy-skin` module reads `apply-to-all` from its existing `ConfigView` API.
- `modules/core-modules/fuzzy-skin/src/lib.rs` — unchanged. The region-stamped `apply-to-all` config key is sufficient per the module's existing branching at lines 80-81 (read-only verified at Step 0).
- Bambu Studio printer-config block parsing (`Metadata/project_settings.config`).
- STL+sidecar JSON ingestion.
- Sidecar `<part>/<metadata key="matrix">` consumption as a geometry source. Captured into `config_delta` for sanity-check telemetry only; placement remains driven by the model XML's `<component>` transform.
- Sidecar `<assemble>` and `<plate>` sections.
- The `extruder="N"` per-modifier override (captured but no consumer in this packet; future work).
- Subdivision triangle selectors (deferred per `docs/02_ir_schemas.md:122-124`).

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 `MeshIR` (lines 62-244), the versioning rule at line 5, and `ConfigDelta`/`ModifierVolume` shape (lines 192-211 in the doc — read directly; small).
- `docs/01_system_architecture.md` — RegionMapping responsibility and pipeline ordering. Delegate SUMMARY if > 300 lines on a fresh read.
- `docs/04_host_scheduler.md` — prepass / region-mapping ordering. Delegate the section read.
- `docs/08_coordinate_system.md` — scaled integer units (1 unit = 100 nm). Read directly (small).
- `docs/07_implementation_status.md` — append TASK-190..193.
- `docs/DEVIATION_LOG.md` — register DEV-047/048/049.
- `docs/14_deviation_audit_history.md` — chronology entries.

## OrcaSlicer Reference Obligations

The host implementation MUST be project-internal Rust. Cite OrcaSlicer function names only; do not paste source.

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — sidecar parser, `<part subtype>` branching, negative-part per-layer subtract, support enforcer/blocker geometry. Delegate three Explore agent dispatches at Step 0 / Step 1 / Step 7 / Step 8 with the LOCATIONS return-format contract.
- `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp` (general 3MF format) — only if Step 1's sidecar parser hits format ambiguity.

## Acceptance Summary (measurable outcomes)

- `MeshIR.objects[0].mesh.indices.len() / 3 == 225_240` for `resources/benchy_4color.3mf` (was 225,252; the 12 cube triangles are excluded from solid geometry).
- `MeshIR.objects[0].modifier_volumes.len() == 1` for the same fixture, with the entry's `config_delta` carrying typed `subtype = "modifier_part"` and `fuzzy_skin = "external"` keys.
- `MeshIR.schema_version == SemVer { major: 1, minor: 1, patch: 0 }`.
- Fuzzy-skin G-code markers appear inside the cube's XY+Z projection band AND on `paint_fuzzy_skin`-painted facets; markers do NOT appear on other regions of the body (verified by Z-band counting against `count_perimeter_markers_in_z_band` in `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs`).
- Synthetic 3MF with `subtype="negative_part"` reduces per-layer slice polygon area at every Z in the negative volume's extent.
- Synthetic 3MF with `subtype="support_enforcer"` emits `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` at every overlapping layer; `support_blocker` likewise.
- `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-packet output (Packet 50 / 51 regression tests stay green).
- Malformed sidecar XML produces a `log::warn!` and a fallback load (NOT an error). Unknown subtype values downgrade to `normal_part` with a `log::warn!`.
- Paint data on a non-`normal_part` row is dropped at load time with a `log::warn!`.
- `cargo clippy --workspace -- -D warnings` clean.

## Negative Cases (explicit)

- Malformed `Metadata/model_settings.config` (truncated XML) → fallback to all-`normal_part` + structured warning. Loader returns `Ok(MeshIR)`.
- Unknown `subtype` attribute value → downgrade to `normal_part` + structured warning.
- Paint data on a `modifier_part` / `negative_part` / `support_*` row → dropped at load time + structured warning naming the part id.
- Missing sidecar (no `Metadata/model_settings.config` entry in the ZIP archive) → silent default to all-`normal_part`; no warning, no error. This branch must be byte-identical to the pre-packet path for `resources/benchy_painted.3mf`.

## Cross-Packet Dependencies / Unblockers

- Depends on Packet 51's `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` overlay and the `paint_semantic_namespace_key` resolver in `crates/slicer-host/src/config_resolution.rs`. The `support_enforcer` / `support_blocker` paths emit `PaintRegionIR` entries that flow through that overlay; no new resolver code is added by this packet.
- Depends on `slicer_core::polygon_ops::intersection` and `slicer_core::polygon_ops::difference` (Clipper2-backed). Both are public exports.
- Unblocks any future packet that wires per-modifier `extruder` overrides, sidecar-driven `support_critical_regions_only`, or any other per-part metadata consumer.
- Does NOT affect ralph `swarm` orchestration files. The implementer of this packet runs the standard preflight + `swarm` cycle.

## Verification Commands

```powershell
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p slicer-host --test threemf_sidecar_classification_tdd
cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd
cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd
cargo test -p slicer-host --test threemf_transform_tdd
cargo test -p slicer-host --test gcode_emit_tdd
cargo test -p slicer-host --test benchy_painted_e2e_tdd
cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd
```

Per CLAUDE.md Test Discipline: `cargo test --workspace` is NOT a per-criterion or per-step verification command. It is reserved for the packet's acceptance ceremony at closure and must be dispatched to a worker as `FACT pass/fail`.
