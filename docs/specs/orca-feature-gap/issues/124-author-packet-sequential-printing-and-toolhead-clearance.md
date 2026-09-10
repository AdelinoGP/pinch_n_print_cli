# 124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet (or packet series) that gives this port **sequential
printing — `print_sequence == ByObject` — and the toolhead-clearance validation
canonical attaches to it**, at parity with OrcaSlicer's behaviour.

Filed by [ticket 32](./32-author-packet-p25-extruder-nozzle-nozzle-skirt-brim.md),
which holds the read-site analysis and the from-disk evidence that none of this
exists here. **Read ticket 32's answer first**; it is not restated.

### Why this is a feature ticket and not three key tickets

Canonical's `Print::sequential_print_clearance_valid` reads `nozzle_height`,
`extruder_clearance_radius`, `extruder_clearance_height_to_rod` and
`extruder_clearance_height_to_lid` in one function, guarding one mode
(`print_sequence == PrintSequence::ByObject`). The queue splits those keys over
three packets with three different owners (P25 / P69 / P79), and no one of them
can close alone: the clearance keys have no validator without the mode, the mode
has no meaning without the clearance test, and `nozzle_height` feeds both. This
ticket owns the feature; the keys follow it.

### Keys this carries

- `nozzle_height` — folded in from P25, which is **dissolved** by ticket 32.
- From [76](./76-author-packet-p69-others-special-mode-layer-planner.md) (P69):
  `print_sequence`. `slicing_mode` is a separate question and P69 may keep it —
  that ticket's session decides, not this one.
- From [86](./86-author-packet-p79-printer-machine-print-volume-print-orchestration.md)
  (P79): `extruder_clearance_radius`, `extruder_clearance_height_to_rod`,
  `extruder_clearance_height_to_lid`.
- From [87](./87-author-packet-p80-quality-walls-and-surfaces-print-orchestration.md)
  (P80, **dissolved** 2026-09-10): `extruder` — canonical's per-object/volume tool
  assignment (`apply_to_print_region_config` + `normalize_fdm`, `PrintObject.cpp`;
  fans out onto the six `*_filament_id` selectors ticket 46 already resolves at
  this port's runtime entity-assembly seam). Read ticket 87's answer for the
  grounding and the named non-borrows (shared-object cache predicate,
  `auto_assign_extruders`, GUI arms); it is not restated here.

Re-derive membership from disk at authoring time; do not freeze it from here.
Adjacent keys that this feature *may* pull in — `skirt_type == stPerObject` (left
declared-with-gap by ticket 13 for want of per-object skirt grouping),
`bed_exclude_area` (packet 256, authored not merged) — are **not** claimed here.
Check at authoring whether canonical couples them tightly enough to belong, and
say so either way rather than silently absorbing them.

### What the packet must settle first

**Scope, with the human.** "Sequential printing" spans at least three separable
layers, cheapest first:

1. **Validation only** — accept the keys, run canonical's horizontal (convex-hull
   inflate by `obj_distance`) and vertical (`extruder_clearance_height_to_lid` /
   `_to_rod` against instance bounding boxes, ordered by arrange order) tests, and
   reject an unprintable arrangement with a structured error. This is the part
   `nozzle_height` and the `extruder_clearance_*` keys actually drive.
2. **Emission** — actually print object-by-object: one object's full height
   before the next starts, which reorders the whole layer loop.
3. **The GUI-side consumers** — `Arrange.cpp` / `ArrangeJob.cpp` /
   `GLCanvas3D.cpp`. This port is a CLI and has no arranger; these are almost
   certainly out of scope, but say so explicitly rather than by omission.

(1) is coherent on its own and closes every key in the list; (2) is a much larger
change to the layer loop and the emitter. **Get the ruling before authoring** —
the prime-tower precedent (ticket 29) is that this map takes scope rulings from
the human, and the answer determines whether this is one packet or a series.

### Authoring obligations

- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet>
  --preflight` (must pass). Packet number and status derived from disk at
  authoring time (ticket 06).
- **Authoring rules 1–6 bind.** In particular rule 1: a packet that declares
  `nozzle_height` and the clearance keys without building the validator they
  drive is exactly the packet ticket 32 refused to author.
- Rule 4 — the validation belongs where this tree puts validation
  (`crates/slicer-scheduler/src/validation.rs` already emits fatal structured
  errors for holder misconfiguration; confirm the seam from code rather than
  assuming it). The clearance test needs per-instance convex hulls and object
  heights, so verify at authoring what the prepass IR actually exposes about
  object placement.
- Apply ticket 02's parity-evidence standard — canonical function-read plus
  invariant tests. `OrcaSlicerDocumented/` is readable, not runnable.
- If scope lands on (2), split by layer with validation first and say so in the
  answer rather than authoring one packet that cannot close.

Resolved when the packet (or packet series) is authored, preflighted, and its
directory linked here.

## Answer
