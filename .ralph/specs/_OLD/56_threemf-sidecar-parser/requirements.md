# Requirements: 56_threemf-sidecar-parser

## Problem Statement

The 3MF loader at `crates/slicer-host/src/model_loader.rs::resolve_object` (lines 430-552) and `find_model_path` (lines 575-587) reads only `3D/3dmodel.model`. It never opens the OrcaSlicer / Bambu Studio sidecar `Metadata/model_settings.config`, which classifies each `<part>` by `subtype=` and carries per-part metadata such as `fuzzy_skin="external"`, `extruder="N"`, and a row-major `<part>/<metadata key="matrix">` transform (informational).

This is the root cause behind the visible Bug B observed against `resources/benchy_4color.3mf` — the modifier cube's `<metadata key="fuzzy_skin" value="external"/>` row is silently dropped, and the consumer (Packet 51's region overlay) has nothing to overlay against. Without a sidecar parser, no downstream packet can route geometry into `ObjectMesh.modifier_volumes` (Packet 56b) or wire subtype-specific consumers (Packet 56c).

This packet is producer-only. It adds the parser, plumbs it into `load_3mf` BEFORE the `ZipArchive` is dropped, and threads the resulting `HashMap<u32, ObjectSidecarInfo>` into the existing `parse_3mf_model_xml` → `resolve_object` call chain. `resolve_object`'s body remains unchanged in this packet — the sidecar argument is currently unused. Packet 56b is where `resolve_object` actually branches on the classification; Packet 56c is where the four non-`normal_part` subtypes get downstream wiring.

This packet is the first of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet (status: draft, never activated). The original packet's L-aggregate context cost was the explicit activation blocker. Splitting reduces each child packet's aggregate to **M**.

WIT scope is **clean** — confirmed in the original packet's Step-0 sub-agent gate. `ObjectMesh.modifier_volumes` and `ModifierVolume` are host-only types. This packet does not even touch IR; the parser produces a host-local `HashMap<u32, ObjectSidecarInfo>` that is not exposed at any contract boundary.

Two deviations are registered and closed by this packet:

- **DEV-050** — Partial subtype coverage. The parser recognizes `normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`. Unknown values silently downgrade to `NormalPart` with a `log::warn!`. Recommended ID DEV-050 (verified by Step 6 against `docs/DEVIATION_LOG.md`).
- **DEV-051** — Missing or malformed `Metadata/model_settings.config` is non-fatal. Missing → empty map, no warning. Malformed → empty map + `log::warn!`.

(DEV-048 — paint dropped on non-`normal_part` rows — is registered and closed by Packet 56b, which is where `resolve_object` branching introduces the drop logic.)

This packet does not absorb any prior packet's directory. The Cross-Packet Mutation Rule does not apply to this packet's writes. The in-place refinement of `56_threemf-modifier-and-subtype-sidecar-ingestion` was performed by overwriting that draft's files (the directory was renamed to `56_threemf-sidecar-parser` via `git mv`); no prior packet's status changes.

## Task IDs (registered by this packet)

- **TASK-190** — Parse 3MF sidecar `Metadata/model_settings.config`; classify `<object>`/`<part>` by `subtype=`; surface typed per-part metadata. Covers DEV-050 (unknown subtype downgrade) and DEV-051 (missing/malformed sidecar fallback). Closed when this packet's verification commands pass.

Packets 56b and 56c will register TASK-191, TASK-192, and TASK-193 as new rows in `docs/07_implementation_status.md`. This packet does not preallocate those rows.

## In Scope

