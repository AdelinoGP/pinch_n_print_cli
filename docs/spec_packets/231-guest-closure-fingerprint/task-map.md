# Task Map: 231-guest-closure-fingerprint

Single task ID, five steps: the crosswalk exists because `TASK-342` spans four distinct code surfaces (the xtask fingerprint model, the xtask pnp_cli gate, a `crates/` rustdoc under an ADR obligation, and the backlog ledger), and because the packet sits mid-queue in an approved four-packet plan whose upstream exports must stay traceable.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-342` | `Step 1` | `docs/specs/guest-freshness-artifact-verification-plan.md` (C2, C8, R5-6) | `xtask/src/build_guests.rs` (`#[cfg(test)] mod tests`) | none | `S` | Twelve red tests bind the closure API's shape before it exists; the compile failure naming `guest_closure_input_paths` is the proof they are not vacuous |
| `TASK-342` | `Step 2` | `docs/specs/guest-freshness-artifact-verification-plan.md` (C2, C5 as corrected here, C8, R5-6); `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` (read-only) | `xtask/src/build_guests.rs` — add `ClosureCache`, `ClosureError`, `path_dep_manifests`, `guest_closure_input_paths`; extend `guest_input_paths` to charge `<module>/*.toml`; delete `shared_input_paths`, `compute_shared_freshness`, `stage_wit_snapshot` and their two dedicated tests; re-thread `compute_guest_freshness`, `is_stale`, `build_one`, `CheckContext` | none | `M` | Proves AC-1..AC-9, AC-15 and AC-N1..AC-N4; AC-15 closes the C5 module-manifest coverage hole (C5's `include_str!` justification is false — the only such `include_str!` is test-only); AC-N3 is the packet's reason to exist — an out-of-closure edit must not mark a guest stale |
| `TASK-342` | `Step 3` | `docs/specs/guest-freshness-artifact-verification-plan.md` (C7); `CLAUDE.md` §"Test Discipline" | `xtask/src/test.rs` — `ensure_pnp_cli_fresh_with` loses its mtime gate; `newest_mtime_in` deleted; AC-10 test added | none | `S` | Proves AC-10 and AC-11; `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` must keep passing unchanged |
| `TASK-342` | `Step 4` | `docs/adr/0054-host-side-test-support-crate.md` §Decision rules 1-5; `CLAUDE.md` §"In-Tree Citation Style" | `crates/pnp-cli-locator/src/lib.rs` — `staleness_reason` rustdoc only | none | `S` | Proves AC-12 and AC-13. This packet **owns** the ADR-0054 obligation and discharges it by conformance (rustdoc update), not by amendment; packet 232 must not touch ADR-0054 |
| `TASK-342` | `Step 5` | `docs/07_implementation_status.md` §"### Workstream 5 — Governance and closure drift" (delegated append); `CLAUDE.md` §"Ledger Facts Must Be Re-derived" | none (ledger + workspace gates) | none | `S` | Proves AC-14; re-derive that `TASK-342` is still free at write time and renumber on collision |

Copied from `implementation-plan.md`: aggregate `M`, largest step `M`, no L step, no split required.

## Upstream/downstream crosswalk

| Relationship | Packet | Task ID | What crosses the boundary |
| --- | --- | --- | --- |
| Forward-dep (must be `implemented` first) | `docs/spec_packets/229-wit-verify-declaration-model` | `TASK-340` | Nothing consumed directly; reached transitively through packet 230 |
| Forward-dep (must be `implemented` first) | `docs/spec_packets/230-output-based-guest-freshness` | `TASK-341` | Consumed: `CheckOutcome`, `check_command`, `build_stale_command`, `StaleReason`, `stale_reason`, `CheckContext`, `EXIT_FRESH` / `EXIT_STALE` / `EXIT_INFRA_ERROR`, `FINGERPRINT_VERSION`. Preserved on 230's behalf: `GuestSpec.stage_id`, `parse_stage_id_from_module_manifest` (R5-4) |
| Downstream | `docs/spec_packets/232-freshness-gate-docs` | `TASK-343` | Documents the model this packet lands (dependency-closure fingerprint, code-inputs-only, unconditional pnp_cli rebuild); amends ADR-0014 and ADR-0045, **not** ADR-0054 |
