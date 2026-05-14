---
status: draft
packet: 56_threemf-sidecar-parser
task_ids:
  - TASK-190
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
supersedes: none
absorbed_predecessors:
  - 56_threemf-modifier-and-subtype-sidecar-ingestion (in-place refinement; never closed)
split_children:
  - 56b_threemf-modifier-part-ir-routing
  - 56c_threemf-negative-and-support-subtype-routing
---

# Packet Contract: 56_threemf-sidecar-parser

> This packet is the first of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet (status: draft, never activated). The original packet's L-aggregate context cost was the explicit activation blocker (Activation Q1). The split decision: 56 owns the sidecar parser only; 56b owns `resolve_object` branching, schema bump, and `modifier_part` region-mapping overlap; 56c owns `negative_part` host stage and `support_enforcer`/`support_blocker` paint-segmentation piggyback. Each child packet's aggregate cost is **M**.

## Goal

Add a host-internal parser for the OrcaSlicer / Bambu Studio sidecar `Metadata/model_settings.config` to the 3MF loader. Surface typed per-part metadata (`subtype`, `fuzzy_skin`, optional `extruder`, optional `matrix` for telemetry) keyed by `(<object id>, <part id>)`. No IR mutation. No `resolve_object` branching. No downstream consumer wiring. The parser is a pure data producer; its output is consumed by Packet 56b (`resolve_object` branching into `ObjectMesh.modifier_volumes`) and Packet 56c (downstream subtype-specific consumers).

This packet closes two deviations driven by parser-level behavior:

- **DEV-047** — Partial subtype coverage. The parser enumerates `normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`. Unknown subtype values silently downgrade to `normal_part` with a `log::warn!` naming the unrecognized string.
- **DEV-049** — Missing or malformed `Metadata/model_settings.config` is non-fatal. Missing entry → returns an empty map silently (no warning; absence is the default). Malformed XML (truncated, unclosed elements) → returns an empty map AND emits a `log::warn!` containing the substring "treating all parts as normal_part".

(`DEV-048` — paint dropped on non-`normal_part` rows — is registered and closed by Packet 56b, which is the packet that actually branches `resolve_object` to perform the drop. Packet 56 cannot close DEV-048 because it does not modify `resolve_object`.)

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/model_loader.rs` — add a `parse_3mf_sidecar(zip: &mut ZipArchive<...>) -> HashMap<u32, ObjectSidecarInfo>` helper. Add the host-local `PartSubtype` enum and the carrier structs `ObjectSidecarInfo` / `PartSidecarInfo`. Wire the call from `load_3mf` (currently `crates/slicer-host/src/model_loader.rs:555-587`) to read the sidecar from the same `zip::ZipArchive` already opened — BEFORE the archive is dropped. Thread the resulting map into the existing `parse_3mf_model_xml` -> `resolve_object` call chain as an additional argument that is currently **unused** by `resolve_object` (Packet 56b is where `resolve_object` actually branches on it). Per skill rule: this is producer-side only; the data flows but no consumer reads it yet.
  - `crates/slicer-host/src/model_loader_sidecar.rs` — NEW sibling file allowed if `model_loader.rs` exceeds 800 lines after the addition (Packet-author estimate: parser is ~150 lines; sibling file is the conservative split).
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — NEW. Unit tests covering: (a) well-formed sidecar from `resources/benchy_4color.3mf`; (b) malformed XML fallback; (c) unknown subtype downgrade; (d) missing sidecar silent default; (e) `fuzzy_skin = "external"` parsed as `ConfigValue::String`; (f) `<part id>`-to-`<object id>` mapping per Bambu's sidecar convention.
  - `docs/07_implementation_status.md` — append the TASK-190 row naming this packet.
  - `docs/DEVIATION_LOG.md` — register DEV-047 and DEV-049 as `Closed — Packet 56, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — chronology entries for the two new DEV rows.

