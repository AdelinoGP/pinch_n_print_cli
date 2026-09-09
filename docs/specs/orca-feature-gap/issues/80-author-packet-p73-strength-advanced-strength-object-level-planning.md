# 80 — Author packet P73 — Strength / Advanced (Strength) — object-level planning

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7bcfa92affeQu7s5K9dMFXDSr) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P73 — Strength / Advanced (Strength) — object-level planning** — 4 keys, Tier B new logic, owner object-level planning. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P73 — Strength / Advanced (Strength) — object-level planning):

`ensure_vertical_shell_thickness`, `extra_solid_infills`, `infill_combination`, `infill_combination_max_layer_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owner confirmed but narrowed to the prepass seam, packet authored, no re-file.** All four keys live in canonical's slicing pipeline (`PrintObject.cpp` — every read converges in `discover_vertical_shells`, `discover_horizontal_shells`, `combine_infill`; `check_layer_id_pattern` in `utils.cpp` for the layer pattern) and are zero-occurrence as behaviour in this tree (verified by tree-wide `rg` over `crates/` + `modules/` + `xtask/` — doc-only hits; no `ORCA_CONFIG_PADDING` rows, no manifest declarations, no `ResolvedConfig` fields; no draft packet owns the decision — 234a's `extra_solid_infills` mention is a non-borrow, not ownership). Ticket 04's "object-level planning" owner is accurate but is a seam, not a module: the decision points land in the host prepass `commit_shell_classification_builtin` plus the rectilinear sparse emitter (the ticket-36 precedent), never host special-cases.

Packet `docs/spec_packets/299-object-level-shell-infill-planning/` authored (`draft`), **preflight PASS** (S0–S8, all tree-verified: 13/14 named pre-existing symbols resolved on first sweep; the one FAIL — `ResolvedFloatOrPercent::get_abs_value`, a method the type does not have — corrected to the real `ConfigView::get_abs_value` percent-resolution shape in `crates/slicer-ir/src/slice_ir.rs`; DEV-190 verified absent from the log and all drafts and is exactly one above the spec-packets max 189; no hardcoded SemVer; ADR-0062/0063 conformance confirmed, not amended; `--lib` test homes valid in both crates). Membership 4/4, none shed, none returned: the mode key drives a strict-parsed vertical-shell stage in the prepass (canonical default `ensure_all` — **one intended default output change**, vertical shells newly active at default, AC-2 pins it with `none` as the baseline arm); `extra_solid_infills` drives a `check_layer_id_pattern` port inserting solid layers; the combination pair drives a sparse-grouping stage (cap = `min(percent-cap, nozzle_diameter)`) whose summed height rides a net-new `SlicedRegion.combined_infill_height` field + `SliceRegionView` accessor + one WIT line to the rectilinear sparse arm (walls keep original height, AC-5). Canonical scalarity IS held (all four scalar — no ticket-125 vector arm); no numeric range rejection (ticket-113 rule; the only rejection is strict-parse of the mode enum, AC-N1); locked-path ADR-0062/0063 conformance pinned by AC-N2; CONFIG_BLOCK honest absence (prepass inputs, spellings ride 132; no padding edits, rule 2); rule 4 does not fire (in-stage mode branching, `seam_position` precedent). Three divergences in new **DEV-190**: no `stInternalVoid` tri-typing (void-marking rides sparse-area emptiness + the height field), the `interface_shells` multi-material gate is P76's non-borrow, and `Print.cpp` reslice-invalidation rides ticket 124. Standing approval for writing without interactive review: the wayfinder queue entry (05 P73) + this ticket + the map's execution override, the same basis as packets 276–298. No code change; no queue-count change; no new fog, nothing out of scope.
