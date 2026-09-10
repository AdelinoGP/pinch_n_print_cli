# 87 — Author packet P80 — Quality / Walls and surfaces — print-orchestration

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P80) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P80 — Quality / Walls and surfaces — print-orchestration** — 1 keys, Tier B new logic, owner print-orchestration. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P80 — Quality / Walls and surfaces — print-orchestration):

`extruder`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**P80 dissolved; its single key folded into ticket 124, no packet, no code change** (the ticket-32 note's instruction shape, confirmed — no "why not").

Claim-time grounding, verified against the oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) and re-derived from this tree:

- **The key passes rule 3** (live in canonical's slicing pipeline, stays in scope). `PrintConfig.cpp` declares `extruder` as `coInt` with no default-value line (`0 = inherit defaults`); its live slicing reads are `apply_to_print_region_config` (`PrintObject.cpp`, the object/volume `extruder` → six `*_filament_id` fan-out with the explicit-overrides mask) plus `object_config_from_model_object`'s `normalize_fdm` normalisation, `auto_assign_extruders` (`Print.cpp`, multi-volume objects), the object-index bookkeeping in `Model.cpp` / `bbs_3mf.cpp` / `3mf.cpp`, the layer-range/ProjectSettings `has("extruder")` arms, and the shared-object identity predicate. Zero occurrences as `extruder` behaviour under this tree's `crates/` + `modules/` + `xtask/` + `resources/` (the tree hits are all `extruder_*` siblings, `extruder_colour`, or prose).
- **A standalone P80 packet cannot close** (rule 1: declaration-only is prohibited). Canonical's `extruder` never drives emission itself — it assigns objects/volumes to tools and normalises onto the six `*_filament_id` selectors, which this port already resolves per entity at runtime (`FeatureFilamentSelection`, `parse_feature_filament_selection` (`crates/slicer-runtime/src/run.rs`), `feature_tool_for_role` porting `LayerTools::extruder` (`GCode/ToolOrdering.cpp`), wired below paint/variant/spatial/modifier in `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`, called with the run's `SupportToolSelection` from `execute_single_layer_inner`'s fallback assembly arm), ticket 46). What is missing here is everything those selectors *select over*: this port has no per-object tool axis in the prepass IR (the tool is known only from a `("material", ToolIndex(n))` paint variant chain, `tool_config:<idx>:` overlays, or 3MF-sidecar `extruder` metadata rebased into `ObjectMesh.config.data` and lifted via `object_config:<id>:extruder` into `ResolvedConfig.extensions`), which is exactly ticket 125's per-tool model ruling (itself gated on 126, already landed). Declaring `extruder` on a module or in `ResolvedConfig` alone would wire a key whose *subject* (per-object tool selection composed onto every assigned selector) does not exist — the ticket-28/39/45 shape.
- **Ticket 124 is the carrying ticket, not 125.** 125 rules the axis; the feature that must *consume* a per-object `extruder` assignment — multi-object sequential gating, per-object tool identity surviving into region resolution — is exactly what 124's sequential-printing feature owns (`print_sequence`, `nozzle_height`, the three clearance keys; its Question already lists the per-object seam as an open authoring decision). Folding the key there keeps the one feature's keys in one place. The fold is into 124's *carried-keys* list; the first act remains 124's human scope ruling (validation-only vs emission), which now also decides whether a standalone `extruder` assignment lands with validation or waits for the tool axis.
- **Named non-borrows (not queue work):** the `TimelapsePosPicker`-style traditional park reads (ticket-86 finding — no picker seam here), the `is_print_object_the_same` shared-object cache predicate (this port has no print-object cache; `Print.cpp`), `auto_assign_extruders` multi-volume defaulting (no per-volume config list in `ObjectMesh`), and the GUI-side `Plater.cpp` / `Tab.cpp` arms. None is folded.
- Owner note (ticket-27 hazard): the tier table's `print/orchestration` owner is reviewed against canonical, not against this tree — this tree has no print-orchestration seam; tool resolution lives in runtime entity assembly, which 124's authoring must confirm from code, not assume.

No packet number taken, no deviation rows, no `ORCA_CONFIG_PADDING` edit. No code change; no queue-count change. 04/05 rows annotate the fold.
