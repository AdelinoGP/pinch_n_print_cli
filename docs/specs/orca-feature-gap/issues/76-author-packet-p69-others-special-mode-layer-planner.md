# 76 — Author packet P69 — Others / Special mode — layer-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7c84b256ffeUUsZ86sRT01tSl) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06, 104
Map: ../map.md

## Question

> **Ticket 32 note (2026-09-03):** `print_sequence` is the mode gate for
> sequential printing, whose clearance validation is owned by
> [124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation](./124-author-packet-sequential-printing-and-toolhead-clearance.md).
> It cannot close here: `print_sequence == PrintSequence::ByObject` selects a
> print mode this port does not have, and the keys that make it meaningful
> (`nozzle_height`, `extruder_clearance_*`) live in ticket 124. **Fold
> `print_sequence` into 124 when this ticket is claimed.** `slicing_mode` is a
> separate question and may well stay with P69 — this session decides that, not
> ticket 124.

Author the spec packet for **P69 — Others / Special mode — layer-planner** — 2 keys, Tier B new logic, owner layer-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P69 — Others / Special mode — layer-planner):

`print_sequence`, `slicing_mode`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Split at claim time: `print_sequence` folded into ticket 124, `slicing_mode` authored as packet 296** (human-grilled Q1–Q2, 2026-09-09 — the ticket-32 shape for the first key, a Tier B packet for the second; no code change, no packet number taken for the fold).

### `print_sequence` — folded into 124

Canonical `print_sequence` (`PrintConfig` coEnum `by layer`/`by object`, default `by layer`) is live — `Print::sequential_print_clearance_valid` (`Print.cpp`), the `GCode.cpp` ByObject emission arms, `ToolOrdering.cpp` order selection, `Brim.cpp` per-object brim — so it passes rule 3 and stays in scope. But every slicing meaning it has rides the sequential-printing validator and ByObject emission this port does not have (ticket 32's finding: `print_sequence` appears here only as the `("print_sequence", "by layer")` padding row, rule 2). Ticket 124 already lists this key as carried (`nozzle_height` from P25 + `print_sequence` from P69 + `extruder_clearance_*` from P79); the fold needs no new ticket and no 124 edit. P69 keeps `slicing_mode` alone.

### `slicing_mode` — authored as packet 296

Canonical `slicing_mode` (`PrintObjectConfig` coEnum `regular`/`even_odd`/`close_holes`, default `regular`) is live — the `PrintObjectSlice.cpp` slicing-mode switch into `MeshSlicingParams` fill rule (`Regular`/`EvenOdd`/`Positive`) — so it passes rule 3 and stays in scope. Tree evidence (from disk, not the tier table): **zero occurrences** of the key as config under `crates/`/`modules/`/`xtask/`; `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) always unions `EvenOdd` (identical to `Regular` for valid manifold meshes, per the `polygons_to_expolygons` doc comment); no Positive branch exists. The tier-table `layer-planner` owner is wrong (ticket-27 hazard): `layer-planner-default` plans uniform layer heights only — the union site is the slicer-core prepass (`execute_prepass_slice_single_layer_impl`, `crates/slicer-core/src/algos/prepass_slice.rs`, fed by `RegionMapIR::config_for`), so the packet corrects the owner there, following the `slice_closing_radius` plumbing precedent. Rule 4 does not fire (in-stage fill-rule parameter, not cross-module algorithm selection — Q8 trigger test).

Packet [`296-slicing-mode-prepass`](../../../spec_packets/296-slicing-mode-prepass/packet.spec.md) authored as `draft`, preflight **PASS** (S0–S8 + AC-command + Doc-Impact, operator-verified 2026-09-09 after subagent rate-limit — all tree greps run directly; DEV-188 absent from the log, all preexisting symbols resolved, per-file test binaries need no aggregator). One key in, none shed, none returned: `ResolvedConfig` field (canonical default, per-object via the existing overlay — explicitly not ticket 125's tool axis) + additive mode-aware slice entry (`regular`/`even_odd` → existing EvenOdd path, byte-identical; `close_holes` → CCW + `FillRule::Positive` union, both variants verified live in `clipper2-rust`) + prepass read beside `slice_closing_radius`. DEV-188 ((a) Regular-as-EvenOdd simplification, (b) strict unknown-value rejection); CONFIG_BLOCK honest absence (rides 132); spiral `PositiveLargestContour` named non-borrow. 04/05 annotations ride the packet's Step 4. Packet number `295` → `296` derived from disk; DEV-188 first collision-free (LOG max 171, drafts 172–187). No queue-count change; no new fog, nothing out of scope.

### Gates

- Packet preflight PASS (see above); no tree code changed, so no build/clippy/test gate applies beyond the packet's own AC matrix (which re-dispatches on implementation).
