# Task Map: 72_wit-single-source-unification

This packet spans two task IDs and **completes work `docs/07` already marks `[x]`** — TASK-144/TASK-145 describe consolidating codegen onto one canonical WIT source, but the current tree still carries three copies (phantom `wit/`, inline macro literals, inline host `bindgen!`). The mapping below records which step is sufficient evidence for each task.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-144` (consolidate host/macro/guest codegen onto one canonical WIT source) | Step 1, Step 2, Step 3, Step 4 | `docs/03_wit_and_manifest.md` | `crates/slicer-schema/wit/**`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-runtime/src/wit_host.rs`, `xtask/src/build_guests.rs` | none | M | Sufficient when both consumers read the single canonical dir (AC-2, AC-3), the phantom is deleted (AC-1), and the ABI is unchanged (AC-7/AC-8). |
| `TASK-145` (normalize WIT package/version identifiers; restore missing members) | Step 1, Step 4, Step 5 | `docs/03_wit_and_manifest.md` | `crates/slicer-schema/wit/**` (legal `extrusion-path3d`, shared `module-error`, dropped orphan), `crates/slicer-runtime/src/wit_host.rs` | none | M | Sufficient when AC-4 (legal label), AC-5 (one `module-error`), AC-6 (no orphan), and AC-9/AC-N1 (canonical dir parses; illegal labels rejected) pass. |

Aggregate context cost across rows: `M`. No cell is `L`. Both task IDs are re-touched (not net-new); `requirements.md` §Problem Statement records why the prior `[x]` was premature.
