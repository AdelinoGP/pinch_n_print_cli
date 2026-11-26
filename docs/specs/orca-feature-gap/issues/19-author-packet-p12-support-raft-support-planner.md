# 19 — Author packet P12 — Support / Raft — support-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa5ee413affeJ8SsO4RQ4BfdD6) — claimed 2026-08-31
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P12 — Support / Raft — support-planner** — 2 keys, Tier A plumbing, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P12 — Support / Raft — support-planner):

`raft_contact_distance`, `raft_expansion`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Resolved 2026-08-31 — packet `docs/spec_packets/261-raft-keys/` authored
(`draft`), preflight **PASS** (0 blockers, 0 high; report in `preflight-report.md`).

Grounding re-derived the tier picture from code (map rule: verify, don't trust):

- **Both keys zero-occurrence, re-adjudicated declared-with-gap.** `raft_contact_distance`
  and `raft_expansion` appear nowhere in `modules/`, `crates/`, `xtask/`, or
  `docs/15_config_keys_reference.md`. No raft geometry generator exists in-tree: the
  `com.core.raft-default` module of draft packet 240-support-raft (support-families plan)
  is unimplemented, and the `RaftPlan` record (`crates/slicer-ir/src/slice_ir.rs`
  `RaftPlan`) carries only layer counts. Canonical consumers pinned: `Slicing.cpp`
  `SlicingParameters::SlicingParameters` (raft Z-gap → `gap_raft_object` →
  `object_print_z_min`; forced to 0 when `raft_z_gap == 0.0 || zero_topZ_contact`),
  `SupportMaterial.cpp` `generate_contact_polygons` (layer_id==0 XY expansion:
  `raft_expansion > 0 ? expand(overhang_polygons, scaled(raft_expansion)) :
  overhang_polygons`), `TreeSupport3D.cpp` `generate_raft_contact` /
  `finalize_raft_contact`, `GCode.cpp` `_print_z` support-gap warning, `PrintObject.cpp`
  `invalidate_state_by_config_options`. The reference tooltip's "ignored for soluble
  interface" is **GUI-only** (`ConfigManipulation.cpp` disables the field) — no slicing
  branch; recorded, not ported.
- **Owner confirmed, narrowed to one claim holder.** The tier table's owner
  `support-planner` is right, but only `tree-support-planner` has raft surface (the raft
  config cluster — `support_raft_layers` / `raft_first_layer_density` / `base_raft_layers`
  / `interface_raft_layers` — read in `from_config`, plus the configuration-only
  `RaftPlan` emission when `support_raft_layers > 0`). `traditional-support-planner`
  declares no raft keys and emits no `RaftPlan`; the traditional-family geometry module
  has no raft handling either — raft is tree-family-only in this port. The packet
  declares in `tree-support-planner.toml` and pins the traditional omission (AC-N2); the
  04 owner column stays unchanged.
- **Canonical defaults/bounds adopted outright.** Both keys declared float with
  canonical defaults (0.1 / 1.5) and canonical bounds (min 0.0, no max — the in-tree
  `max_bridge_length` table is the no-max float precedent). Net-new declarations create
  no deviation rows (deviation block stays 27) and no declared-bounds divergence.
- **No user rulings required** — nothing to align (keys are net-new), no sentinel
  question, no divergence to keep or drop.
- **Packet-240 relationship recorded, not deferred.** 240 (draft) plans to declare these
  keys in `com.core.raft-default`'s manifest and wire them to geometry; its AC-5
  requires a written wire-or-record decision for the four support-module manifests —
  this packet's declarations + the traditional omission pin are that record's input.
  Same-key-in-two-modules is the packet-260 spacing-key precedent.
- **CONFIG_BLOCK verified clean**: neither key rides `SUPPORT_CONFIG_DEFAULTS` or
  `ORCA_CONFIG_PADDING` (`serialize_config_block`, `crates/slicer-gcode/src/serialize.rs`
  — the padding list's `("raft_layers", "0")` is the canonical layer-count key, not these
  two), so at defaults zero `raft_*` lines appear; explicit values reach the block once
  (AC-4).
- Test binaries for the AC arms verified to exist and drive the asserted behavior,
  including empirical auto-discovery confirmation for the net-new guard binary
  (`cargo test -p tree-support-planner --test orca_parity_tdd --no-run` builds without a
  `[[test]]` entry); the module needs the `toml = "0.8"` dev-dependency (add-if-absent)
  for the schema guard.

Fog graduated: the traditional family's raft absence (no raft keys, no `RaftPlan`, no
raft geometry in `traditional-support-planner` / `traditional-support`) is a port state
recorded in the map's Not-yet-specified — future raft-key packets (P13) and the raft
geometry work (packet 240) must decide whether the traditional family gains raft
handling. 04-asset-tier-assignment.md rows: unchanged (owner `support-planner` confirmed;
the A → declared-with-gap re-adjudication is recorded here and in the packet, per ticket
12/13/14/18 precedent).

Next frontier: ticket 20 (P13 — Support / Support) is unblocked (06, 104 resolved), as
are the remaining rename tickets 106/107/108.

## Answer
