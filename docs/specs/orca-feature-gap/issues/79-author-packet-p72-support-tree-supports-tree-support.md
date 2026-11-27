# 79 — Author packet P72 — Support / Tree supports — tree-support

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7c264d5bffe8sDALV4x7aWo39) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P72 — Support / Tree supports — tree-support** — 8 keys, Tier B new logic, owner tree-support. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P72 — Support / Tree supports — tree-support):

`tree_support_angle_slow`, `tree_support_auto_brim`, `tree_support_branch_angle_organic`, `tree_support_branch_diameter_organic`, `tree_support_branch_distance_organic`, `tree_support_brim_width`, `tree_support_tip_diameter`, `tree_support_top_rate`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owners confirmed, packet authored, no re-file.** All eight keys live in canonical's slicing pipeline and are zero-occurrence as behaviour in this tree (verified by tree-wide `rg` over `crates/` + `modules/` + `xtask/` — the only hits are the non-organic classic names). The six `_organic`/tip/top/slow keys are read only by the organic engine (`TreeSupportCommon.hpp` settings constructor; tip also by `TreeSupport3D.cpp` area generation), which this port does not implement — DEV-156's Strong substitution stands and is explicitly preserved (AC-N2). The two brim keys are read by the classic engine's `TreeSupport::draw_circles`, whose local counterpart was never built — the renderer emits no brim loops at all.

Packet `docs/spec_packets/298-organic-tree-support-keys/` authored (`draft`), **preflight PASS** (S0–S8, all tree-verified: every named function/manifest row/test binary/IR identifier re-grepped, DEV-189 verified free, 238b verified `implemented`, ADR sweep clean). Membership 8/8, none shed, none returned: the six params resolve into the planner's effective branch fields behind the explicit-organic style gate (`organic_substitution_requested`, `modules/core-modules/tree-support-planner/src/lib.rs`), the two brim keys drive a new first-layer brim stage in the renderer on the same gate (the renderer must newly declare `support_style` to see it — ticket-34 whitelist lesson). Three divergences in new **DEV-189**: organic params drive the substituted Strong engine; the gate is explicit-organic only (canonical would use organic params for default/grid/snug-on-tree too — default output stays stable until the engine port); the `Print.cpp` diameter/tip cross-validations ride ticket 124. Canonical scalarity IS held (all eight scalar — no ticket-125 vector arm); no range rejection (GUI hints — saturate, AC-N1); no padding edits (rule 2); bool spelling rides 132. Rule 4 does not fire (internal style-gated selection, `seam_position` precedent). Standing approval for writing without interactive review: the wayfinder queue entry (05 P72) + this ticket + the map's execution override, the same basis as packets 276–297. No code change; no queue-count change; no new fog, nothing out of scope.
