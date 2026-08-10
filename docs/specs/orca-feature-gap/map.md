# Map: Close the OrcaSlicer FFF feature gap

Label: `wayfinder:map`

## Destination

A queue of **fully authored, preflighted spec packets** under `docs/spec_packets/` that
together cover every FFF (non-SLA) OrcaSlicer config feature Pinch 'n Print is
still missing — each packet complete with `packet.spec.md`, `requirements.md`,
`design.md`, `implementation-plan.md`, and passing `/spec-review --preflight`.
Packets are ordered **cheapest-first** (smallest diff before new geometry).

The map is done when every in-scope feature is either covered by an authored
packet or has been consciously ruled out of scope. Implementation (`/swarm`) runs
off-map, after.

## Notes

- **Domain:** 3D-printing slicer config/feature parity with OrcaSlicer. The gap
  source is `docs/ORCA_CONFIG_REFERENCE.md` — an upstream snapshot whose
  ✅/❌ "In Codebase" column is **hand-maintained and measurably wrong** (only
  the `Default` column is machine-read, by `xtask/src/gen_config_docs.rs`).
  Ticket 01 measured it: wrong on 66 of 574 FFF keys. **Never size anything off
  that column** — use ticket 01's asset, or re-derive.
- **Pinch 'n Print renamed Orca's keys — now being standardised away.**
  Ticket 07's ruling: **standardise to Orca's names**, not document. The
  mechanical rename workstream is tickets **99–107** (26 keys: 22 exact rows +
  3 duplicate collapses + `ironing_spacing_mm`); it **gates the queue** — P01
  (ticket 08) is blocked by all nine, and sessions take frontier tickets in
  order. `03-asset-scoped-gap.md` remains the historical adjudication; each
  workstream ticket updates its own rows there. The 34 Pinch-specific keys
  and the `raft_layers` 1→3 split (a strict superset — not a gap) stay
  untouched. The two narrowed ironing enums were reclassified as **gaps**:
  P14 +`ironing_type`, P15 +`support_ironing` (see Decisions so far).
- **The scoped target is 407 queue keys** (03's 414 minus 04's 11 rulings plus
  07's 2 reclassified ironing keys plus 99's 2 fan-scale keys) — per-key tier table in
  [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md), packet
  list in [`05-asset-packet-list.md`](issues/05-asset-packet-list.md). Size
  packets off those, never off the reference's ❌ column.
- **Execution override:** this map deliberately carries execution — packet
  *authoring* happens inside the map, not after it. Implementation does not.
- **Skills every session should consult:** `/grilling` and `/domain-modeling`
  for decision tickets; `/spec-packet-generator` for authoring; `/spec-review
  <packet> --preflight` as the authoring gate.
- **Repo rules that bind this effort** (`CLAUDE.md`): in-tree citations by
  symbol name + crate-qualified path, never bare line numbers; OrcaSlicer
  citations by file + function, never line numbers; ledger facts (next free
  packet number, next `DEV-###`, line counts) must be **re-derived at point of
  use**, never frozen into a ticket or packet.
- **Live ledger note:** the map's original 200–205 "untracked" hazard resolved
  itself — those packets are committed (spec-packets migration `a352c6b5`);
  206–212 also exist, with live uncommitted edits on 200/201. Numbering is fully
  decoupled from all of it by ticket 06's Rule 1: one number at a time, derived
  from disk at authoring time.

## Decisions so far

<!-- one line per resolved ticket: gist + link -->

- [01 — Build a mechanically verified FFF gap inventory](issues/01-verified-gap-inventory.md)
  — the real FFF gap is **419–481 keys, not ~640**; the hand-maintained ✅/❌
  column is wrong on 66 of 574 keys, and Pinch 'n Print uses an undocumented
  renamed key vocabulary (62 declared keys have no Orca counterpart), which is
  what makes the count a band rather than a number.
