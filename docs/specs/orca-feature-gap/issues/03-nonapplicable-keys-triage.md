# 03 — Triage which verified-missing keys are not applicable at all

Type: grilling
Status: resolved
Blocked by: 01
Map: ../map.md

## Question

Of the verified-missing FFF keys, which will Pinch 'n Print **never** implement,
and on what principle?

Not every Orca key is a feature gap. Candidate exclusion classes to test against
the human:

- GUI-only / editor-state keys with no slicing meaning in a CLI slicer.
- Vendor- or firmware-specific keys tied to Bambu/Orca-specific hardware.
- Printer-profile *metadata* (identity, notes, thumbnails) that describes a
  machine rather than driving the pipeline.
- Keys whose behaviour we already provide under a different name — these are
  renames, not gaps, and belong in `docs/DEVIATION_LOG.md`, not in a packet.
- Auto-set / derived flags (e.g. keys the reference marks `[hidden]` or
  `(auto-set)`) that a slicer computes rather than accepts.

Output: the exclusion principle plus the resulting key list, recorded on this
ticket and reflected in the map's **Out of scope** section. Everything not
excluded is the true scope this map must cover with packets.

## Answer

Asset: [`03-asset-scoped-gap.md`](./03-asset-scoped-gap.md) — the exclusion
table, the confirmed alias map, the Pinch-specific residue, and the full scoped
key list by section.

### Headline

**The packet queue must cover 414 keys.** Not the ~640 the reference's ❌ column
implies, and not ticket 01's 481 upper bound. The band from 01 is now closed:
42 keys ruled out of scope, 25 already implemented under a Pinch name.

| | count |
|---|---:|
| Absent by exact Orca name (ticket 01) | 481 |
| − ruled out of scope | 42 |
| − already implemented under a Pinch name | 25 |
| **= in-scope gap** | **414** |

Largest remaining clusters: Filament/Notes 32, Multimaterial/Prime tower 28,
Extruder/Retraction 20, Quality/Walls and surfaces 19, Cooling/Notes 19,
Quality/Seam 17, Filament/Bed temperature 17.

### Exclusion principle

Five classes, all confirmed with the human:

1. **Print-host / preset management (17).** `printhost_*`, `print_host`,
   `print_host_webui`, `host_type`, `bbl_use_printhost`, `printer_agent`,
   `printer_settings_id`, `printer_notes`, `upward_compatible_machine`,
   `default_print_profile`, `default_filament_profile`. Upload credentials and
   preset inheritance; `pnp_cli` writes a `.gcode` file and stops.
2. **Filament metadata, non-physical (9).** `filament_notes`,
   `filament_settings_id`, `filament_ids`, `filament_vendor`,
   `filament_colour_type`, `filament_multi_colour`, `default_filament_colour`,
   `filament_adhesiveness_category`, `filament_printable`. The *physical*
   filament keys stay in scope — `filament_density`, `filament_cost`,
   `filament_diameter`, `filament_flow_ratio`, `filament_max_volumetric_speed`,
   `filament_shrink`, `filament_shrinkage_compensation_z`, `filament_soluble`,
   `filament_is_support`, `filament_type`, `temperature_vitrification`.
3. **Bambu-proprietary hardware (8).** `bbl_calib_mark_logo`,
   `head_wrap_detect_zone`, `scan_first_layer`, `enable_wrapping_detection`,
   `wrapping_detection_layers`, `wrapping_exclude_area`,
   `allow_multicolor_oneplate`, `nozzle_flush_dataset`.
4. **Pellet extruder hardware (2).** `pellet_flow_coefficient`,
   `pellet_modded_printer`.
5. **Plater / GUI state (6).** `bed_custom_model`, `bed_custom_texture`,
   `best_object_pos`, `preferred_orientation`, `print_order`, `notes`. A CLI
   slicer receives an already-positioned model.

**Auto-set flags stay in scope**, by explicit decision: `has_scarf_joint_seam`
(the reference's only `[hidden]`/`(auto-set)` FFF row) is treated as an *output*
the pipeline computes and exposes, not an input it accepts. Whichever packet
implements scarf-joint seams owns it.

MMU toolchange physics (`filament_ramming_parameters`, `filament_cooling_moves`,
the loading/unloading speed family, `filament_stamping_*`) is **in scope with no
special sequencing** — the cost rubric alone decides where it lands.

### The 62-key rename pool, adjudicated

Split three ways:

- **25 are genuine renames** whose Orca key was being counted as a gap. Full
  table in the asset. Twenty are exact one-to-one (`wall_count`/`wall_loops`,
  `retract_length`/`retraction_length`, `travel_z_hop`/`z_hop`,
  `bed_shape`/`printable_area`, `wipe_tower_*`/`prime_tower_*`, and the
  unit-suffix pair `support_top_z_distance_mm` / `smaller_perimeter_threshold_mm`).
  Five are not clean:
  - `raft_layers` **split into three** Pinch keys (`support_raft_layers`,
    `base_raft_layers`, `interface_raft_layers`).
  - `ironing_type` and `support_ironing` are Orca **enums narrowed to a Pinch
    bool** (`ironing_enabled`). Behaviour is a subset — a packet touching ironing
    must decide whether to widen it, and this is a real parity gap hiding inside
    a "present" key.
- **34 are Pinch-specific**, with no Orca counterpart. They remove nothing from
  the gap list; they are the alias map's "no Orca entry" rows.
- **3 are neither — they are duplicates.** `infill_density`, `infill_speed` and
  `infill_overlap` are declared *alongside* the Orca-named
  `sparse_infill_density`, `sparse_infill_speed` and `infill_wall_overlap`,
  which are also live. Two spellings of the same setting exist in the tree.

### Two findings that were not the question but matter

1. **Fuzzy skin strips the module prefix.** `modules/core-modules/fuzzy-skin`
   declares its keys as bare `thickness`, `point_distance`, `apply_to_all`.
   Ticket 01's exact-name diff could never have caught these, and there is no
   namespacing convention in the manifests to make the collision safe — no
   declared key anywhere in the tree contains a dot. A key called `thickness` in
   a shared config space is a name waiting to collide.
2. **The two ironing modules disagree with each other.**
   `top-surface-ironing` declares `ironing_flow` + `ironing_spacing_mm`;
   `support-surface-ironing` declares `ironing_flow_rate` + `ironing_spacing`.
   Same concept, four spellings, two of which match Orca and two of which don't.
   This is an internal inconsistency, not just an upstream deviation.

Neither is a feature gap, so neither belongs in the packet queue — but both are
alias-map content and both argue that ticket 07 is load-bearing rather than
cosmetic.

### Verification

Every excluded and renamed key was checked to be currently listed *absent* in
ticket 01's inventory, and to exist in that inventory at all — three sanity
sweeps, all empty. The 414 figure is `481 − 42 − 25` with no double-counting.

Rename adjudication was done by reading each key's `[config.schema]` entry
(type, default, display name) in its owning manifest, not by name similarity.
Where the reference snapshot has no counterpart, the key is called
Pinch-specific rather than guessed at.

**Caveat inherited from 01 and not resolved here:** these 414 are absent by
*name*. A key can still be present-in-name and unimplemented-in-behaviour — the
narrowed `ironing_type` bool is a live example. Ticket 04 owns that distinction.