- Files-in-scope (write):
  - `crates/slicer-host/src/model_loader.rs` — host-local types (`PartSubtype`, `ObjectSidecarInfo`, `PartSidecarInfo`), `parse_3mf_sidecar` helper, `load_3mf` plumbing.
  - `crates/slicer-host/src/model_loader_sidecar.rs` — NEW sibling file if `model_loader.rs` exceeds 800 lines after the addition.
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — NEW; parser unit suite.
  - `docs/07_implementation_status.md` — append TASK-190 row naming this packet.
  - `docs/DEVIATION_LOG.md` — register DEV-050 and DEV-051 as `Closed — Packet 56, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — chronology entries.

## Out of Scope

- `resolve_object` branching, geometry routing into `ObjectMesh.modifier_volumes`, paint-data drop on non-`normal_part` rows, `MeshIR.schema_version` bump, fuzzy-skin manifest gate, region-mapping overlap stamp — all owned by Packet 56b.
- `apply_negative_part_subtract` host stage, support enforcer/blocker paint-segmentation piggyback — owned by Packet 56c.
- Any change to `crates/slicer-ir/`, `crates/slicer-sdk/`, `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`. Confirmed clean by predecessor's Step-0 gate; not re-checked because this packet introduces no IR types.
- Bambu Studio printer-config block parsing (`Metadata/project_settings.config`).
- STL+sidecar JSON ingestion.
- Sidecar `<part>/<metadata key="matrix">` consumption as a geometry source. The parser captures the raw 16-float string into `PartSidecarInfo.metadata.get("matrix")` for telemetry; no consumer reads it.
- Sidecar `<assemble>` and `<plate>` sections.
- The `extruder="N"` per-modifier override consumer (parser captures the value; no consumer in this packet OR Packet 56b — future packet).
- Subdivision triangle selectors (deferred per `docs/02_ir_schemas.md:122-124`).

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 `MeshIR` (lines 62-244) and `ConfigDelta`/`ModifierVolume` shape (lines 192-211). Read directly; informational only.
- `docs/08_coordinate_system.md` — informational; parser does no geometry.
- `docs/07_implementation_status.md` — append TASK-190 row.
- `docs/DEVIATION_LOG.md` — register DEV-050 and DEV-051.
- `docs/14_deviation_audit_history.md` — chronology entries.

## OrcaSlicer Reference Obligations

The host implementation MUST be project-internal Rust. Cite OrcaSlicer function names only; do not paste source.

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — sidecar parser entry points. Delegate ONE Explore agent dispatch at Step 1 with LOCATIONS return-format ("Name the function(s) that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`. ≤ 8 entries.").
- `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp` (general 3MF format) — consult only if Step 1's sidecar parser hits format ambiguity around `<object id>` / `<part id>` namespacing.

## Acceptance Summary (measurable outcomes)

- `parse_3mf_sidecar(zip)` returns `HashMap<u32, ObjectSidecarInfo>` with one entry per `<object>` row in the sidecar; each entry's `parts: HashMap<u32, PartSidecarInfo>` carries one `PartSidecarInfo` per `<part>` row.
- `PartSubtype` enum recognizes exactly five values: `NormalPart`, `ModifierPart`, `NegativePart`, `SupportEnforcer`, `SupportBlocker`. Unknown values map to `NormalPart` with `log::warn!`.
- Missing `Metadata/model_settings.config` → empty map; no warning.
- Malformed XML → empty map + `log::warn!` containing "treating all parts as normal_part".
- `load_3mf` invokes `parse_3mf_sidecar` exactly once per archive, BEFORE the `ZipArchive` is dropped. The parsed map is threaded into the `parse_3mf_model_xml` → `resolve_object` call chain but `resolve_object` does not yet branch on it (Packet 56b's scope).
- All existing regression suites (`threemf_transform_tdd`, `gcode_emit_tdd`, `benchy_painted_e2e_tdd`, `benchy_painted_overrides_e2e_tdd`) stay GREEN. Slice output is byte-identical to pre-packet output for all fixtures (consumer is not yet wired).
- `cargo clippy --workspace -- -D warnings` clean.
- `docs/07_implementation_status.md` has a `[x] TASK-190` row naming this packet.
- `docs/DEVIATION_LOG.md` has DEV-050 and DEV-051 rows marked `Closed — Packet 56`.

## Negative Cases (explicit)

- Malformed `Metadata/model_settings.config` (truncated XML) → empty map + `log::warn!` + `Ok(MeshIR)` from the eventual `load_model` call. Loader does NOT return `Err`.
- Unknown `subtype` attribute value → downgrade to `NormalPart` + `log::warn!`.
- Missing sidecar (no `Metadata/model_settings.config` entry in the ZIP archive) → silent default to empty map; no warning, no error.
- Sidecar with zero `<part>` elements → returned `ObjectSidecarInfo.parts.is_empty()`; no warning.

## Cross-Packet Dependencies / Unblockers

- This packet has no dependencies on prior packets' IR or contracts.
- Unblocks Packet 56b's `resolve_object` branching (Packet 56b cannot start until the sidecar map is threaded into `resolve_object`).
- Unblocks Packet 56c transitively via 56b.

## Verification Commands

```powershell
cargo check --workspace
cargo clippy -p slicer-host --tests -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test -p slicer-host --test threemf_sidecar_classification_tdd
cargo test -p slicer-host --test threemf_transform_tdd
cargo test -p slicer-host --test gcode_emit_tdd
cargo test -p slicer-host --test benchy_painted_e2e_tdd
cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd
```

Per CLAUDE.md Test Discipline: `cargo test --workspace` is NOT a per-criterion or per-step verification command. This packet's closure does not require a workspace-wide test run because the parser is producer-only and threaded but unused — every behavioral check is covered by the targeted commands above. If a worker accidentally runs `cargo test --workspace`, it must be dispatched as `FACT pass/fail` and the output must not be absorbed.