- [03 — Triage which verified-missing keys are not applicable at all](issues/03-nonapplicable-keys-triage.md)
  — **the queue must cover 414 keys** (405 after ticket 04's 11 additional
  out-of-scope rulings and ticket 07's 2 reclassified ironing keys). 42 ruled out of scope (print-host/preset,
  non-physical filament metadata, Bambu-proprietary, pellet, plater/GUI state);
  25 of the 62-key rename pool are genuine renames whose Orca key was a false
  gap, 34 are Pinch-specific, and 3 are *duplicate spellings of live keys*.
  Auto-set flags stay in scope as pipeline **outputs**; MMU toolchange physics
  stays in with no special sequencing.
- [02 — Set the canonical-parity evidence standard for gap packets](issues/02-parity-evidence-standard.md)
  — packets may assume the in-tree `OrcaSlicerDocumented/` checkout (readable,
  not runnable). Evidence is canonical function-read + described behaviour
  pinned by **invariant tests** — goldens are impossible here; porting
  OrcaSlicer's own `tests/fff_print/` assertions is acceptable with the
  attribution header. Plumbing keys need only default-matches-upstream +
  reaches-the-consumer. Unverifiable behaviour is surfaced to the human first
  and only filed as a `DEVIATION_LOG.md` row with their sign-off; never
  blocks. Boilerplate lives as the `parity-evidence` snippet in the
  spec-packet-generator skill.
- [04 — Define the cost rubric that makes "cheapest-first" decidable](issues/04-cost-tiering-rubric.md)
  — **A=119, B=226, C=15, D=47, X=11 — 407 keys in scope** (403 at 04's
  closure; +2 reclassified by ticket 07, +2 by ticket 99). Tier A = plumbing
  into an existing decision point (owner + decision point exist); B = new
  logic in an existing owner; C = new granular module at a new seam; D =
  deferred (per-filament config model); X = out of scope. Owners were
  **verified in code and adversarially reviewed against canonical OrcaSlicer
  five times until convergence** (~90 corrections): flow ratios, spiral,
  seam clipping and retraction are emission-time (host emitter
  `crates/slicer-gcode`); shell thickness is object-level planning;
  toolchange keys are emission-time not wipe-tower; 11 filament keys are
  global not per-filament; 11 keys ruled out of scope (dead-in-canonical 8,
  preset-management 3). 5 ResolvedConfig-only keys are Tier A
  manifest-declaration work. Tie-breaker: owning module. Full per-key table
  in [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md).
- [05 — Decide packet granularity and grouping](issues/05-packet-granularity.md)
  — **the queue is 91 packets (18 A, 67 B, 6 C; 358 keys)** (354 at 05's
  closure; P14 +`ironing_type` and P01 +`fan_max_speed`/`fan_min_speed` become
  mixed A/B — tickets 07/99 — P15 +`support_ironing` stays A). Grouping: owning
  module, then Orca UI section; tier is a purity check + queue-order key.
  Ceilings by tier: A ≤ 25, B ≤ 12, C ≤ 4 (split by sub-theme: Prime tower
  13+13, Retraction 10+10, Seam 8+8, Walls 9+9, interlocking 3+3). No merging
  of small groups (36 packets are ≤2 keys — packet 212 precedent). ADRs only
  for interlocking + mmu-segmented-region, authored inside the packet ticket.
  Full list in
  [`05-asset-packet-list.md`](issues/05-asset-packet-list.md) — the 91
  authoring tickets are cut from it. 47 D + 2 fog-blocked A keys
  (`filament_density`, `filament_diameter`) not packetized.
- [06 — Settle packet numbering and how this queue interleaves with live work](issues/06-queue-numbering-and-sequencing.md)
  — **one packet number at a time, allocated by directory existence, derived
  from disk at authoring time; no reserved block.** Derivation command:
  `ls -d docs/spec_packets/[0-9]*/ | sed ... | sort -n | tail -1`; next free =
  +1; letter suffixes only for re-splits (210a/b precedent). All packets born
  `status: draft`; activation is a `/swarm`-time act. Authoring proceeds in
  parallel with live packet work (200–205 are committed; 200/201 have in-flight
  edits) — no merge blocking; numbering decouples via Rule 1.
