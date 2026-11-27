# 82 — Author packet P75 — Quality / Bridging — bridge-over-infill

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260909_P75) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P75 — Quality / Bridging — bridge-over-infill** — 3 keys, Tier B new logic, owner bridge-over-infill. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P75 — Quality / Bridging — bridge-over-infill):

`dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed without a packet and without a code change: P75's three keys are
already live.** Claim-time re-sizing (per the map's "Packets are for complex
implementation only" rule — the ticket title is a rotted ledger fact, not an
instruction) found every key declared, consumed at a behaviour-changing
decision point, and pinned at a non-default value — the ticket-57 shape. The
feature itself landed off-map as bridge-parity packets 233 / 234 / 234a
(`implemented`; 234a's closure revision absorbed the F4 coverage/anchoring
machinery), with ticket 35's follow-up giving internal solid infill its own
fill domain. No packet number consumed (next free stays 302, derived from
disk at the next authoring).

Canonical grounding (fresh read of the designated oracle
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, never the GUI
fork): all three keys pass rule 3 — live in canonical's slicing pipeline
(`PrintObject.cpp`), stay in scope:
- `dont_filter_internal_bridges` — `PrintConfig.cpp` coEnum, default
  `ibfDisabled` (values `disabled`/`limited`/`nofilter`); reads at
  `bridge_over_infill`'s `expansion_multiplier = 3` (`ibfDisabled`, else 1)
  and the `ibfNofilter` expand-bypass vs the `9·spacing²` partial-support
  area gate. **Live** in this tree: the prepass
  `gate_internal_bridge_sites`
  (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) reads the key
  from `region_map.config_for(...).extensions` (unwired-module-key routing)
  and selects multiplier 3/1; `unsupported_span_areas` + 
  `qualify_internal_bridge_surface`
  (`crates/slicer-core/src/algos/bridge_over_infill.rs`) implement the shrink
  and the partial gate verbatim (RC-A fills-as-initial fix landed); the
  `InfillPostProcess` construction arm
  (`crates/slicer-runtime/src/layer_executor.rs`) re-reads it as the short-line
  filter. Defaults: port bool `false` = canonical `ibfDisabled` (full
  filtering). Pinned at a non-default value by
  `partial_support_area_gate_and_nofilter_bypass`
  (`crates/slicer-core/tests/bridge_support_gating_tdd.rs`: `false` → None,
  `true` → Some) plus both calicat e2e rows (`dont_filter...: false`).
- `internal_bridge_angle` — `PrintConfig.cpp` coFloat, default `0.0`,
  min 0 / max 180; read at the construction site (`> 0` gate, absolute /
  `+ model rotation` / `relative_bridge_angle`-relative arms). **Live**:
  the construction arm reads it (`angle_override`) into
  `determine_bridging_angle`, whose `override_deg > 0.0` arm returns it
  verbatim — checked by `bridging_angle_override_is_exactly_45_degrees`.
  Default `0.0` = canonical `0.` = automatic; min/max match (manifest
  `[0, 180]`). The `relative_bridge_angle` + `align_infill_direction_to_model`
  companion arms are absent from the gap source, the queue, and the tree
  (verified: no hits) — named non-borrows, not gaps in this ticket.
- `enable_extra_bridge_layer` — `PrintConfig.cpp` coEnum, default
  `eblDisabled` (values `disabled`/`external_bridge_only`/`internal_bridge_only`
  /`apply_to_all`); internal-side read at the `eblApplyToAll ||
  eblInternalBridgeOnly` duplicate-material gate. **Live**: the prepass
  carrier-free duplicate pass (`extra_bridge_layer_emission_semantics.rs`:
  default-off byte-stable, enabled duplicates the layer above) plus the
  234a handoff's recorded semantics (duplicate at parent + 90° intent via the
  existing anchor-derived construction). Port bool `false` = canonical
  `eblDisabled`.

Two deliberate simplifications, recorded here not as new deviations (no packet,
no DEV row opened) but as observations on 234a's accepted implementation:
(a) **bool-vs-enum on two keys.** Canonical declares both as 3-/4-value
enums; the port declares both as bools. `dont_filter` collapses
`limited`+`nofilter` (multiplier 1 either way) and loses the limited/no-filter
distinction at the expand-bypass; `extra_bridge_layer` collapses the external
gate (canonical's other read site, the stInternalAfterExternalBridge two-phase
split — external bridges are a different seam, owned by packet 235's
orientation work) into off. (b) **No `ResolvedConfig` field for any of the
three** — module-manifest + extensions routing, like the ticket-34 siblings;
the generated reference already lists all three under `rectilinear-infill`.

Owner correction (ticket-27/35 hazard): the tier table's "bridge-over-infill
(slicing stage, PrintObject.cpp)" owner is accurate but is a seam, not a
module — the decision points live in the host prepass qualification +
`InfillPostProcess` construction (the ticket-36 precedent), never in a guest
module. The `rectilinear-infill` manifest rows for all three keys are
parse-only (the module's own comment says the post-process seam consumes
neither channel — `ConfigView::from_declared` whitelist shape, ticket-34);
the module's `Layer::Infill` arm holds no InfillPostProcess claim (only
`infill-linker` binds that stage), so the rows cannot be live decision
points, only schema witnesses.

04/05 rows annotated (P75 → already-live, no packet; split line unchanged);
no queue-count change. Gates for a no-code-change closure, all re-run this
session: `bridge_support_gating_tdd` 8/8 + `bridge_over_infill_tdd` 8/8
(`-p slicer-core --features host-algos`), `extra_bridge_layer_emission_semantics`
2/2 + `region_partition_tdd` 15/15 (integration), both calicat e2e rows green
(after a `cargo build -p pnp-cli` refresh — the stale-binary failure is the
known narrow-run harness shape, not a regression).
