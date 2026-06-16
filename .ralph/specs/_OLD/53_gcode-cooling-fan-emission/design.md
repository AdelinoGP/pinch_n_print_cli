# Design: 53_gcode-cooling-fan-emission

## Controlling Code Paths

- Primary code path:
  - **NEW** `modules/core-modules/part-cooling/` — new finalization-stage module crate, structured exactly like `modules/core-modules/skirt-brim/` (the reference template).
  - `crates/slicer-host/src/dispatch.rs` around `:2854` — `dispatch_finalization_call` site; one new match arm or registration entry that loads `part-cooling.wasm` and invokes its `run_finalization` entry point.
  - `crates/slicer-host/src/config_schema.rs:104-176` — register eight cooling-profile keys.
  - `crates/slicer-host/src/gcode_emit.rs:466` — UNCHANGED. The `M106` serializer already exists. This packet only PRODUCES `GCodeCommand::FanSpeed` events; serialization is already wired.
- Neighboring tests or fixtures:
  - `modules/core-modules/skirt-brim/tests/*` (if any) — reference for how a finalization module is exercised.
  - `crates/slicer-host/tests/orca_comment_contract_tdd.rs` — regression target.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` — algorithm SUMMARY.

## Architecture Constraints

- The cooling module lives on the **finalization stage**, not the path-optimization stage. This is the deliberate supersession of TASK-152c.
- The module receives a finalized `LayerCollectionIR` and may insert new `PrintEntity`-equivalent events that carry `GCodeCommand::FanSpeed` — exact insertion API mirrors `SkirtBrim`'s `push_entity_to_layer` pattern (per reconnaissance: skirt-brim/src/lib.rs:100 sets `role: path.role.clone()` on entities, so the same `FinalizationOutputBuilder` API is used).
- IR contract is unchanged: `GCodeCommand::FanSpeed` already exists. No new IR variants.
- WIT manifest must declare `cooling` as a `finalization` stage module reading the same `LayerCollectionView` surface that `SkirtBrim` reads, and writing `FanSpeed` events (the WIT capability set covering this is whatever `SkirtBrim` declares — confirmed in Step 1).
- Determinism: cooling decisions depend only on `(layer_index, layer_time, region_role)` → fan-speed. Layer time is currently NOT computed in the live path (would require packet 52's feedrate emission). For this packet, layer-time-based slowdown is deferred; the algorithm reduces to:
  - layer `< disable_fan_first_layers` → emit `M107` (or `M106 S0`) at layer start.
  - layer `>= disable_fan_first_layers` → emit `M106 S<fan_speed_max>` at layer start.
  - Overhang region detected (`PrintEntity` with role tag indicating overhang) → emit `M106 S<scale(overhang_fan_speed, fan_speed_max)>` at region start, restore prior at region end.
  - Last layer end → emit `M107`.
- The cooling rejection snippet in `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections" is removed entirely — cooling is now supported via the finalization-stage module, making the old "unsupported" wording misleading.

## Code Change Surface

- Selected approach: **finalization-stage WASM module + dispatcher wiring + config keys.** Mirrors the existing `SkirtBrim` shape exactly.
- Exact files / functions / fixtures expected to change:
  - **NEW** `modules/core-modules/part-cooling/Cargo.toml` — new crate manifest, identical structure to `modules/core-modules/skirt-brim/Cargo.toml`.
  - **NEW** `modules/core-modules/part-cooling/part-cooling.toml` — module manifest (TOML), declares `stage = "finalization"`, capability set, the 8 config keys it reads.
  - **NEW** `modules/core-modules/part-cooling/src/lib.rs` — implements `pub struct Cooling`, `from_config`, `run_finalization`. Uses the same `FinalizationOutputBuilder` API as skirt-brim. ≤ 250 lines target.
  - **EDITED** `crates/slicer-host/src/dispatch.rs` — one new arm in the finalization dispatcher near `:2854`. ≤ 15 lines added.
  - **EDITED** `crates/slicer-host/src/config_schema.rs` — eight new field registrations.
  - **EDITED** `docs/05_module_sdk.md` — remove the cooling rejection snippet entirely from the Layer Stage Module Surface Rejections section (cooling is now supported via the finalization-stage module; the old "unsupported" wording is misleading).
  - **EDITED** `docs/07_implementation_status.md` — TASK-152c gets a Superseded marker; new TASK-152d and TASK-154 rows.
  - **EDITED** `docs/DEVIATION_LOG.md` — supersession + DEV-009 progress.
  - **EDITED** `docs/14_deviation_audit_history.md` — DEV-009 remediation progress note.
  - **NEW** `crates/slicer-host/tests/gcode_part_cooling_emission_tdd.rs` — TDD test file.
  - **EDITED** `modules/core-modules/build-core-modules.sh` — add the `cooling` crate to the build list (one-line edit).
