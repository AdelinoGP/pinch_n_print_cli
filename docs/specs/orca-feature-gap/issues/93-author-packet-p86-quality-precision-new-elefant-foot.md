# 93 — Author packet P86 — Quality / Precision — new: elefant-foot

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P86 — Quality / Precision — new: elefant-foot** — 2 keys, Tier C new module, owner new module elefant-foot. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P86 — Quality / Precision — new: elefant-foot):

`elefant_foot_compensation`, `elefant_foot_compensation_layers`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier C held, owner confirmed and sharpened, packet authored, no re-file.**

**Rule 3 (dead-in-canonical) — both keys pass.** Each is live in canonical's slicing
pipeline under one primary consumer, `PrintObject::slice_volumes`
(`PrintObjectSlice.cpp`), with downstream readers in `Brim.cpp::use_brim_efc_outline`
and `Fill.cpp`. `elefant_foot_compensation` is coFloat, default `0.`, min `0`;
`elefant_foot_compensation_layers` is coInt, default `1`, min `1`
(`PrintConfigDef::init_fff_params`). Canonical gates the pass on `raft_layers == 0`
("Only enable Elephant foot compensation if printing directly on the print bed") and
tapers it as `elfoot = efc - (efc / layers) * layer_id` under a `layer_id < layers`
guard. The kernel itself, `elephant_foot_compensation`
(`ElephantFootCompensation.cpp`), is a **width-limited variable inward offset**, not a
uniform one: it throttles the shrink wherever the contour is narrower than
`min_contour_width` so thin features survive.

**Zero-occurrence in this tree.** The single hit for either key across `crates/`,
`modules/`, and `xtask/` is the hardcoded `("elefant_foot_compensation", "0")` row in
`ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) — rule 2, not evidence.

**Size re-derived at claim time: packet, not direct implementation.** The map's
"Packets are for complex implementation only" rule resolves to *packet* here — the
work is a new module plus a non-trivial geometry kernel, not key declaration and
wiring.

**Owner confirmed, and sharpened to the seam.** Ticket 04's `new elefant-foot module`
holds. The precise seam is a new core module at stage **`Layer::SlicePostProcess`** —
which exists in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`)
between `Layer::PaintRegionAnnotation` and `Layer::Perimeters`, is treated by the
layer executor as a merge into the committed `SliceIR` rather than a primary commit,
and **carries zero production modules today** (exercised only by
`dispatch-layer-slice-postprocess-guest`). `elefant-foot` becomes its first occupant.
The pure kernel lands **ungated** in `crates/slicer-core/src/algos/` beside
`bridge_over_infill` so it compiles for `wasm32` and links into the guest (the
`gyroid-infill` dependency precedent). No WIT change, no IR field, no host service —
`slice-postprocess-builder.set-polygons` already exists.

**This deliberately does not follow packet 297's host-prepass shape.** 297 (conical
overhang) needed the layer *above*, a cross-layer read a per-layer stage cannot serve.
Elephant-foot reads only the layer's own footprint, so the per-layer module seam fits
exactly and the tier table's module assignment stands.

**Rule 4 does not fire.** One geometric correction, no canonical alternative
implementation, so no claim holder is minted: `[claims] holds = []`, `requires = []`.

**Packet `docs/spec_packets/303-elefant-foot-slice-postprocess/` authored (`draft`),
preflight PASS** (S0–S8 clean; 11 ACs + 2 negative). Both keys wired, **zero
declaration-only keys**. Five keys are *re-declared* to reach the decision point —
`support_raft_layers` (the gate; the `classic-perimeters` / `arachne-perimeters`
precedent) and `outer_wall_line_width` / `initial_layer_line_width` / `line_width` /
`nozzle_diameter` (feeding `slicer_core::flow::resolve_role_width` +
`line_width_to_spacing`, so `min_contour_width` is derived exactly as canonical's
`Flow` overload does). Re-declarations are not queue keys: **no queue-count change,
scoped target stays 409**. No range rejection beyond canonical's `min` bounds — the
GUI's `> 1` mm clamp in `ConfigManipulation.cpp` is a GUI hint under the ticket-113
rule. CONFIG_BLOCK emission rides as a live-key side effect; the padding twin is
untouched and its spelling stays with ticket 132. One new deviation row (three
clauses: the later mutation seam, the `lslices_elfoot_uncompensated` non-borrow, and
the substituted acceleration structure); the ID is re-derived at write time, not
frozen here.

**Three authoring defects were caught by the preflight symbol sweep and fixed before
this ticket closed**, all of the class the gate exists for: (a) the packet implied
canonical's `SCALED_EPSILON` was reusable — in this tree it is a file-private
`i128` in `crates/slicer-core/src/smooth_outward.rs`, a different type and magnitude,
so the kernel now declares its own; (b) a gate command used `rg -c`, whose *desired*
no-match outcome exits 1 and reads as failure; (c) Step 6 named `xtask/src/editions.rs`
as an edit surface, but core modules are discovered dynamically by `discover_guests`
and `dist/editions.toml` names only the three natively-integrated `hybrid` modules —
so edition membership needs no edit at all.

**Two neighbouring canonical keys were examined and deliberately left out.**

- `brim_use_efc_outline` (P05, currently `shed-to-queue` in
  `key-correction-inventory.md` as a rule-1 violation from packet 257) is blocked on
  EFC geometry *and* on brim following the real object contour. This packet supplies
  the first. It cannot supply the second: `skirt-brim`'s `generate_brim_entities`
  derives brim loops from a **bounding box**, which is ticket 12's recorded
  bbox-vs-contour divergence. Because brim never touches the object outline today,
  omitting canonical's uncompensated-footprint store changes **no** observable
  behaviour. Owner stays `skirt-brim`; filed as fog.
- `elefant_foot_layers_density` (canonical coPercent, min 50, max 100, default 100;
  read by `Fill.cpp` to densify solid infill across the same layer band) is
  **absent from `docs/ORCA_CONFIG_REFERENCE.md` entirely** — so it is not in the
  409-key queue and cannot be scoped by any packet until the gap source is corrected.
  Surfaced to ticket 123 (gap-source completeness audit) rather than smuggled in here.

**Ordering obligation handed to P88 (ticket 95).** Canonical applies
`_shrink_contour_holes` — the `xy_contour_compensation` / `xy_hole_compensation` pair —
to the same expolygons immediately *before* EFC inside `slice_volumes`. When P88 lands
on this same `Layer::SlicePostProcess` stage it must run first. Recorded in the
packet, not built by it.

No code change in this ticket. 04 and 05 row annotations are the implementer's Step 7.

