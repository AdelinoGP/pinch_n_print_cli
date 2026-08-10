# 04 — Define the cost rubric that makes "cheapest-first" decidable

Type: grilling
Status: resolved
Assignee: wayfinder session (ses_0173d7c4cffeYnq9tFnN69J1Ci)
Blocked by: 01
Map: ../map.md

## Question

What makes a missing feature "cheap" in *this* codebase, and how is every
in-scope key assigned a tier without hand-arguing each one?

The effort's chosen ordering is cheapest-first, which is only actionable if
"cheap" is a mechanical classification. Draft rubric to grill and sharpen:

- **Tier A — pure config plumbing.** Key declared in an existing module manifest
  or `docs/config/host-keys.toml` and consumed at a decision point that already
  exists. No IR change, no WIT change, no new module.
- **Tier B — new logic in an existing module.** Key drives new behaviour inside
  a module that already owns the relevant stage.
- **Tier C — new surface.** Requires a new core-module, a new IR field, a WIT
  change, or a new host-service bridge arm — i.e. anything the repo gates behind
  an ADR and a guest-WASM rebuild.

Settle: are three tiers enough? What is the tie-breaker *within* a tier — Orca UI
section, owning module, or print-quality impact? Does a key that is Tier A but
whose consumer decision point is itself missing get demoted to Tier B, and how
is that detected without reading every module?

Output: the rubric, plus the tier assignment applied to the in-scope inventory
(as a linked asset), since the tier counts directly size the packet queue.

## Answer

Asset: [`04-asset-tier-assignment.md`](./04-asset-tier-assignment.md) — the
rubric, the verified owner map, and the per-key assignment (414 rows).

### Headline

**A=118, B=223, C=15, D=47, X=11 — 403 keys in scope.** The queue is
dominated by plumbing (A) and new-logic-in-existing-owner (B) work; only 15
keys need new modules, 47 are deferred on the per-filament config model, and
11 were ruled out of scope.

### Adversarial review (user-requested, five passes until convergence)

Parallel reviewer subagents traced every key's canonical consumer in
`OrcaSlicerDocumented/`, in five passes (~90 placements corrected), until a
pass returned no real findings:

- **Pass 1** (6 reviewers): flow ratios, spiral, shell thickness, arc
  fitting, toolchange keys, flush-into keys, print_sequence/slicing_mode, 11
  global filament keys.
- **Pass 2** (3 reviewers): Seam (16/17) and Retraction (20) are
  emission-time; clearance keys are arrangement; 5 tree keys are live; **9
  keys ruled out of scope** (user ruling); precise_z_height folded;
  spiral_mode cross-cutting.
- **Pass 3** (3 reviewers): post_process + gcode_add_line_number are
  host-export; extruder_ams_count + nozzle_volume_type are tool-ordering;
  default_nozzle_volume_type is config-resolution; nozzle_height is
  skirt-brim; bridge_angle is consumed in LayerRegion/PerimeterGenerator.
- **Pass 4** (2 reviewers): max_layer_height is tool-ordering;
  min_layer_height is layer-planner; support_multi_bed_types is
  print/orchestration; default_bed_type + support_chamber_temp_control have
  no pipeline consumer → **2 more out of scope** (established classes).
- **Pass 5** (2 reviewers) — **converged**: all pass-4 corrections
  re-verified; 30-key random-sample audit found zero real findings.

Also found: `hole_to_polyhole_max_edges` missing from the 414 inventory
(flagged for ticket 01). Full detail in the asset's "Adversarial review
corrections" section.

### The rubric (all rulings confirmed with the human)

- **Tier A — plumbing into an existing decision point.** Owner exists AND the
  decision point exists (behaviour implemented under any key, hardcoded, or
  typed field). Work: declare + wire. The draft's "declared + consumed" Tier A
  was **empty** — none of the 414 are declared — so it was redefined.
- **Tier B — new logic in an existing owner.** Owner exists, decision point
  doesn't.
- **Tier C — new module at a new seam.** Granular: one feature per module,
  plural, not a catch-all. ADR where the repo gates new surfaces.
- **Tier D — deferred (fog).** The seam itself is unresolved (filament
  profile). Tiered C pending.
- **Tie-breaker within a tier: owning module** — keeps each owner's diff
  local across the queue.
- **Decision-point detection: mechanical proxy + authoring-time check.** The
  proxy (owner reads a sibling key from the same Orca section, exact name or
  Pinch rename) sizes the queue; the ambiguous remainder is verified per key
  at packet-authoring time. This also answers the fog patch
  "declared-but-not-consumed": it cannot be proven mechanically; the tier
  assignment is a sizing instrument, not a proof.

### Special rulings

- **5 ResolvedConfig-only keys** (`disable_m73`, `filament_density`,
  `filament_diameter`, `mmu_segmented_region_interlocking_depth`,
  `mmu_segmented_region_max_width`) are implemented via typed fields and
  consumed, but **not declared in any module manifest** — a contract
  violation. They are Tier A work: declare in the owning module's manifest +
  wire. (User ruling: "They should have been added to modules as well, not
  just via ResolvedConfig.")
- **Owners were verified in code, not assigned by name.** The verification
  corrected several name-based guesses: emission-time config lives in the
  **host emitter `crates/slicer-gcode`** (flavor.rs, estimator.rs, m73.rs,
  serialize.rs), not the machine-gcode-emit module; `spiral_vase` is read by
  the **perimeter modules**; `bed_shape` by **wipe-tower**; bridge pattern
  keys route through the **bridge-fill holder to infill modules** while
  bridge-flow keys stay in perimeters. (User ruling: "Explore into the code
  to verify what is the correct placement... do this for all of your
  claims.")
- **Modularity principle** (user ruling): a module implements a feature at
  its correct seam in the slicing chain, which may differ from Orca's
  organization. New modules are granular — Quality/Precision splits into
  five (elefant-foot, polyhole, arc-fitting, contour-compensation,
  z-height); Multimaterial advanced into two (interlocking,
  mmu-segmented-region).

### Fog graduated by this ticket

- **"Features with no owning module"** — resolved: owners assigned (wipe-tower
  for tower/toolchange/flush, host emitter for emission-time, new modules for
  precision/interlocking).
- **"Which Tier A keys are declared-but-not-consumed"** — answered: the proxy
  + authoring-time check; not mechanically provable, and the tier assignment
  is explicitly a sizing instrument.
- **"Whether any tier needs an ADR first"** — sharpened and absorbed by
  ticket 05: Tier C modules need ADRs; whether the ADR is a separate ticket or
  part of the packet ticket is 05's question.
- **"Where filament- and machine-level config even lives"** — machine-level
  is now assigned (host emitter + wipe-tower); the filament half remains fog
  as Tier D.
