# 160 — Graduate Phase 2 SOTA packet re-authoring batches

Type: grilling
Status: open
Assignee: —
Blocked by: 123, 158, 159
Map: ../map.md

## Question

**Lock the final Phase 2 SOTA packet re-authoring plan — confirming packet batching, dissolves / folds / direct-implementation demotions, newly surfaced upstream keys from `123`, and any late delta from `159` — and graduate the concrete Phase 2 packet re-authoring tickets out of `map.md`'s `## Not yet specified`.**

Before grilling, read:
- [158](./158-triage-draft-packets-for-drift-and-phase2-batching.md)'s triage asset (`./158-asset-packet-drift-triage.md`)
- [123](./123-audit-gap-source-key-set-completeness.md)'s upstream completeness findings (any missing sibling keys like `elefant_foot_layers_density`, `farthest_point_timelapse`, `flush_multiplier_fast`, `prime_volume_mode`, etc. that should fold directly into Phase 2 packets)
- [159](./159-gate-merge-closed-perf-and-proptest-into-branch.md)'s late-merge delta from the final closure of `perf-vs-orca` and `proptest-openrscad`
- The current resolution state of open HITL rulings (`125`, `127`, `128`, `148`, `153`) so any still-open ruling is wired as a native `Blocked by:` edge on the exact Phase 2 batch tickets that depend on it.

### Decisions to settle with the human

1. **Approve per-packet dispositions from `158`:** confirm which legacy packets are dissolved (already live or dead-in-canonical), demoted to direct implementation (no packet), folded together, split, or kept as packets.
2. **Fold missing canonical keys from `123`:** assign any verified in-scope missing keys to their owning Phase 2 batch (or new ticket) so each packet is rewritten **once** with a complete key set.
3. **Lock the Phase 2 ticket batches & SOTA rewrite rubric:** confirm batch boundaries (2–4 closely related packets per session, 1 ticket each for major architectural redesigns like interlocking beams and prime tower body parity), confirm every batch ticket targets the modern 2-file `orca-feature-gaps_<NN>_<slug>/` layout (`packet.spec.md` + `implementation-plan.md`), `docs/22_test_quality.md`, Authoring Rules 1–6, and optional `/spec-review --preflight`.
4. **Wire blocking edges & clear the fog:** create the graduated Phase 2 batch tickets (and any direct-impl tickets), wire blocking edges onto any still-open rulings (`125`, `127`, `128`, `148`, `153`), and clear the graduated *"Phase 2 SOTA re-alignment of the 52 draft packets"* patch from `map.md`'s `## Not yet specified`.

### Obligations

- **HITL ticket:** call `/grilling` and `/domain-modeling`. Never answer for the human.
- **Plan, don't do:** this session graduates the Phase 2 tickets; it does not rewrite packet files itself.
- Re-derive the next free ticket number from `docs/specs/orca-feature-gap/issues/` at claim time.

## Answer
