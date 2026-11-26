# 46 — Author packet P39 — Multimaterial / Filament for Features — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8b399bfcffeFkNSKiomwD9CdG) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P39 — Multimaterial / Filament for Features — emitter** — 3 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P39 — Multimaterial / Filament for Features — emitter):

`solid_infill_filament`, `sparse_infill_filament`, `wall_filament`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet** (the ticket-26/27/30/34 shape, under the "Packets are for complex implementation only" rule). Human-grilled scope rulings (Q1–Q5, 2026-09-06): adopt canonical `_id` names, wire the full live family of 6, implement directly, paint wins, global only.

**Claim-time re-derivation changed everything except the tier.** The 3 queued names are *legacy aliases* in the oracle: `PrintConfig.cpp` `handle_legacy` rewrites `sparse_infill_filament` → `sparse_infill_filament_id` (likewise `solid_infill_filament` → `internal_solid_filament_id`, `wall_filament` → `outer_wall_filament_id`) with a 1→0 value remap, because the live scheme is coInt 0 = Default/inherit. The gap source snapshot still lists the 3 old names with 1-based defaults, and the other 3 live family members (`top_surface_filament_id`, `bottom_surface_filament_id`, `inner_wall_filament_id`) appear nowhere in the source, the queue, or the tree. The tier table's `crates/slicer-gcode` owner is wrong (ticket-27 hazard): the emitter only emits the `tool_index` the runtime resolves — the decision point is `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`), which already resolves a per-entity tool below all paint-derived sources.

**What landed** (all 6 keys live, default path byte-identical):
- `FeatureFilamentSelection` (new `Copy` struct beside `SupportToolSelection`, which gains it as a field — zero signature changes across the threaded call chain): six `Option<u32>` fields, `None` = inherit.
- `parse_feature_filament_selection` (`crates/slicer-runtime/src/run.rs`, beside the support parser): 1-based N → tool N−1 clamped to the configured tool count (canonical `clamp_feature_filament_to_valid`; deliberate asymmetry with the support selectors, which reject out-of-range to tool 0); 0/absent/negative/non-`Int` → `None`.
- `feature_tool_for_role` ports canonical `LayerTools::extruder` (`GCode/ToolOrdering.cpp`): top-surface + ironing → top, bottom-surface → bottom, other solid → internal, non-solid infill → sparse, inner → inner, everything else perimeter-side → outer. The port's roles are per path, so the collection-level else-branches land per role: `ThinWall`/`GapFill` → outer, bridge roles → sparse. Wired below paint/variant/spatial/modifier (walls) and authored/variant/spatial (infill), above `DEFAULT_TOOL`; support/skirt/brim/raft/tower/custom roles untouched. Runtime-only like the support selectors: no manifest, no `ResolvedConfig` field, no CONFIG_BLOCK work (the stale `("solid_infill_filament", "0")` padding twin is left for ticket 132, which owns padding).
- 9 new tests: 3 parser (1-based rebase of all six, zero/invalid→inherit, clamp incl. `i64::MAX`), 3 entity-resolution (per-role routing of all six with a `Custom`-role passthrough pin, default identity, paint-wins). Gates: `slicer-runtime` lib 107/107, contract `authored_tool_index` 4/4, integration `structured_support_identity` 1/1, clippy clean, check-literals clean.

**Records:** 04 rows annotated (legacy → `_id` adoptions; 3 siblings appended Tier B — queue **407 → 410**), 05 P39 → 6 keys (packet keys 357 → 360), map Notes scoped-target line updated. No deviation rows (canonical defaults are 0; the port defaults to inherit). No packet number taken.

**Known limitations (named, not fixed):** legacy spellings arriving in an Orca 3MF/preset land in the raw map unused — the loader has no legacy-remap seam (canonical translates on load); a ~10-line remap is future micro-work. Out-of-range clamping follows canonical rather than the support precedent's reject-to-default. `ThinWall`/`GapFill` → outer and bridges → sparse are branch-analogy rulings (canonical branches on collection role), pinned by test.
