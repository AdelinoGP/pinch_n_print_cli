# 18 — Author packet P11 — Support / Interface — support-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa6101498ffet6lK3xqGwUvC6r) — claimed 2026-08-31
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P11 — Support / Interface — support-planner** — 4 keys, Tier A plumbing, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P11 — Support / Interface — support-planner):

`support_bottom_interface_spacing`, `support_interface_loop_pattern`, `support_interface_pattern`, `support_interface_spacing`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Resolved 2026-08-31 — packet `docs/spec_packets/260-support-interface-keys/` authored
(`draft`), preflight **PASS** (0 blockers, 0 high; report in `preflight-report.md`).

Grounding re-derived the tier picture from code (map rule: verify, don't trust):

- **Owner correction.** The tier table's owner `support-planner` is the claim held by
  `tree-support-planner` + `traditional-support-planner`, but neither planner reads
  interface configuration; the four keys' decision points live in `tree-support` and
  `traditional-support` (both read the two spacing keys in `from_config` and derive the
  interface scan-line pitch in `pitches_mm` via `slicer_core::support_regularize` —
  formula canonically identical to `SupportParameters::SupportParameters`'
  `top_interface_density = min(1, flow.spacing()/spacing)`). The packet declares in the
  decision-point modules; the 04 owner row correction rides this closure.
- **Two keys wired + verified** (Tier A confirmed): `support_interface_spacing` and
  `support_bottom_interface_spacing` are already declared in both manifests and consumed.
  Canonical read overturned the port's top default: Orca's is **0.5**, not 0.4 (the port's
  own comments claimed 0.4 was Orca — mis-derived; packet 238c already fixed the bottom key
  to 0.5). **User ruling: align 0.4 → 0.5** — removes the two known doc-15 deviation rows
  (deviation block 27 → 25 data rows, re-measured at authoring). Second canonical finding:
  Orca's `support_bottom_interface_spacing` has **no -1 sentinel** (min 0; the canonical
  sentinel belongs to `support_interface_bottom_layers`) — the port's negative-mirrors-top
  branch is a PnP extension. **User ruling: keep as recorded divergence**; AC-3 pins the
  mirror as a witness, AC-4 keeps `-1.0` legal in the bounds index, manifest comments
  document it.
- **Two keys zero-occurrence, re-adjudicated declared-with-gap** (tier A row corrected):
  `support_interface_pattern` (coEnum auto/rectilinear/concentric/rectilinear_interlaced/
  grid) and `support_interface_loop_pattern` (**coBool** default false — canonical type
  correction) appear nowhere in `modules/`, `crates/`, `xtask/`. Canonical consumers
  pinned: the `contact_fill_pattern` branch order in `SupportParameters::SupportParameters`
  (grid→ipGrid, interlaced→ipRectilinear, auto-with-zero-gap∥concentric→ipConcentric,
  density>0.95→ipRectilinear, else ipSupportBase — a `FillSupportBase : FillRectilinear`
  filler at `spacing/density`, the same rectilinear family as the port's scan-line, so the
  sparse-density default is structurally faithful) and `LoopInterfaceProcessor`'s
  `n_contact_loops` in `generate_support_toolpaths` (plus
  `SupportMaterial::has_contact_loops`). Both declared with-gap, unread in source, AC-N1
  pins non-perturbation.
- **No deviation rows created; none filed with the human beyond the two rulings above.**
  Declared-bounds divergences recorded, not changed (canonical has no max; port keeps
  `max = 2.0`; bottom keeps `min = -1.0`).
- **CONFIG_BLOCK verified clean**: none of the four keys rides `SUPPORT_CONFIG_DEFAULTS` or
  `ORCA_CONFIG_PADDING` (`serialize_config_block`, `crates/slicer-gcode/src/serialize.rs`),
  so at defaults zero `support_interface_*` lines appear; explicit values reach the block
  once (AC-5).
- Test binaries for the AC arms verified to exist and drive the asserted behavior,
  including empirical auto-discovery confirmation for the net-new guard binaries; both
  modules need the `toml = "0.8"` dev-dependency (add-if-absent) for the schema guards.

Fog graduated: the two declared-with-gap decision points (pattern dispatch +
angle specialization; contact-loop processor) are queue-sized geometry work — recorded in
the map's Not-yet-specified (see map update). 04-asset-tier-assignment.md rows: P11's four
keys correct from owner `support-planner` to the actual decision-point modules, and the two
pattern keys re-adjudicated A → declared-with-gap (04's tier column is unchanged — the
re-adjudication is recorded here and in the packet; the table's owner column correction is
this closure's record, per ticket 12/13/14 precedent).

Next frontier: ticket 19 (P12 — Support / Raft) is unblocked (06, 104 resolved), as are the
remaining rename tickets 106/107/108.