- Out of scope:
  - `resolve_object` branching, geometry routing into `ObjectMesh.modifier_volumes`, paint-data drop on non-`normal_part` rows, and the `MeshIR.schema_version` bump. → Packet 56b.
  - The new `apply_negative_part_subtract` host stage and `support_enforcer`/`support_blocker` paint-segmentation piggyback. → Packet 56c.
  - Any change to `crates/slicer-ir/`, `crates/slicer-sdk/`, `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, or `crates/slicer-host/src/dispatch.rs`. The parser produces a host-local data structure that is not visible at any contract boundary in this packet.
  - Any change to the `fuzzy-skin` module manifest (`modules/core-modules/fuzzy-skin/manifest.toml`). → Packet 56b confirms `apply-to-all` is declared.
  - Bambu's `Metadata/project_settings.config` (printer profile sidecar; different file, different schema).
  - Consuming `<part>/<metadata key="matrix">` as a geometry source — the model XML's `<component>` transform remains the source of truth. The sidecar matrix is captured into the per-part metadata map as telemetry only.
  - `<assemble>` and `<plate>` sidecar sections — informational; not consumed.

## Prerequisites and Blockers

- Depends on:
  - Nothing new. This packet does not depend on any prior packet's IR or contract.
- Unblocks:
  - Packet 56b (which consumes the parser output in `resolve_object`).
  - Packet 56c (transitively via 56b).

## Acceptance Criteria

- **Given** `resources/benchy_4color.3mf` exists and its `Metadata/model_settings.config` exists with `<part id="2" subtype="modifier_part">` and `<metadata key="fuzzy_skin" value="external"/>`, **when** `parse_3mf_sidecar` is invoked on the opened `ZipArchive`, **then** the returned `HashMap<u32, ObjectSidecarInfo>` contains exactly one `ObjectSidecarInfo` with `parts.len() >= 1`, AND the part keyed `2` has `subtype == PartSubtype::ModifierPart`, AND its `metadata.get("fuzzy_skin") == Some(&"external".to_string())`. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd parses_benchy_4color_sidecar -- --exact --nocapture`
- **Given** a 3MF archive with no `Metadata/model_settings.config` entry (e.g., `resources/benchy_painted.3mf`), **when** `parse_3mf_sidecar` is invoked, **then** the returned map is empty (`HashMap::is_empty() == true`), AND no `log::warn!` is emitted (absence is the silent default). | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd missing_sidecar_is_silent_default -- --exact --nocapture`
- **Given** a synthetic 3MF archive whose `Metadata/model_settings.config` contains XML truncated mid-`<part>` (no closing `</part>`, no closing `</object>`), **when** `parse_3mf_sidecar` is invoked, **then** the returned map is empty AND a `log::warn!` is emitted with target `slicer_host::model_loader::sidecar` (or equivalent verified target name) containing the substring `"treating all parts as normal_part"`. The function returns `HashMap::new()` — NOT `Err(_)`. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd malformed_sidecar_falls_back_to_normal_part -- --exact --nocapture`
- **Given** a synthetic 3MF archive whose sidecar `<part>` carries `subtype="unrecognized_subtype_value"`, **when** `parse_3mf_sidecar` is invoked, **then** that part's classification is `PartSubtype::NormalPart` AND a `log::warn!` is emitted with a message containing both the substring `"unrecognized_subtype_value"` and the substring `"downgrading to normal_part"`. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd unknown_subtype_downgrades_to_normal_part -- --exact --nocapture`
- **Given** the Bambu sidecar convention where `<object id="3"><part id="2" subtype="modifier_part">` means "part 2 inside object 3", **when** `parse_3mf_sidecar` is invoked on a fixture with that exact structure, **then** the returned map's outer key is `3` AND `parts.get(&2)` returns `Some(&PartSidecarInfo { subtype: PartSubtype::ModifierPart, .. })`. The `<part id>` value is the inner key and refers to the part within its parent object. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd object_and_part_id_mapping_matches_bambu_convention -- --exact --nocapture`
- **Given** `load_3mf` is invoked on `resources/benchy_4color.3mf` after this packet lands, **when** the call returns, **then** `parse_3mf_sidecar` has been called exactly once with the same `ZipArchive` (assert via a side-effect counter or `log::trace!` line containing `"parse_3mf_sidecar: 1 object(s), N part(s)"` — the exact format is locked at Step 2). The archive is read BEFORE being dropped. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd load_3mf_invokes_sidecar_parser_before_archive_drop -- --exact --nocapture`
- **Given** `cargo clippy` is the lint gate, **when** Step 5 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`
- **Given** the existing regression suites must stay GREEN with the parser plumbed in (no consumer changes), **when** Step 4 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` reports all-pass AND `cargo test -p slicer-host --test gcode_emit_tdd` reports all-pass AND `cargo test -p slicer-host --test benchy_painted_e2e_tdd` reports all-pass AND `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` reports all-pass. (The parser is plumbed but its output is unused; behavior must be unchanged.) | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`
- **Given** TASK-190 is registered by this packet, **when** Step 6 runs, **then** `docs/07_implementation_status.md` contains a row matching `[x] TASK-190` AND naming this packet (`56_threemf-sidecar-parser`). | `rg -q '\[x\] TASK-190.*56_threemf-sidecar-parser' docs/07_implementation_status.md`
- **Given** DEV-047 and DEV-049 are registered and closed by this packet, **when** Step 6 runs, **then** `docs/DEVIATION_LOG.md` contains two rows whose ID column matches `DEV-047` and `DEV-049` AND whose status column reads `Closed — Packet 56, 2026-MM-DD` (date filled at close time). DEV-048 must NOT appear as closed by this packet (it is closed by Packet 56b). | `rg -c '^\| DEV-04[79].*Closed.*Packet 56[^b]' docs/DEVIATION_LOG.md` (expected: 2) `&& ! rg -q '^\| DEV-048.*Closed.*Packet 56[^b]' docs/DEVIATION_LOG.md`

