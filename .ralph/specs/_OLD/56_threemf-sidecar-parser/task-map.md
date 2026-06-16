# Task Map: 56_threemf-sidecar-parser

## Purpose

This packet introduces one new TASK ID (TASK-190) not present in `docs/07_implementation_status.md` at packet-author time. Step 6 of `implementation-plan.md` appends it as a new row after the current high-water mark. This file maps the new TASK to the implementation steps that satisfy it, the deviations it touches, and the OrcaSlicer reference applicable to its scope.

This packet is the first of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packets 56b and 56c (registered separately) will introduce TASK-191, TASK-192, and TASK-193 as their own rows. Each child packet's `task-map.md` enumerates only the TASK IDs it owns.

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-190 | Parse `Metadata/model_settings.config` sidecar; classify `<part subtype>`; surface typed per-part metadata; plumb the producer into `load_3mf` and thread an unused argument through `parse_3mf_model_xml` → `resolve_object`. | Step 1 (TDD-RED + stub), Step 2 (parser implementation), Step 3 (`load_3mf` plumbing + signature widen), Step 4 (regression sweep), Step 5 (clippy), Step 6 (doc/dev registration), Step 7 (acceptance ceremony). | DEV-050 (unknown subtype downgrade); DEV-051 (missing/malformed sidecar fallback). | `docs/02_ir_schemas.md` lines 192-211 (informational); `docs/07_implementation_status.md`; `docs/DEVIATION_LOG.md`. | `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — sidecar parser function name (one LOCATIONS dispatch at Step 1). |

## Deviation Map

| Deviation ID (recommended) | Title | Registered by step | Closed by step | Owner packet |
|---|---|---|---|---|
| DEV-050 | Partial subtype coverage; unknown subtypes downgrade to `NormalPart` with `log::warn!`. | Step 6 | Step 6 (registered as Closed by Packet 56). | This packet (56). |
| DEV-051 | Missing or malformed `Metadata/model_settings.config` is non-fatal; loader returns empty map (missing) or empty map + warning (malformed). | Step 6 | Step 6 (registered as Closed by Packet 56). | This packet (56). |
| DEV-052 (Packet 56b's deviation; actual slot TBD at Packet 56b activation) | Paint data on non-`normal_part` rows dropped at load time with `log::warn!`. | Step 6 of Packet 56b | Step 6 of Packet 56b. | Packet 56b. NOT this packet — `resolve_object` is not branched here. |

Recommended numbering verified at Step 6 via FACT dispatch ("highest existing DEV-### in `docs/DEVIATION_LOG.md`"); bump if 047/048/049 are claimed by another in-flight packet.

## OrcaSlicer Reference Schedule

| Step | Question | Return format |
|---|---|---|
| Step 1 | "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`." | LOCATIONS, ≤ 8 entries. |

All OrcaSlicer reads are delegate-only. Function names are cited in this packet's `requirements.md` and `design.md`; no source snippets are pasted. Packets 56b and 56c carry their own OrcaSlicer dispatches for their respective scope (modifier_part fuzzy overlay, negative-part subtract, support enforcer/blocker geometry).

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56b | This packet unblocks | Packet 56b consumes `parse_3mf_sidecar`'s output in `resolve_object` branching. Packet 56b cannot start until Packet 56 closes. |
| Packet 56c | This packet unblocks | Transitively, via Packet 56b. |
| Packets 50, 50b, 51 | None | The parser does not depend on paint ingestion or paint-semantic config overlays. |

## Notes for Implementer

- This packet does not modify any prior packet's `.ralph/specs/` directory. The in-place refinement of `56_threemf-modifier-and-subtype-sidecar-ingestion` was performed by overwriting that draft's own files (the directory was renamed to `56_threemf-sidecar-parser` via `git mv`).
- `cargo test --workspace` is NOT run at closure of this packet. The parser is producer-only and threaded but unused; the targeted regression suites in Step 4 cover the full behavioral surface.
- The `_sidecar` parameter added to `resolve_object` in Step 3 is intentionally unused. Packet 56b removes the underscore when it branches on the value.
