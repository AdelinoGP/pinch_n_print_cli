# Closure Log — Packet 112 (arachne-extrusion-and-wireup)

## M2 Closure Ceremony (T-234) — GREEN
- `cargo xtask test --workspace --summary`: **VERDICT: PASS**, 0 failed across the workspace (127 test binaries). Integration binary 216 passed / 0 failed (loader source-guard fixed).
- `cargo test -p slicer-core --features host-algos`: 310 passed / 0 failed. The host-algos-gated Arachne algorithm tests (centrality, bead_count, propagation, generate_toolpaths, arachne_pipeline, thin-wall widening) genuinely EXECUTE — a default `cargo test --workspace` would silently skip them (feature-gated), so they are run explicitly as part of the ceremony.
- `cargo xtask build-guests --check`: CLEAN (31 guests). `cargo clippy --workspace --all-targets -- -D warnings`: clean.

## Schema version
- `CURRENT_SLICE_IR_SCHEMA_VERSION`: 4.6.0 -> 4.7.0 (additive; new `ExtrusionLine`/`ExtrusionJunction` IR with `serde(default)`).
- Downstream test assertions that pinned the old 4.6.0 were updated to 4.7.0: `crates/slicer-ir/tests/ir_tests.rs`, `crates/slicer-ir/tests/material_boundary_widening_tdd.rs`. Historical/version-history comments left as-is.

## Fixtures / goldens
- ALL Arachne unit + parity fixtures are SELF-CAPTURED regression baselines (this repo has no OrcaSlicer binary/oracle; matches the existing `perimeter_parity.rs` convention). Independent OrcaSlicer geometric parity is DEFERRED — see `D-112-SELFCAPTURED-BASELINES`.
- `toolpaths_tapered_wedge.json` re-recorded once, for the Step-9 width-fidelity rework.
- cube_4color Arachne: a NEW self-captured STRUCTURAL test (per-color fragmentation via `wall_generator=arachne`); there was no pre-existing P109 `cube_4color_orca.gcode` reference to reuse (the packet's original claim was incorrect). Arachne walls are a junction graph (not classic's closed rings), so the test asserts per-color fragmentation + finite geometry, not self-closure.

## Architecture changes beyond the original packet design (surfaced during implementation, user-approved)
1. **WIT host-service bridge** (`generate-arachne-walls`): the WASM guest cannot call `host-algos` slicer-core (rayon/boostvoronoi are host-only, `cfg(not(wasm32))`); the real wire-up mirrors the existing `medial-axis` host service. The packet's original in-guest `from_polygons` design was infeasible. (`D-112-HOSTSVC-BRIDGE`)
2. **Strategy-faithful bead widths**: `generate_toolpaths(graph, strategy)` emits `BeadingStrategy::compute()` widths + toolpath offsets (was a geometric `2*r/bead_count` approximation that never surfaced the Widening min-width clamp or width distribution). (`D-112-TOOLPATH-WIDTH`, closed)
3. **Thin-wall widening**: wired the registered-but-dead `min_feature_size` / `min_bead_width` / `detect_thin_wall` config keys so `WideningBeadingStrategy` activates (previously a user enabling "detect thin wall" got nothing). Default unchanged (off = parity-correct). (`D-112-THIN-WALL-WIDENING`, closed)
4. **`wall_generator` config selection** (classic|arachne, default classic): closed a real PRODUCTION defect — classic + arachne both claim `perimeter-generator`, the scheduler dedup silently kept the alphabetically-first (arachne), no config selected between them, and the `incompatible-with` DAG validation ran post-dedup so it never fired. Verified via live `pnp_cli` both ways. (`D-112-WALL-GENERATOR-SELECT`, closed)
5. Test/manifest updates for arachne becoming a real (non-placeholder) module: M1 parity-harness generator routing (now via `wall_generator`), `placeholder_wasm`/core-module-count guard (19->20), and the `run.rs` loader source-guard.

## Deviations registered (docs/DEVIATION_LOG.md)
- CLOSED by P112: `D-112-WALL-GENERATOR-SELECT`, `D-112-TOOLPATH-WIDTH`, `D-112-THIN-WALL-WIDENING`.
- Justified residuals (with follow-on targets): `D-112-CENTRALITY-ADAPT` (adaptation, not literal port; bead_count per-edge not per-node), `D-112-PROPAGATION-ADAPT`, `D-112-SIMPLIFY-DP` (Douglas-Peucker vs Visvalingam), `D-112-HOSTSVC-BRIDGE`, `D-112-SELFCAPTURED-BASELINES`, `D-112-MMU-TOPOLOGY`.
- Roadmap closures (T-232, in the roadmap doc not DEVIATION_LOG.md): D-7 (ADR-0023 / P110), D-9 (T-215b / P111), D-15 (orca-mmu-perimeter-investigation.md / P105).

## Follow-ups
- True independent OrcaSlicer geometric parity for the self-captured fixtures.
- Per-node (vs per-edge) bead_count for faithful transition interpolation.
- MMU: some extrusion points land outside the naive per-face footprint on painted geometry — upstream paint_segmentation/geometry investigation (`D-112-MMU-TOPOLOGY`).
