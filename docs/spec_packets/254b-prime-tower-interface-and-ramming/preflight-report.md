# Preflight Gate: 254b-prime-tower-interface-and-ramming

Reviewed: 2026-09-02 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet · Authored under map Authoring rules 1–6 · Split half B of former packet 254

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty |
| S1 Prerequisite-status truth      | PASS | `254a` is `status: draft` and is declared a **FORWARD-DEP and activation blocker**, never a satisfied dependency; Step 0 is a hard gate that re-checks `254a`'s `status:` line and the presence of `depth_offset` on disk before any edit |
| S2 Deviation-ID conformance       | PASS | live log format is `DEV-###` (rows `DEV-157`, `DEV-158`); no `D-254b*` token exists in the log; `D-254b-*` are declared packet-local labels and Step 8 re-derives real `DEV-###` IDs at write time |
| S3 Schema-version computed        | PASS | no `*_SCHEMA_VERSION` pinned; the packet states no IR schema bump is required. The one hardcoded count it does pin — the core-module count `23 → 24` — is a live tree fact, re-derived at Step 5 against `manifest_ingestion_tdd.rs` rather than assumed |
| S4 ADR slot allocation            | PASS | no new ADR authored; `docs/adr/` runs 0001–0063, untouched |
| S5 Shipped-symbol existence/shape | PASS | verified in tree: `GCodeCommand::Move { role: ExtrusionRole, .. }`, `GCodeCommand::ToolChange { after_entity_index, from, to }`, `GCodeCommand::Temperature { tool: u32, celsius: f32, wait: bool }`, `ExtrusionRole::{WipeTower, PrimeTower, Ironing}`, `GcodeOutputBuilder::push_temperature(tool, celsius, wait) -> Result<(), String>`, `run_gcode_postprocess(&self, &[GcodeCommand], &mut GcodeOutputBuilder, &ConfigView)` (default-implemented trait method), `GcodeFlavor::set_temperature`, `discover_guests` / `guest_input_paths` (`xtask/src/build_guests.rs`), the `23`-core-module assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`, `machine-gcode-emit`'s `PostPass::GCodePostProcess` stage id |
| S6 WIT/IR identifier drift        | PASS | no WIT change claimed or permitted. `push-raw` exists in `crates/slicer-schema/wit/deps/postpass-gcode-postprocess/`, confirming the postprocess world is the command-bearing one; the packet uses `push_temperature`, whose `GCodeCommand::Temperature` marshalling is plumbed through `crates/slicer-wasm-host/src/dispatch.rs` and `marshal/native.rs` |
| S7 Test-target wiring             | PASS | `interface_temp_tdd.rs` lands in a **fresh** `modules/core-modules/prime-tower-interface/tests/` dir — standalone binary, no aggregator, no `mod` registration (matching `wipe-tower`'s and `skirt-brim`'s pattern, both of which have no `tests/main.rs`); `wipe_tower_config_schema_tdd.rs` is created by `254a` and extended here; `manifest_ingestion_tdd` and `config_bounds_enforcement_tdd` are both already registered in `crates/slicer-scheduler/tests/integration/main.rs` |
| S8 ADR conformance                | PASS | no ADR normatively governs prime-tower behaviour or the postpass command channel. ADR-0050 (custom-G-code architecture, manifest-scoped placeholder domain) is the nearest; the packet conforms — it pushes a typed `GCodeCommand::Temperature` through the existing builder rather than injecting raw text or widening the placeholder domain |
| (existing) AC runnable command    | PASS | all 11 ACs and all 3 negative cases end in a single runnable pipe-suffixed command; no `cargo test --workspace` as an AC command |
| (existing) Doc Impact Statement   | PASS | `docs/15_config_keys_reference.md` and `docs/07_implementation_status.md` named, both generated-only, verified by AC-11 and `check-deviations --check` |

### Blockers (S4/S5/S6)

None. **In particular, the temperature seam is not a `[BLOCK]`.** The session constraint blocks on "a new WIT interface, IR schema bump, or host `ResolvedConfig` field"; this packet needs none. The load-bearing facts, each verified against the tree at authoring:

- `GCodeCommand::Temperature` and `GcodeOutputBuilder::push_temperature` already exist and are already marshalled end-to-end. What was missing is a *producer* — no production module has ever constructed one — which is exactly what this packet builds.
- `GCodeCommand::Move` already carries `role: ExtrusionRole`, and `ExtrusionRole::WipeTower` exists, so the emitter can locate the tower in the stream with no new carrier.
- The only real constraint is stage placement: the command channel lives on `GcodeOutputBuilder` (`run_gcode_postprocess`), never on `FinalizationOutputBuilder`. The packet respects that by adding a module at `PostPass::GCodePostProcess` instead of widening a builder — recorded as divergence D-254b-5.

### High (S1/S2/S3/S7/S8)

None outstanding. One structural risk is recorded rather than flagged: the packet introduces the tree's 24th core module, whose registration spans five files plus a hard-asserted count. `implementation-plan.md` §Blast-radius discipline enumerates every site, and Step 5's edit list carries the count change rather than leaving it to a follow-up `cargo check` — the exact failure mode the blast-radius rule exists to prevent.

### Accepted FORWARD-DEPs

- **`plan_layer_depths` / `generate_purge_paths(depth_offset, block_depth)` ← produced by draft packet `254a-prime-tower-geometry-keys`.** Names and shapes are reconciled between the two packets' `design.md` files; `254a`'s Step 3 postcondition and this packet's Step 0 precondition state the same signature. Step 0 is a hard gate that verifies the symbol on disk **and** `254a`'s `status: implemented` line before any edit, so the dep cannot be assumed satisfied.

All three `[FWD]` items in `design.md` (per-filament parameter model, nozzle-temperature model, staged unload/load) are forwarded *out* of this packet and gate no AC here.

### Map gates (wayfinder Authoring rule 6)

- **(a) zero declaration-only keys** — **PASS.** All nine keys drive a behaviour-changing decision point this packet builds: `enable_tower_interface_features` + `filament_tower_interface_purge_volume` (interface block depth), `filament_tower_interface_pre_extrusion_dist` (lead-in travel span), `filament_tower_interface_pre_extrusion_length` (lead-in extrusion), `prime_tower_flat_ironing` + `filament_tower_ironing_area` (ironing pass), `enable_filament_ramming` (ramming zigzag), `filament_tower_interface_print_temp` + `enable_tower_interface_cooldown_during_tower` (`Temperature` command and its position). Zero declared-with-gap, zero returned, zero dead-in-canonical.
- **(b) non-default AC per key** — **PASS.** `enable_tower_interface_features` = `true` and `filament_tower_interface_purge_volume` = `40.0` (AC-3); `filament_tower_interface_pre_extrusion_dist` = `25.0` (AC-4); `filament_tower_interface_pre_extrusion_length` = `5.0` (AC-5); `prime_tower_flat_ironing` = `true` and `filament_tower_ironing_area` = `9.0` (AC-6); `enable_filament_ramming` = `false` (AC-7 — canonical's default is `true`, so `false` is the non-default value and the packet says so explicitly rather than letting a reader mistake it for a default-identity assertion); `filament_tower_interface_print_temp` = `250` and `enable_tower_interface_cooldown_during_tower` = `true` (AC-8). AC-9's `-1` case asserts the *absence* behaviour that divergence D-254b-2 defines, and AC-N1's identity check is an additional criterion, never the sole evidence for any key.

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
