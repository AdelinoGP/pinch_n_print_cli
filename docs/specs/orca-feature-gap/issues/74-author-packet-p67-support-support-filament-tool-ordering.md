# 74 — Author packet P67 — Support / Support filament — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7ccc3b68ffesnQ5JMHpUpR46c) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P67 — Support / Support filament — tool-ordering** — 1 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P67 — Support / Support filament — tool-ordering):

`support_interface_not_for_body`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet** (map's Packets-are-for-complex-implementation-only rule: the whole remaining work is declaring one bool and wiring it to a decision point that already exists).

Claim-time re-sizing: Tier B held as effort but not as packet — the tier-table `tool-ordering` owner is wrong for this tree (ticket-27 hazard: no `ToolOrdering` module exists; per-layer tool order is assembled in `assemble_ordered_entities_with_support_identities`, `crates/slicer-runtime/src/layer_executor.rs`, via ticket-38's `SupportToolSelection` seam). Canonical grounding (`PrintConfig.cpp` coBool default true; `ToolOrdering::collect_extruders`, `GCode::process_layer` in `ToolOrdering.cpp`/`GCode.cpp`) is live, so the key stays in scope (rule 3 pass).

What changed (no packet number taken):
- `SupportToolSelection` gains `support_interface_not_for_body: bool` (default true), parsed in `parse_support_tool_selection` (`crates/slicer-runtime/src/run.rs`) from `Bool`/`Int`/`String` with absent-means-true; `loader.rs` object-metadata arm passes the key through `coerce_string_to_config_value`.
- When the gate is on, the interface filament is explicitly configured (`support_interface_filament` Int ≥ 1), the body collides with the interface tool, and `tool_count > 1`, the body advances to the smallest configured tool that is not the interface tool — the PnP simplification of canonical's `get_next_extruder` flush-volume ordering (no flush matrix exists here; recorded in code comment). A default profile never changes assignment.
- `WipingExtrusions::mark_wiping_extrusions` override arm has no port analogue — named non-borrow, not implemented.
- No manifest / `ResolvedConfig` / CONFIG_BLOCK work (ticket-46 runtime-only precedent: support selectors never were module keys).

Tests (behaviour change at non-default value):
- `run::tests::support_interface_not_for_body_defaults_true_and_advances_colliding_body` — default true, explicit-false spellings honoured without advancement, colliding explicit pair advances 1→0, implicit/single-tool never advances.
- `layer_executor::tests::support_interface_not_for_body_moves_colliding_body_off_interface_tool` — parse→assemble: colliding pair yields body/interface tools `[0, 1]` by default and `[1, 1]` with explicit false.

Gates: `slicer-runtime --lib` 109/109, `slicer-model-io --lib` 17/17, `slicer-runtime --test integration structured_support_identity` 1/1, `slicer-runtime --test contract` 296/296, `cargo xtask check-literals` 0 violations, `cargo clippy --workspace --all-targets` clean. `cargo xtask build-guests --check` reports STALE (exit 1) for all guests — pre-existing and outside this ticket's closure (runtime + model-io are in no guest's dependency closure; no guest-relevant test failed). 04/05 rows annotated; no queue-count change; no new fog, nothing out of scope.
