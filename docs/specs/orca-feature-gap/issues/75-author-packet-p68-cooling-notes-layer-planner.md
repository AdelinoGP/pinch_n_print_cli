# 75 — Author packet P68 — Cooling / Notes — layer-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7cb0f4abffelgkZbH6EkvbE7R) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P68 — Cooling / Notes — layer-planner** — 1 keys, Tier B new logic, owner layer-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P68 — Cooling / Notes — layer-planner):

`min_layer_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code change**
(the ticket-28/39 shape — and the sibling of ticket 69's `max_layer_height`, whose
re-file [141](./141-author-packet-p62-max-layer-height-tool-ordering-refiled.md)
already owns half of this key's envelope).

Canonical grounding (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — paths re-derived at
point of use per the map Notes): `min_layer_height` is declared `coFloats`,
default `{0.07}`, min 0 (`PrintConfig.cpp`; tooltip: "the lowest printable layer
height for the extruder. Used to limit the minimum layer height when enable
adaptive layer height"). It passes rule 3 — it is live in `libslic3r/` — but its
only live behaviour subject is the **variable-layer-height machinery**: the
per-nozzle `Slicing.cpp::min_layer_height_from_nozzle` (`0 → MIN_LAYER_HEIGHT_DEFAULT`,
else `max(MIN_LAYER_HEIGHT, value)`) feeding `SlicingParameters::create_from_config`'s
min/max envelope, which clamps the adaptive profile (`layer_height_profile_adaptive`,
`clamp(z_gap, min, max)`). The two apparent other consumers are not decision
points: the `GCode.cpp` min read sits inside a `/* FIXME … */` **commented-out**
block (`collect_layers_to_print`), and the `Print.cpp` opt_key entry is
invalidation bookkeeping, not behaviour. Canonical's own enable switch is dead —
`adaptive_layer_height` is commented out of `PrintConfig.cpp` (already recorded
on 141) — the envelope is live, the switch is not.

Tree evidence (from disk, not the tier table): **zero occurrences** of the key as
config under `crates/`/`modules/`/`xtask/`. The one `min_layer_height` spelling
in-tree (`tree-support-planner`'s `lib.rs`) is a plan-derived local — the min over
the `LayerPlanView` effective heights — not the Orca key, and reads no config.
`layer-planner-default` is an explicit uniform MVP (`first + n * step` in
`generate_object_layers`, "MVP — uniform layers" in its own doc comment): no
variable profile exists to clamp, and no per-extruder vector model exists either
(`nozzle_diameter` is an `extensions` scalar — 141's finding, same shape here
since both keys are per-nozzle `coFloats` and first-wins ingestion keeps element 0
only). A packet today would be 100% declaration-only — prohibited by rule 1.
Tier B held as effort; the tier-table `layer-planner` owner stands (no ticket-27
correction — the envelope belongs with the layer planner, one of 141's own owner
options).

**Re-filed as [144](./144-author-packet-p68-min-layer-height-refiled.md), blocked
on 06 + 125.** Not folded into 141 now: 141 sequences after 122 for its
tower-partition consumer, and this key has no tower consumer — folding today would
over-block it behind the tower body. 141 stays the fold candidate at claim time
(same `SlicingParameters` envelope; a packet building the envelope for max alone
and leaving min out would be artificial). 04/05 rows annotated; no queue-count
change.

### Gates

- `grep -rn "min_layer_height" crates/ modules/ xtask/` — zero config-key hits
  (one plan-derived local, named above); canonical reads verified against the
  oracle paths above. No code changed, so no test/clippy gate applies beyond the
  commit gates (`check-literals` clean, `clippy --workspace --all-targets` clean).
