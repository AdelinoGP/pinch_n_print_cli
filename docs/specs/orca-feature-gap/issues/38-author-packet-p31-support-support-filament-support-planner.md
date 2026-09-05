# 38 — Author packet P31 — Support / Support filament — support-planner

Type: task
Status: resolved
Assignee: —
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P31 — Support / Support filament — support-planner** — 2 keys, Tier B new logic, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P31 — Support / Support filament — support-planner):

`support_filament`, `support_interface_filament`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

No new P31 packet is required. Claim-time re-sizing found that the two-key
feature was already implemented by packet
[`172-mm-e2e-and-object-keys`](../../../../docs/spec_packets/_OLD/172-mm-e2e-and-object-keys.md),
which closed `TASK-210` and `TASK-211`.

- `parse_support_tool_selection` in `crates/slicer-runtime/src/run.rs` reads
  `support_filament` and `support_interface_filament`, rebases Orca's 1-based
  values to 0-based runtime tool indices, and supplies the global
  `SupportToolSelection`.
- `PipelineConfig::support_tools` and
  `assemble_ordered_entities_with_support_identities` in
  `crates/slicer-runtime/src/pipeline.rs` and
  `crates/slicer-runtime/src/layer_executor.rs` route support/base paths to
  `support_tool` and interface/ironing paths to `interface_tool`.
- `object_metadata_to_config_data` in
  `crates/slicer-model-io/src/loader.rs` preserves both keys and applies the
  same 1-based-to-0-based conversion for 3MF object metadata.
- Existing parser, loader, runtime, and real-fixture coverage includes
  `mm_support_filament_real_fixture` in the `slicer-runtime` `e2e` test binary;
  the fresh targeted run passed (`1 passed; 0 failed`).

The original owner/tier is stale: this is global runtime routing with existing
decision points, not new support-planner logic. The canonical backlog already
records `TASK-210`/`TASK-211` as done via packet 172. Therefore this queue
ticket closes without authoring another packet or changing production code.
