# Criterion bench refresh

Type: task
Status: open
Blocked by: 11

## Question

Bring the seven criterion benches current and trustworthy so they can back
optimization decisions (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §7a — explicitly authorized for
refresh).

Work:

- Run each bench and establish on-disk baselines (`target/criterion` is absent
  in this tree): `polygon_ops` (slicer-core), `mesh_ops` (slicer-helpers),
  `pipeline`, `per_stage`, `wasm_modules`, `shell_classification`,
  `gate_evidence` (slicer-runtime) — invocations in `.agents/aux-commands.md`;
  `wasm_modules` needs `cargo xtask build-guests` first.
- Re-validate that each fixture does **real** work before trusting any number:
  the `shell_classification` bench has a first-hand precedent of a green bench
  measuring nothing (its earlier fixture made `difference(layer, neighbour)`
  empty so `apply_opening` short-circuited). Check `git log -p` per bench before
  trusting fixture age — several "last touched" commits were incidental sweeps
  (crate rename, clippy/literals, WIT review).
- Note `gate_evidence`'s shape (per DEV-026 it times a real `pnp_cli slice`
  subprocess against `resources/regression_wedge.stl` and measures rather than
  hard-fails).

Deliverable: refreshed baselines plus a per-bench trust verdict (fixture does
real work / measures what it claims / stale). Timing ticket — chain position 2
after [Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md).
