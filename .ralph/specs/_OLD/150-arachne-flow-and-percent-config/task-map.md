# Task Map: 150-arachne-flow-and-percent-config

This packet does NOT correspond to a `docs/07_implementation_status.md` task ID
(`task_ids: none` in `packet.spec.md`) — the audit
(`docs/18_arachne_parity_audit.md`) surfaced gaps G4/G5/G6, and the three
red-to-green tests in `crates/slicer-runtime/tests/arachne_parity_gaps.rs`
(`..._wall_gap_uses_flow_spacing_not_width`,
`..._thick_bridges_flow_factor_not_stubbed_to_one`,
`..._percent_config_type_for_arachne_keys`) are the runnable fingerprint.
`docs/07_implementation_status.md` gets no new row for this packet, matching
precedent set by packets 148/149.

| Audit gap | Packet step | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- |
| G6 (no percent config type) / D-104h | Steps 1-3 | `config.wit` variant, `slicer-macros` adapter arms, `slicer-ir::ConfigValue`/`get_abs_value`, `slicer-scheduler/{manifest.rs,config_resolution.rs}`, `arachne-perimeters.toml` retype (3 keys) | `PrintConfig.cpp:1498-1511,7169-7178,7217-7226` | M | AC-1/AC-2/AC-N1/AC-N2; undocumented scope extension found here — host `ConfigValueStorage` (`crates/slicer-wasm-host/src/host.rs`) needed `Percent`/`FloatOrPercent` variants for the value to survive host→guest delivery (see `design.md` Scope note) |
| G4 (flow spacing not wired) / D-105 | Step 4 | `arachne-perimeters/src/lib.rs` (`layer_height`/`nozzle_diameter` reads, `line_width_to_spacing` wiring), `arachne-perimeters.toml` | `PerimeterGenerator.cpp:2129,2172-2173` | M | AC-3; AC-6 lock watch (moves wall positions); also uncovered and fixed a `precise_outer_wall` spacing bug (outer wall paired with inner wall's spacing) alongside the main fix — see D-105 row in `docs/DEVIATION_LOG.md` |
| G5 (thick_bridges stubbed to 1.0) / D-104g | Step 5 | `crates/slicer-core/src/flow.rs` (`bridging_flow` round-section factor), `arachne-perimeters/src/lib.rs` call site | `Flow.hpp:106`, `Flow.cpp` bridging_flow, `LayerRegion.cpp:31-50,135` | M | AC-4; undocumented scope extension found here — the new signature also required updating the `classic-perimeters` caller (new `layer_height` param threaded through `emit_walls`), so `classic-perimeters.toml` gained `layer_height` too (see `design.md` Scope note) |
| (no gap ID; adjacent dead-read fix, AC-5) | Step 6 | `classic-perimeters.toml` (`nozzle_diameter` registration) | none | S | AC-5; `layer_height` also lands in this manifest but belongs to Step 5's scope extension above, not Step 6 — corrected in the manifest comment (see `classic-perimeters.toml:246-255`) |

**Closed deviations:** D-105 (flow spacing), D-104g (thick_bridges stub), and
D-104h (no percent config type) all close in this packet — see their rows in
`docs/DEVIATION_LOG.md` (dated 2026-07-10) for the full closure text.

The Context cost column copies the per-step estimate from
`implementation-plan.md`. Aggregate is M (Steps 1-5 are M, Step 6 is S; no
single step is L). The packet does not need to split before activation.
