# 20 — Author packet P13 — Support / Support — support-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa48e9251ffeqQKYlu3iZ46OlF) — claimed 2026-09-01, resolved 2026-09-01
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P13 — Support / Support — support-planner** — 12 keys, Tier A plumbing, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P13 — Support / Support — support-planner):

`enforce_support_layers`, `raft_first_layer_expansion`, `support_bottom_z_distance`, `support_critical_regions_only`, `support_expansion`, `support_object_first_layer_gap`, `support_object_xy_distance`, `support_remove_small_overhang`, `support_style`, `support_threshold_angle`, `support_threshold_overlap`, `support_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/265-support-support-keys/` authored (`draft`), preflight **PASS**
(S0–S8 + AC-runnable + Doc-Impact all green, report in the packet dir). Packets 16/17's
working tree was committed first (both were resolved-but-uncommitted from a prior session).

Authoring-time re-derivation split the 12 Tier-A keys into four states (ticket 18's
"re-derive, don't trust" applied again):

- **Six already wired + canonical-faithful, pinned not changed** —
  `support_object_xy_distance` (0.35 / [0, 10] declared in both planner manifests,
  consumed by both planners' clearance passes), `support_threshold_angle` (host typed
  30.0, consumed by `resolve_contact_params`; traditional-side manifest declaration +
  `support_overhang_angle` alias; tree-side asymmetry recorded, not duplicated),
  `support_style` (tree-side canonical 7-value enum; **traditional-side string → enum
  type correction** lands in this packet), `support_type` (the family selector —
  functional via raw config + `slicer-ir::SupportType` + `select_support_family`, but
  manifest-less: **declared as the canonical 4-value enum in both planner manifests**,
  giving global-path enum enforcement), `support_expansion` (0.0 host typed, Step-6 XY
  expansion), `support_threshold_overlap` (50% percent host typed, zero-angle branch;
  declared percent `"50%"` [0,100] in both planners).
- **One wired by this packet** — `enforce_support_layers`: the decision point existed
  (`force_support = params.layer_id < params.enforce_support_layers` in
  `slicer-core`'s `overhang_annotation.rs`, geometry arms already in
  `support_overhang_detection_tdd.rs`) but `resolve_contact_params` hardcoded `0` with a
  "no production config source yet" comment. The packet reads the typed CLI-bound field
  (default 0 → identity); declared int 0 [0, 5000] in both planners. Canonical's
  tree-family nuance (`-0.15 × extrusion_width` inside the enforced band) recorded as a
  divergence note, not implemented.
- **Five re-adjudicated declared-with-gap** — `raft_first_layer_expansion`
  (zero-occurrence everywhere; canonical default **2.0**, not the 3.0 an older BBS
  comment hints at — fresh read; declared in `tree-support-planner.toml` only, AC-N2
  pins the traditional absence) and `support_bottom_z_distance` /
  `support_critical_regions_only` / `support_object_first_layer_gap` /
  `support_remove_small_overhang` (host-declared ResolvedConfig fields with canonical
  defaults, zero production read sites; declared in both planner manifests with the
  canonical consumers recorded for future geometry packets).

Manifest homes: 9 net-new tables in `tree-support-planner.toml`, 8 in
`traditional-support-planner.toml`, `support_style` enum type-correction in
`traditional-support.toml` — no `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` twins
(AC-5), deviation block stays at **26** data rows (all declared defaults canonical —
re-measured), zero user rulings required. Guards: three net-new schema guards + a
net-new planner non-perturbation harness (filenames avoid packets 253/260/261's planned
guards); scheduler bounds/enum arms (+4) and CONFIG_BLOCK arms in the existing
integration binaries; the one behavior change is identity at defaults. Owner correction
recorded (decision points span host analysis + scheduler + planners; tier rows ride this
closure). No new fog graduated — the four divergence notes are packet records, and the
traditional-raft fog item's P13 reference now resolves to a declaration while the
traditional-raft-handling question stays open. **P14 (ticket 21) is the next unblocked
queue head** (blocked by 06 + 106, both resolved).
