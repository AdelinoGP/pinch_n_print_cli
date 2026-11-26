# 37 — Author packet P30 — Support / Advanced (Support) — support-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8f232505ffeC5QpRmRCTW9VP5) — claimed 2026-09-05, resolved 2026-09-05
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P30 — Support / Advanced (Support) — support-planner** — 5 keys, Tier B new logic, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P30 — Support / Advanced (Support) — support-planner):

`bridge_no_support`, `independent_support_layer_height`, `max_bridge_length`, `support_base_pattern`, `support_base_pattern_spacing`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet** — the claim-time audit found
existing decision points for the live behavior, while `support_base_pattern`
required a separate holder/module seam.

### Direct implementation

- `bridge_no_support` now reaches `resolve_contact_params` from the typed
  `ResolvedConfig` field.
- Each `SlicedRegion::bridge_areas` now reaches the existing bridge-removal
  decision point in `detect_support_contacts`. A producer regression test
  compares identical bridge geometry with the flag disabled and enabled.
- `independent_support_layer_height`, `max_bridge_length`, and
  `support_base_pattern_spacing` were already covered by existing packet work;
  no duplicate implementation was added.
- Under the Q3 holder-only ruling, the dead `support_base_pattern` enum
  declaration, 3MF object-metadata alias, planner field, and
  `traditional-base-pattern` capability label were removed. The actual
  algorithm holder/module work is returned to [ticket 135](135-author-packet-support-base-pattern-holder.md).

### Verification

The focused producer, core support-contact, model-I/O, and traditional-planner
tests passed. `cargo check --workspace --all-targets`, workspace clippy with
`-D warnings`, `cargo xtask check-literals`, generated-config documentation
validation, and the rebuilt guest freshness check also passed.
