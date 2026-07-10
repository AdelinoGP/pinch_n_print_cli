---
status: implemented
packet: 56_threemf-sidecar-parser
task_ids:
  - TASK-190
supersedes: none
---

# 56_threemf-sidecar-parser

## Goal

Add a host-internal parser for the OrcaSlicer / Bambu Studio sidecar `Metadata/model_settings.config` to the 3MF loader. Surface typed per-part metadata (`subtype`, `fuzzy_skin`, optional `extruder`, optional `matrix` for telemetry) keyed by `(<object id>, <part id>)`. No IR mutation. No `resolve_object` branching. No downstream consumer wiring. The parser is a pure data producer; its output is consumed by Packet 56b (`resolve_object` branching into `ObjectMesh.modifier_volumes`) and Packet 56c (downstream subtype-specific consumers).

This packet closes two deviations driven by parser-level behavior:

- **DEV-050** — Partial subtype coverage. The parser enumerates `normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`. Unknown subtype values silently downgrade to `normal_part` with a `log::warn!` naming the unrecognized string.
- **DEV-051** — Missing or malformed `Metadata/model_settings.config` is non-fatal. Missing entry → returns an empty map silently (no warning; absence is the default). Malformed XML (truncated, unclosed elements) → returns an empty map AND emits a `log::warn!` containing the substring "treating all parts as normal_part".

(Note: The paint-drop-on-non-`normal_part`-rows deviation — originally planned as DEV-048 — will be registered by Packet 56b under its own free DEV ID, as DEV-048 and DEV-049 were claimed by packet 53. Packet 56 cannot close that deviation because it does not modify `resolve_object`.)

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

## Architecture Constraints

- WIT boundary: clean. This packet does not even touch IR types; it only adds host-internal `PartSubtype` enum + carrier structs.
- IR versioning: no change. `MeshIR.schema_version` stays at 1.0.0; the bump to 1.1.0 happens in Packet 56b.
- Logging: use the project's existing `log` facade (`log::warn!`, `log::trace!`). Pick the log target carefully — Step 2's FACT dispatch confirms the exact target string used by other 3MF-loader log lines so the AC's `"slicer_host::model_loader::sidecar"` literal matches reality (or the AC is updated to match the existing convention).

## Data and Contract Notes

- `parse_3mf_sidecar` returns `HashMap<u32, ObjectSidecarInfo>`. Caller must ensure the `ZipArchive` is still open at call time.
- `ObjectSidecarInfo { parts: HashMap<u32, PartSidecarInfo> }`. The outer key is `<object id>` from `<object id="N">` in the sidecar; the inner key is `<part id>` from `<part id="M" subtype="...">`.
- `PartSidecarInfo { subtype: PartSubtype, metadata: BTreeMap<String, String> }`. `metadata` is the verbatim key/value map from each `<metadata key="..." value="..."/>` row inside the `<part>`. Reserved keys: `fuzzy_skin`, `extruder`, `matrix`.
- `PartSubtype` enum is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`.
- Logging: pick the existing log target convention (verified by Step 2 FACT dispatch: "Return the log target string used in the 5 most recent `log::warn!` / `log::trace!` calls in `crates/slicer-host/src/model_loader.rs`. SNIPPETS, ≤ 5 lines."). Default placeholder for ACs: `slicer_host::model_loader::sidecar`.
- No determinism concern in this packet — the parser produces stable output for stable input by construction (no UUID, no timestamp).

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| `parse_3mf_sidecar` introduces a new `quick_xml::Reader` parse loop; if it diverges from `parse_3mf_model_xml`'s loop pattern, future maintenance burden. | Step 2 FACT dispatch returns the existing `quick_xml::Reader` pattern verbatim (≤ 30 lines). Implement using that pattern. |
| `resolve_object` signature widen breaks any call site that passes a different argument list. | Step 3 FACT dispatch enumerates every call site of `resolve_object`. Expected count: 1 (from `parse_3mf_model_xml`). Update that call site. |
| `_sidecar` underscore parameter triggers `unused_variables` lint despite the underscore prefix in some clippy configurations. | Step 4 verifies clippy is clean. If the underscore alone doesn't satisfy clippy, add `#[allow(unused_variables)]` on the parameter — and remove the allow in Packet 56b when the value is consumed. |
| Sidecar `<part id>` and `<object id>` could collide if a fixture violates Bambu's convention. | AC-5 asserts the convention against a synthetic fixture. If a real-world fixture violates it, register an additional DEV at Step 6 and adapt; do not block this packet. |
| Step 2 worker accidentally absorbs the full sidecar XML (~60 lines) into its context. | Step 2 dispatch uses SNIPPETS return-format with explicit ≤ 30 line cap. |

## Locked Assumptions and Invariants

1. WIT scope is clean and not re-checked in this packet (no IR types introduced).
2. `parse_3mf_sidecar` returns a `HashMap<u32, ObjectSidecarInfo>` where the outer key is `<object id>` and the inner key is `<part id>` per Bambu's sidecar convention.
3. `PartSubtype` is host-local and not exposed at any IR or WIT boundary.
4. Missing sidecar → empty map, no warning. Malformed sidecar → empty map + warning. Unknown subtype → `NormalPart` + warning.
5. `resolve_object`'s body is unchanged; its signature widens with an underscore-prefixed `_sidecar` parameter.
6. `MeshIR.schema_version` stays at 1.0.0 in this packet. Packet 56b bumps it to 1.1.0.
7. All regression suites stay byte-identical to pre-packet output because no consumer reads the sidecar map.
8. `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-packet output. This is a hard invariant verified by AC-8.
