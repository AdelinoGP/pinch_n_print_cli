# 89 — Author packet P82 — Extruder / Nozzle / Nozzle — config-resolution

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P82) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06, 104, 105
Map: ../map.md

## Question

Author the spec packet for **P82 — Extruder / Nozzle / Nozzle — config-resolution** — 1 keys, Tier B new logic, owner config-resolution. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P82 — Extruder / Nozzle / Nozzle — config-resolution):

`default_nozzle_volume_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed as out of scope: `default_nozzle_volume_type` is preset-management machinery, not slicing config — no packet, no code change** (the ticket-04/12 `brim_ears` precedent under Authoring rule 3, applied to the key, not a feature).

Claim-time grounding, verified against the oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) and re-derived from this tree:

- **Declaration** (`PrintConfig.cpp::add`): `coEnums` over `NozzleVolumeType` (Standard / High Flow / Hybrid / TPU High Flow), default Standard, `comDevelop` ("internal use only"). It is the *printer-profile* side of a default/current pair; its sibling `nozzle_volume_type` is the project side.
- **Every read site is preset/config-expansion plumbing, none of it in `libslic3r/`'s slicing pipeline:** `PresetBundle::load_selections` seeds the project `nozzle_volume_type` vector from the printer's default (with the short-vector length guard); `PresetBundle::reset_default_nozzle_volume_type` re-seeds it on extruder-count change (`on_extruders_count_changed`); `PresetBundle::get_default_nozzle_volume_types_for_filaments` indexes the *project* vector by filament map (its inputs are GUI plate state — `PartPlate.cpp`, `Plater.cpp` call sites — and it feeds `full_fff_config`'s `filament_volume_maps` composition, which lands in the `filament_volume_map` project key, not in a slicing read); the `PrintConfig.cpp` legacy-migration arm (`"Normal" → "Standard"`) and the `printer_extruder_options` / vendor-option set memberships are expansion bookkeeping. No read in `Print.cpp`, `GCode.cpp` emission, `ToolOrdering.cpp` grouping, or `FilamentGroup.cpp` scoring touches this spelling — those consume the project-side `nozzle_volume_type` / composed `filament_volume_map`, which are separate queued keys (P64 / ticket 143, P90 scope).
- **This is the exact shape of ticket 04's out-of-scope class**, applied to a key the adversarial passes missed: `default_bed_type` was ruled X because its only consumer is `Preset::get_default_bed_type` in preset-management called from GUI; `printer_technology` / `printer_variant` / `flush_volumes_vector` are X as preset-management metadata. This key's consumers are the same class — printer-profile seeding plus GUI-plate volume-map composition — with zero slicing-pipeline decision points. Filing a config-resolution packet for it would build preset-bundle/GUI machinery this CLI port has no equivalent of, or declare it with-gap (rule 1 prohibits both outcomes here: the first is out of scope, the second is declaration-only).
- **Rule 3's dead-in-canonical caution observed:** the key is *live* in canonical but only in the preset/GUI layer, which is the layer ticket 03's scope decision already drew the destination boundary at (print-host/preset-management ruled out at charting, `default_filament_profile` / `default_print_profile` precedent). The ruling is on the *layer*, not liveness.
- **Tree state:** zero occurrences under `crates/` + `modules/` + `xtask/` (only map-prose ledger hits, the reference snapshot rows, and tickets 71/143's "stays separate" notes — which named the separation correctly but not the consequence). No `ORCA_CONFIG_PADDING` twin, so rule 2 is not engaged. No packet number taken, no deviation rows, no code change; queue target 410 → 409 via the scope ruling.

**Records:** 04's `default_nozzle_volume_type` row re-tiered B → X (preset-management, this ticket's precedent); 05's P82 section annotated as dissolved (the authoring ticket for nothing — no re-file); queue target **410 → 409**; map Notes scoped-target line updated.
