# 78 — Author packet P71 — Quality / Overhangs — slice-prepass

Type: task
Status: resolved
Assignee: wayfinder-session-2026-09-09
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P71 — Quality / Overhangs — slice-prepass** — 3 keys, Tier B new logic, owner slice-prepass. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P71 — Quality / Overhangs — slice-prepass):

`make_overhang_printable`, `make_overhang_printable_angle`, `make_overhang_printable_hole_size`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owner confirmed, packet authored — no re-size, no re-file.** Claim-time grounding (two delegated surveys) holds all three keys in: each is live in canonical's slicing pipeline under a single consumer, `PrintObject::apply_conical_overhang` (`PrintObjectSlice.cpp`, invoked from `PrintObject::slice`) — `make_overhang_printable` (coBool, default `false`, double per-layer/per-region gate), `make_overhang_printable_angle` (coFloat, default `55.0`, `== 90.0` early-return else `tan(angle) * layer_height` offset), `make_overhang_printable_hole_size` (coFloat, default `0.0` mm², small-covered-hole cut from the upper layer) — and zero-occurrence as behaviour in this tree (20 doc-only hits, 0 in any `.rs`/`.toml`/`.wit`/`.json`; no `apply_conical_overhang` analog; the neighbour `PrePass::OverhangAnnotation` only classifies, never grows). The tier table's `slice-prepass (apply_conical_overhang)` owner is confirmed and corrected to this tree's seam: pure kernel in `slicer-core::algos` beside `overhang_annotation`, thin producer beside `overhang_annotation_producer`, registered between `PrePass::Slice` and `PrePass::OverhangAnnotation` via `run_builtin_stage`, mutating committed `SliceIR` through the `PaintSegmentation` `replace_slice_ir` precedent and reusing `polygon_ops` offset/union/difference (no new boolean code, no ticket-27 hazard — the owner is the host prepass, not a module).

Packet `docs/spec_packets/297-conical-overhang-slice-prepass/` authored (`draft`), **preflight PASS** (S0–S8 + AC-command + Doc-Impact checks, all tree-verified by delegation): 3 `ResolvedConfig` fields at canonical defaults, per-object shape on the existing overlay (explicitly not ticket 125's tool axis), rule 4 does not fire (scalar parameters of one pass, not claim holders), no range validation (canonical bounds are GUI hints — no deviation), CONFIG_BLOCK honest side-effect only with the bool spelling riding ticket 132, zero declaration-only keys, zero new deviations, zero new ADRs, no schema bump, six S/M steps. Next free packet number re-derived at authoring (296 max → 297); overlap grep clean.

04/05 linkage rows are the packet implementer's Step 6 (not this ticket). No queue-count change; no new fog, nothing out of scope.