- [07 — Document the Orca→Pinch alias map and retire the hand-maintained ❌ column](issues/07-alias-map-and-column-retirement.md)
  — **standardise to Orca's names, don't document; the alias map is
  eliminated, not maintained.** 26 mechanical renames (22 exact rows + 3
  duplicate collapses + `ironing_spacing_mm`) executed as workstream tickets
  **99–107**, gating the queue (08 blocked by all nine). Shape changes stay
  out of the rename: `raft_layers` 1→3 split is a strict superset (recorded
  divergence, no gap); the two narrowed enums are **gaps** — the shared
  `ironing_enabled` bool can't express `ironing_type`'s modes nor toggle the
  two Orca features independently → P14 +`ironing_type` (B), P15
  +`support_ironing` (A). 34 Pinch-specific keys untouched. ❌ column
  retirement ruled **out of scope** (tooling hygiene; queue never reads it).
- [99 — Rename part-cooling keys to Orca names](issues/99-rename-part-cooling-keys.md)
  — four renames merged, tree green on all gates. The rename **exposed a
  scale deviation**: Orca's `fan_max_speed`/`fan_min_speed` are percent
  (0–100) while Pinch's were raw 0–255, and `fan_min_speed` was declared but
  never read → reclassified as gap work, **P01 +`fan_max_speed`/`fan_min_speed`
  (Tier B)** — queue is now 407 keys, 358 in packets (18 A / 67 B / 6 C).
  Known pre-existing condition reported: `cargo xtask build-guests --check`
  reports all 30+ guests stale even on a clean tree (unrelated to renames).

## Not yet specified

- **Where filament-level config even lives.** 47 keys (Tier D) are deferred
  on this question: does Pinch 'n Print have a per-filament config model at
  all, or do these keys imply a new subsystem? 11 filament keys were found
  to be global (not per-filament) and are assignable now (ticket 04).
  Revisit once the queue reaches Tier D. Graduating with it: 2 fog-blocked
  Tier A keys (`filament_density`, `filament_diameter` — declare-in-manifest
  work whose manifest home depends on the model).

## Out of scope

- **SLA printing** — the whole `## SLA Printing` section (Support/Material/Pad/
  Display/Exposure/Hollowing/Faded-layers). A different pipeline entirely, not an FFF feature
  gap. Ruled out at charting time by the effort's scope decision; returns only
  if the destination is redrawn.
- **42 FFF keys ruled out by class** in
  [03](issues/03-nonapplicable-keys-triage.md) — print-host / preset management
  (17), non-physical filament metadata (9), Bambu-proprietary hardware (8),
  pellet-extruder hardware (2), plater / GUI state (6). Per-key list and class
  assignment in [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md).
  *Physical* filament keys and auto-set flags were explicitly kept in scope.
- **11 more keys ruled out by ticket 04's adversarial reviews** (user
  ruling, dead-in-canonical / preset-management classes) —
  dead-in-canonical (OrcaSlicer never reads them in the pipeline):
  `enable_timelapse` (superseded by `timelapse_type`), `allow_mix_temp`,
  `wiping_volumes_extruders`, `tree_support_with_infill` (obsolete in
  canonical's IGNORE set), `first_layer_sequence_choice` /
  `other_layers_sequence_choice` (dead alternate spellings),
  `support_chamber_temp_control` (GUI-only); preset-management (matching 03's
  class): `printer_technology`, `printer_variant`, `flush_volumes_vector`,
  `default_bed_type`. Per-key rows in
  [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md).
- **Retiring the hand-maintained ❌ column of `docs/ORCA_CONFIG_REFERENCE.md`**
  (07 ruling) — replacing it with generated presence flags + a `--check` gate
  is tooling hygiene, not a queue prerequisite: the queue never reads the
  column (ticket 01's asset and the map Notes neutralise its 66-key error).
  Returns only if the destination is redrawn. The standardisation workstream
  (99–107) makes the vocabulary converge meanwhile, shrinking the column's
  remaining drift surface.
- **Standardising the 34 Pinch-specific keys or the `raft_layers` 1→3 split**
  (07 ruling) — the Pinch-specific keys have no Orca counterpart, and the raft
  split is a strict superset of Orca's single count; neither is a gap or a
  rename. The raft divergence is recorded in
  [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md)'s 07 update.