- Rejected alternatives:
  - **Embed cooling directly in `gcode_emit.rs`.** Rejected: violates the modular-slicer architecture (`docs/00`) and couples emit to policy; if a user wants a different cooling policy they'd have to fork the host.
  - **Reuse the path-optimization surface (revert TASK-152c).** Rejected: that surface was rejected for principled reasons (no live override mechanism). Finalization is the correct home.
  - **Serializer post-pass over the produced G-code text.** Rejected: produces fragile, hard-to-test transformations and re-implements layer-boundary detection that the IR already encodes.

## Files in Scope (read + edit)

- `modules/core-modules/part-cooling/src/lib.rs` — primary edit (new); ≤ 250 lines.
- `crates/slicer-host/src/dispatch.rs` — range-edit `:2840-:2900` only; ≤ 15 lines added.
- `crates/slicer-host/src/config_schema.rs` — edit; eight new field entries.
- `crates/slicer-host/tests/gcode_part_cooling_emission_tdd.rs` — primary edit (new).
- `modules/core-modules/part-cooling/Cargo.toml`, `part-cooling.toml`, `modules/core-modules/build-core-modules.sh`, the four docs files — small targeted edits; not "primary" but unavoidable.

> The "≤ 3 primary files" rule is met (3 primary: cooling/src/lib.rs, dispatch.rs, tests file). The auxiliary edits are small, mechanical, and individually well-scoped.

## Read-Only Context

- `modules/core-modules/skirt-brim/src/lib.rs` — load directly (small per reconnaissance); the canonical template.
- `modules/core-modules/skirt-brim/Cargo.toml` and `skirt-brim.toml` — load directly; templates for the new manifest.
- `crates/slicer-host/src/gcode_emit.rs` — range-read `:460-:475` only — confirm the `M106` serializer arm.
- `docs/03_wit_and_manifest.md` — load directly the manifest TOML schema section.
- `docs/05_module_sdk.md` — load directly the two relevant sections (≤ 60 lines).
- `crates/slicer-host/src/dispatch.rs` — range-read `:2840-:2900` only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate every read.
- `target/`, `Cargo.lock`, generated WIT bindings — never load.
- The full `crates/slicer-host/src/dispatch.rs` — out of range.
- `.ralph/specs/19_path-optimization-tool-order-and-cooling-policy/` — DO NOT reopen. It was the source of TASK-152c. Its conclusion is preserved; this packet does not modify it.
- The full `docs/07_implementation_status.md` — delegate row insertion.
- `modules/core-modules/wipe-tower/`, `modules/core-modules/path-optimization/`, etc. — only `skirt-brim/` is in read scope as the template.

## Expected Sub-Agent Dispatches

- "Return the WIT capability set declared by `modules/core-modules/skirt-brim/skirt-brim.toml` AND its `Cargo.toml` `[lib]`/`crate-type` configuration. Return: FACT, ≤ 12 lines."
- "Return the OrcaSlicer default values for `fan_speed_min`, `fan_speed_max`, `disable_fan_first_layers`, `enable_overhang_fan`, `overhang_fan_speed`, `slow_down_for_layer_cooling`, `slow_down_min_speed`, `slow_down_layer_time`. Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`. Return: FACT, one row per key, ≤ 12 lines."
- "Summarize the cooling decision algorithm in `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` in ≤ 200 words. Highlight first-layer-disable, max-speed, and overhang-bump logic. No code snippets."
- "Show the dispatch arm in `crates/slicer-host/src/dispatch.rs` around `:2854` that invokes `SkirtBrim`. Return: SNIPPETS, ≤ 30 lines, file:line."
- "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail and SNIPPETS for any error."
- "Run `cargo test -p slicer-host --test gcode_part_cooling_emission_tdd`; return FACT pass/fail; SNIPPETS for failing tests."
- "Append rows to `docs/07_implementation_status.md` (TASK-152c superseded marker, new TASK-152d and TASK-154); return EDITED/NOT-EDITED."

## Data and Contract Notes

