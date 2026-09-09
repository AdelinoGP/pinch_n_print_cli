# 77 — Author packet P70 — Quality / Precision — layer-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7c749484ffeLnKNdzJYftpXdg) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P70 — Quality / Precision — layer-planner** — 1 keys, Tier B new logic, owner layer-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P70 — Quality / Precision — layer-planner):

`precise_z_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code
change** (the ticket-28/39 shape). `precise_z_height` is live in canonical
(`PrintObjectConfig` coBool, default false — `ConfigOptionBool(0)`): the
`Slicing.cpp::generate_object_layers` +
`adjust_layer_series_to_align_object_height` last-5-layer redistribution that
fine-tunes heights so the final print_z equals the object height (each
adjusted height clamped to the min/max envelope; no-op when exact or shorter
than first layer + 5), called from `PrintObjectSlice.cpp`. Zero tree
occurrences as behaviour (no `crates/`/`modules/`/`xtask/` read); the
tier-table `layer-planner` owner **stands** — `layer-planner-default`'s
`generate_object_layers`
(`modules/core-modules/layer-planner-default/src/lib.rs`) is the direct
analog of canonical's (no ticket-27 hazard this time). Rule 4 does not fire
(in-stage adjustment, not cross-module selection — Q8 trigger test).
**User ruling (grilled 2026-09-09, Q1): re-file behind 141 + 144** rather
than author now with a substitute clamp — the clamp bounds are the feature's
core semantic, and a port-side invention would bake the divergence in. Named
non-borrows: the `Print.cpp` reslice-invalidation entry and the prime-tower
warning (tower stub — ticket 122 owns the body). Per-object shape rides the
existing overlay (packet 296 precedent), not ticket 125's axis. **Re-filed
as [145](145-author-packet-p70-precise-z-height-refiled.md), blocked on 141 +
144.** 04/05 rows annotated; no queue-count change; no new fog, nothing out
of scope.

### Gates

- No tree code changed, so no build/clippy/test gate applies.