## Negative Test Cases

- Malformed sidecar XML → empty map + warning (AC-3 above).
- Unknown `subtype` value → downgrade to `NormalPart` + warning (AC-4 above).
- Missing sidecar → empty map + silent default (AC-2 above).
- Sidecar present but contains zero `<part>` elements (only `<object>` wrappers) → returned `ObjectSidecarInfo.parts.is_empty()` AND no warning. Verified by:
  - **Given** a synthetic sidecar with exactly one `<object id="1">` element and no `<part>` children, **when** `parse_3mf_sidecar` is invoked, **then** the returned map contains `{1 => ObjectSidecarInfo { parts: HashMap::new() }}` AND no `log::warn!` is emitted. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd empty_object_in_sidecar_returns_empty_parts -- --exact --nocapture`

## Verification

- `cargo check --workspace` — compile health.
- `cargo clippy -p slicer-host --tests -- -D warnings` — lint gate (per-crate, sufficient for this packet).
- `cargo clippy --workspace -- -D warnings` — lint gate (workspace).
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — parser unit suite (all ACs + negative cases).
- `cargo test -p slicer-host --test threemf_transform_tdd` — transform regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — G-code emission regression.
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — no-sidecar E2E regression.
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — paint-semantic regression.

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 `MeshIR` (lines 62-244) and `ConfigDelta`/`ModifierVolume` shape (lines 192-211 in the doc). Read directly (small section); informational only — no IR change in this packet.
- `docs/08_coordinate_system.md` — scaled integer units (1 unit = 100 nm). Read directly (small). Informational; the parser does not perform any geometry.
- `docs/07_implementation_status.md` — append TASK-190 row.
- `docs/DEVIATION_LOG.md` — register DEV-047 and DEV-049.
- `docs/14_deviation_audit_history.md` — chronology entries.

## OrcaSlicer Reference Obligations

The host implementation MUST be project-internal Rust. Cite OrcaSlicer function names only; do not paste source.

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — production sidecar parser. Delegate ONE Explore agent dispatch at Step 1 with the LOCATIONS contract:
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`. Return LOCATIONS with one-line role each; ≤ 8 entries. No source pasted."
  - Cite the returned function names in `requirements.md` and `design.md`.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

Aggregate cost is **M** (4M + 2S step distribution). Each implementation step picks ≤ 3 files and a worker dispatch covers each step in isolation. Downstream agents:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor the out-of-bounds list — `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, OrcaSlicer source — they must not be loaded directly;
- delegate every `cargo` run via a sub-agent FACT contract;
- stop reading at 60% context and hand off at 85%.