- IR contracts touched: none added/changed. `GCodeCommand::FanSpeed { value: u8 }` already exists.
- WIT boundary: the new module's TOML declares `finalization` stage and the same capability set as skirt-brim (or a subset). Confirmed by Step 1 dispatch.
- Determinism: cooling decision is a pure function of `(layer_index, region_role_tags, config)`. Layer time NOT consulted (would require packet 52's per-move time computation; deferred).
- Module loading: dispatcher loads `modules/core-modules/part-cooling/part-cooling.wasm` at startup. Path mirrors `skirt-brim/skirt-brim.wasm`.

## Locked Assumptions and Invariants

- The finalization stage runs after `SkirtBrim` and before `GCodeIR` serialization. The cooling module slot is positioned between them. (Step 1 confirms exact ordering.)
- Overhang detection uses the existing `PrintEntity` role tagging that the bridge-detector packet (36-rev1) installed. If no overhang-tagged entities are present, the overhang branch is a no-op and tests `overhang_fan_bumped` use a hand-built fixture.
- `M106 S0` is treated as equivalent to `M107` for the "fan off" criterion. The module emits `M107` by convention.

## Risks and Tradeoffs

- Risk: WIT capability mismatch — the new module may need to DECLARE a capability that `SkirtBrim` does not. Mitigated by the Step 1 FACT dispatch and a follow-up `cargo build` after the manifest lands.
- Risk: dispatcher invocation order — placing cooling before skirt-brim would let cooling fan-bumps interfere with adhesion. Locked invariant: `[SkirtBrim, WipeTower (if any), Cooling] -> Serialize`. Tests assert this implicitly via the layer-2 fan-on assertion (skirt is layer 0/1 only).
- Risk: layer-time-driven slowdown algorithm is deferred — users with thin layers may see suboptimal cooling. Documented as known limitation; future packet handles it.
- Tradeoff: the new crate adds CI build time. Accepted; `build-core-modules.sh` already compiles four modules.

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 3 (new module crate + manifest). M.
- Highest-risk dispatch: CoolingBuffer SUMMARY — must return ≤ 200 words, no code snippets. If the sub-agent returns SNIPPETS, re-dispatch with tighter scope.

## Resolved Questions

- **docs/05 editing approach (was an activation blocker):** Resolved — user chose to **remove the cooling rejection snippet entirely** from `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections", rather than adding a pointer or splitting sections. Cooling is now supported via the finalization-stage module, so the old "cooling unsupported" wording is misleading and should be deleted.
- Whether the cooling module should be `default_enabled = true` in its manifest. Locked decision: yes — DEV-009 demands cooling out-of-the-box. Users disable by setting `fan_speed_max = 0`.

## Step 1 Discovery Block

### Skirt-brim template (FACT)
- wit-world: `slicer:world-finalization@1.0.0`
- stage: `PostPass::LayerFinalization`
- ir-access: reads `LayerCollectionIR`, writes `LayerCollectionIR.skirt-brim`
- Dependencies: slicer-sdk, slicer-schema, slicer-ir (path deps)
- Crate-type: coder must verify `[lib] crate-type = ["cdylib"]` — explore worker reported "not present" which may be incomplete

### Dispatcher mechanism (SNIPPETS)
- `dispatch.rs:50`: `"PostPass::LayerFinalization" => Some("run-finalization")` — stage → export name mapping
- `dispatch.rs:1052`: `fn dispatch_finalization_call(...)` — generic stage dispatch
- `dispatch.rs:2854`: `let pushes = match self.dispatch_finalization_call(stage_id, module, ...)`
- Modules dispatched generically by stage ID — **no per-module match arm needed**; cooling is registered in the module list and the dispatcher picks it up automatically

### OrcaSlicer defaults (FACT)
| Our key | OrcaSlicer key | Default | Type |
|---------|---------------|---------|------|
| fan_speed_min | fan_min_speed | 20 (%) → 51 (0-255) | u8 |
| fan_speed_max | fan_max_speed | 100 (%) → 255 (0-255) | u8 |
| disable_fan_first_layers | close_fan_the_first_x_layers | 1 | u32 |
| enable_overhang_fan | enable_overhang_bridge_fan | true | bool |
| overhang_fan_speed | overhang_fan_speed | 100 (%) | u8 |
| slow_down_for_layer_cooling | slow_down_for_layer_cooling | true | bool |
| slow_down_min_speed | slow_down_min_speed | 10.0 | f64 mm/s |
| slow_down_layer_time | slow_down_layer_time | 5.0 | f64 s |

Note: fan_speed_min and fan_speed_max stored as 0-255 (M106 range); overhang_fan_speed stored as 0-100 percentage, scaled at runtime: `M106 S(overhang_fan_speed * fan_speed_max / 100)`.

### CoolingBuffer algorithm (SUMMARY)
Simplified for this packet: no layer-time interpolation. Algorithm: (1) layers < disable_fan_first_layers → M107; (2) layers >= disable_fan_first_layers → M106 S<fan_speed_max>; (3) overhang region → M106 S<scale(overhang_fan_speed%, fan_speed_max)>, restore prior after; (4) last layer end → M107.
