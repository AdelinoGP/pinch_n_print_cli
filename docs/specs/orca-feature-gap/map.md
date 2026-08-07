# Map: Close the OrcaSlicer FFF feature gap

Label: `wayfinder:map`

## Destination

A queue of **fully authored, preflighted spec packets** under `.ralph/specs/` that
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
- **Pinch 'n Print renames Orca's keys.** The alias map is adjudicated in
  [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md) — consult it before
  treating any key as a gap. It also records three keys that exist under *two*
  spellings (`infill_density`/`sparse_infill_density` and friends) and two Orca
  enums narrowed to Pinch bools (`ironing_type`, `support_ironing`), which are
  parity gaps hiding inside keys that look present.
- **The scoped target is 414 keys** — see the by-section list in
  [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md). Size packets off
  that, never off the reference's ❌ column.
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
- **Live ledger hazard:** packets `200-205` exist untracked in the working tree
  and are not yet merged. Any numbering decision must be re-derived from disk.

## Decisions so far

<!-- one line per resolved ticket: gist + link -->

- [01 — Build a mechanically verified FFF gap inventory](issues/01-verified-gap-inventory.md)
  — the real FFF gap is **419–481 keys, not ~640**; the hand-maintained ✅/❌
  column is wrong on 66 of 574 keys, and Pinch 'n Print uses an undocumented
  renamed key vocabulary (62 declared keys have no Orca counterpart), which is
  what makes the count a band rather than a number.
- [03 — Triage which verified-missing keys are not applicable at all](issues/03-nonapplicable-keys-triage.md)
  — **the queue must cover 414 keys.** 42 ruled out of scope (print-host/preset,
  non-physical filament metadata, Bambu-proprietary, pellet, plater/GUI state);
  25 of the 62-key rename pool are genuine renames whose Orca key was a false
  gap, 34 are Pinch-specific, and 3 are *duplicate spellings of live keys*.
  Auto-set flags stay in scope as pipeline **outputs**; MMU toolchange physics
  stays in with no special sequencing.

## Not yet specified

- **Where filament- and machine-level config even lives.** The two largest
  ❌ clusters are Filament Notes (~43), Printer identity (~26), Bed temperature
  (~17) and Cooling notes (~23). It is not yet clear whether Pinch 'n Print has
  a filament/printer *profile* concept at all, or whether these keys imply a new
  subsystem rather than new packets. Revisit once the verified inventory (01)
  and the cost rubric (03) exist.
- **Features with no owning module.** Prime tower (~31 ❌) maps loosely to
  `modules/core-modules/wipe-tower`; MMU hardware, flush options and ooze
  prevention have no obvious owner. Whether these need new core-modules, and
  whether that needs an ADR before any packet, is unresolved.
- **Whether any tier needs an ADR first.** Some clusters (multimaterial,
  machine limits, per-role line width) may change IR or WIT surfaces, which this
  repo gates behind ADRs. Which clusters those are is fog until 03 tiers them.
- **The bulk authoring batches.** The main body of the map — one ticket per
  packet-authoring batch — cannot be specified until granularity (04) is
  settled. Expect this patch to graduate into the majority of the map's tickets.
- **Which Tier A keys are declared-but-not-consumed.** 01 established that a key
  appearing in a module `[config.schema]` manifest does not prove anything reads
  it at a decision point. Detecting that at scale — without hand-reading every
  module — is a question the cost rubric (04) will need answered and cannot
  answer itself. Sharpens once 04 fixes the tier boundaries.

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
