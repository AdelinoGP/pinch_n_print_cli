# 83 — Author packet P76 — Multimaterial / Multimaterial advanced — classic-perimeters

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260909_P76) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06, 102, 107
Map: ../map.md

## Question

Author the spec packet for **P76 — Multimaterial / Multimaterial advanced — classic-perimeters** — 1 keys, Tier B new logic, owner classic-perimeters. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P76 — Multimaterial / Multimaterial advanced — classic-perimeters):

`interface_shells`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owner confirmed but narrowed to the prepass seam, packet authored, no re-file.** The key lives in canonical's slicing pipeline (`PrintObject.cpp::detect_surfaces_type` same-region-vs-collective upper/lower arms plus the extra non-bridging bottom, verified against the oracle at claim time; `PrintConfig.cpp` coBool default `false`, print-object scope) and is zero-occurrence as behaviour in this tree (verified by tree-wide `rg` over `crates/` + `modules/` + `xtask/` — the one `crates/` hit is the `ORCA_CONFIG_PADDING` spelling twin, rule 2 not evidence; no manifest declaration, no `ResolvedConfig` field; no draft packet owns the decision — the 299/300 mentions are P76's handoff non-borrows). Ticket 04's "classic-perimeters" owner is accurate but is a seam, not a module: the decision point lands in the host prepass `compute_region_updates` Pass-1 neighbour source fed by a `resolve_shell_counts`-pattern resolver read (the ticket-36 precedent) — the perimeter modules inherit it through the buckets they already consume, never module-side.

Packet `docs/spec_packets/301-interface-shells-classic-perimeters/` authored (`draft`), **preflight PASS** (S0–S8, all tree-verified: 8/8 pre-existing symbols resolved with file evidence; DEV-192 verified next-free — LOG max DEV-171, absent from all drafts; no schema bump — no IR/WIT change; ADR-0062/0063 conformance, not amendment; `--lib` test homes need no aggregator registration; two self-corrections — the packet-299 regen-only precedent corrected to lock-arm + TOML row after the doc-gen sourcing dispatch proved macro rows never reach doc-15, and the fictional D-152 citation dropped for the in-module divergence note). Membership 1/1, none shed, none returned: the flag switches the neighbour source between same-timeline polys (`true` = self-standing, pre-packet shape) and the collective all-timelines union (`false` = canonical default); defaults ARE identity on single-body prints (AC-4 pins the one-timeline fixture byte-identical — inverse of 299's emitting default). Canonical scalarity IS held (scalar coBool — no ticket-125 vector arm); no numeric range rejection (ticket-113 rule; wrong-variant spelling is the typed `extract_bool` mismatch); locked-path conformance pinned (AC-N1); padding twin untouched, shadowed via the `to_config_map` arm (284–286 precedent, spellings ride 132); rule 4 does not fire (in-seam source switch, `seam_position` precedent). Four divergences in new **DEV-192**: the `!spiral_mode` conjunct is not borrowed (`spiral_mode` unimplemented), the `discover_vertical_shells` collective merge is not borrowed as a second site (299 owns that file's stage list), the perimeter-side same-region masks are inherited not re-armed, and `Print.cpp` reslice-invalidation rides ticket 124. No code change; no queue-count change; no new fog, nothing out of scope.
