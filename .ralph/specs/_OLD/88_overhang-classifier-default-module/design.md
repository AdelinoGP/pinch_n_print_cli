# Packet 88 — Design

## Controlling Code Paths

```
modules/core-modules/overhang-classifier-default/
├── Cargo.toml          deps: slicer-sdk, slicer-schema, slicer-ir  (NO slicer-core)
├── overhang-classifier-default.toml   stage = "PostPass::LayerFinalization", trait = "FinalizationModule"
├── src/
│   ├── lib.rs          #[slicer_module] impl FinalizationModule { ... }
│   ├── classify.rs     ← RELOCATED from crates/slicer-core/src/algos/overhang_classifier.rs (~319 LOC)
│   └── lines_distancer.rs   ← RELOCATED from crates/slicer-core/src/aabb_lines_2d.rs (LinesDistancer2D)
├── tests/
│   └── basic_tdd.rs    #[module_test] two-layer fixture (subsumes the P84 golden)
└── wit-guest/          per-guest WIT shim (per ADR-0003)

DELETIONS IN HOST CRATES:
- crates/slicer-core/src/algos/overhang_classifier.rs       (relocated to guest)
- crates/slicer-core/tests/algo_overhang_classifier_tdd.rs  (P84 golden; invariants live in guest tests)
- crates/slicer-core/src/algos/mod.rs                        (drop the `pub mod overhang_classifier;`)
- crates/slicer-runtime/src/lib.rs:192                       (drop P84 compat shim re-export)
- crates/slicer-core/src/aabb_lines_2d.rs                    (conditional — delete if no other consumer)

EMIT PATH AFTER P88:
slicer-gcode/src/emit.rs::DefaultGCodeEmitter::emit_gcode
  ├── (deleted): slicer_core::classify_layers(&mut layers, &feedrate) at L226
  ├── (deleted): use slicer_core::classify_layers at L21
  ├── (deleted): resolve_feedrate's overhang_quartile branch at L106-123
  └── (kept):    multiplicative speed-factor application path — existing finalization-implementing
                 modules already drive this; overhang now flows through the same mechanism.

EXECUTION FLOW:
host scheduler → all FinalizationModule core-modules (claim-ordered) → entities accumulate
                 mutations → gcode_emit serializes with cumulative speed_factor applied
```

**Template module for guest shape**: any of the 20 existing modules in `modules/core-modules/` (e.g., `seam-planner-default/` verified to follow the standard layout: `Cargo.toml` with `slicer-sdk`/`slicer-schema`/`slicer-ir` deps + wasm32-only `wit-bindgen`; `<name>.toml` manifest; `src/`; `wit-guest/`; `<name>.wasm` artifact). **There is no `modules/core-modules/finalization-default/`** — prior packet drafts cited it as a "template"; that directory does not exist. Step 1 dispatch #1 surfaces a real existing FinalizationModule-implementing module.

OrcaSlicer comparison surface: NONE NEW. The parity baseline (AC-2 short-circuit on zero overhang speeds, the strict-`>` quartile threshold convention) is preserved verbatim inside `slicer_core::classify_layers` (moved in P84). The module's job is to call the kernel and translate its output to `set-speed-factor` mutations.

## Architecture Constraints

- ADR-0001 / 0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0008 (P85 — CompiledModule Static/Live split) preserved. ADR-0004 (Test support in slicer-sdk, P77) unrelated to this packet's surface.
- ADR-0008 (drafted at P88 close) — overhang annotation as a `FinalizationModule`; no new stage; module-absent ⇒ no annotation; guest owns the complete algorithm (no `slicer-core` dep) to prevent the `host-algos` feature gate from contaminating the guest dep tree.
- No WIT file is edited. AC-N3 verifies via `git diff`.
- The new module's manifest claims the same world (`slicer:world-finalization@1.0.0`) as `finalization-default`. Both run in the same stage; ordering is per the DAG's claim-based topology (no two modules claim the same role, so they run sequentially in declared order).

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by this edit.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`slicer_core::classify_layers` operates in integer units throughout — the module passes layer geometry to the kernel unchanged.)

## Selected Approach

Ship a new core-module that mirrors the existing 20 in structure. Reuse `slicer-core`'s kernel (P84) and the existing `set-speed-factor` consumption path in `slicer-gcode` (P86). Delete the one direct-call line in `slicer-gcode/src/emit.rs`.

Rejected alternatives:

- **Add a new `Layer::OverhangAnnotation` stage** with a dedicated WIT export. Rejected by Q3 grilling: the existing `world-finalization::run-finalization` already provides the exact seam needed (layer-collection-view input + modify-entity output). Adding a stage = WIT contract change = 20 guest rebuilds = scope explosion.
- **Keep a host fallback** (`OVERHANG_CLASSIFICATION_PRODUCER` builtin that runs when no module claims the role). Rejected by Q6 grilling: two implementations to maintain, claim-based mutual-exclusion logic to add. The ship-the-module-default approach mirrors how the existing 20 core-modules work — clean.
- **Have the module emit absolute speeds via a new `set-absolute-speed` variant**. Rejected: WIT contract change. The multiplicative `set-speed-factor` is mathematically equivalent (`factor = overhang_speed / base_speed`, then emit multiplies); the LSB-precision risk is acceptable per AC-7's documented trade-off.
- **Encode the quartile (not the factor) in `set-speed-factor` and have gcode_emit do the table lookup**. Rejected: that re-introduces the role-aware feedrate logic in `slicer-gcode` that this packet is removing. The module's job is to make the policy decision.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `modules/core-modules/overhang-classifier-default/Cargo.toml` | **CREATE** | Mirror `modules/core-modules/finalization-default/Cargo.toml` (or another FinalizationModule core-module) for shape. Deps: `slicer-sdk` (with appropriate features), `slicer-ir`, `slicer-core`. NO `slicer-runtime` etc. |
| `modules/core-modules/overhang-classifier-default/module.toml` (or `manifest.toml`) | **CREATE** | Stage: `PostPass::LayerFinalization`. Trait: `FinalizationModule`. World: `slicer:world-finalization@1.0.0`. Config reads: the four `overhang_*_4_speed` keys. |
| `modules/core-modules/overhang-classifier-default/src/lib.rs` | **CREATE** | `#[slicer_module]` impl `FinalizationModule`. See §Selected Approach for body shape. |
| `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` | **CREATE** | `#[module_test]` per AC-8. Two-layer fixture; asserts `SetSpeedFactor` mutation on overhang entity. |
| `modules/core-modules/overhang-classifier-default/wit-guest/world-finalization.wit` (or similar — per existing module convention) | **CREATE / SYMLINK** | Mirror what other FinalizationModule modules do. The actual WIT comes from `crates/slicer-schema/wit/` via `bindgen!`; this is the per-guest shim (per ADR-0003). |
| `crates/slicer-gcode/src/emit.rs` | **EDIT** | Delete the `use slicer_core::classify_layers;` line and the `classify_layers(&mut layers, &feedrate_config);` call (the single site added in P86). Body otherwise unchanged. |
| `Cargo.toml` (workspace) | **EDIT** | Add `"modules/core-modules/overhang-classifier-default"` to `members`. (If `xtask` discovers modules via members; otherwise update xtask's module list — confirm via dispatch #2.) |
| `xtask/src/main.rs` (or whichever xtask file lists modules) | **EDIT — conditional** | If guest-build is driven by an explicit module list rather than `members` discovery, add the new module entry. Confirm via dispatch #2. |

Primary edit target ≤ 3 files: the new module's source tree (counts as one — 4-5 small files), `crates/slicer-gcode/src/emit.rs` (one-line deletion), workspace `Cargo.toml`. All other edits are conditional.

## Files in Scope (read+edit)

The 8 files in the table above, plus the conditional xtask file from dispatch #2.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` | Confirm `entity-mutation::set-speed-factor`, `modify-entity`, `layer-collection-view` shapes. | Full file (≤ 130 LOC). |
| `modules/core-modules/finalization-default/{Cargo.toml,module.toml,src/lib.rs}` | Template / pattern. NEVER load full lib.rs. | Cargo.toml + module.toml in full; src/lib.rs first 80 lines (imports + `#[slicer_module]` skeleton). |
| `crates/slicer-core/src/algos/overhang_classifier.rs` (post-P84) | The kernel; confirm `classify_layers` signature and the quartile convention docstring. | Lines 1–60. |
| `crates/slicer-gcode/src/emit.rs` | Find the `classify_layers` call site to delete. | Grep `classify_layers`; ±10 lines. |
| `xtask/src/main.rs` (or per dispatch #2) | Confirm how new modules get registered for guest build. | Grep `core-modules` or `build-guests`. |
| `docs/05_module_sdk.md` | `FinalizationModule` trait sig, `#[slicer_module]` macro, manifest schema. | Delegate SUMMARY if > 300 LOC. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work.
- All P81–P87 files in their post-move locations — read only their `use` lines if needed.
- `crates/slicer-schema/wit/**` — DO NOT EDIT (AC-N3 verifies).
- The other 20 core-modules' src files (except the template `finalization-default`).

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | What is the structural template of an existing `FinalizationModule` core-module? Inspect `modules/core-modules/finalization-default/` (or another existing module that impls `FinalizationModule`) and report: (a) Cargo.toml dep list, (b) module.toml stage entry, (c) the first 30 lines of src/lib.rs including the `#[slicer_module]` attribute and the `impl FinalizationModule` line. | The directory | SNIPPETS (3 short blocks) |
| 2 | How does `cargo xtask build-guests` discover modules to build? Is it members-driven (auto from workspace `Cargo.toml`) or list-driven (explicit `xtask/src/...` enumeration)? | `xtask/src/` | FACT (1-line answer + file:line if list-driven) |
| 3 | What is the exact `FinalizationModule` trait shape in `slicer-sdk` (post-P78 fold)? Method signatures only. | `crates/slicer-sdk/src/` | SNIPPET (≤ 20 lines — the trait def) |
| 4 | Does the existing emit path in `slicer-gcode/src/` already consume `set-speed-factor` annotations off entities? (i.e., does the multiplicative path exist?) Grep for `speed_factor`. | `crates/slicer-gcode/src/` | LOCATIONS (≤ 5 entries — confirm yes/no with file:line) |
| 5 | After scaffold, `cargo xtask build-guests`. | repo root | FACT pass/fail + new wasm filename if success |
| 6 | After scaffold + delete classify_layers call, `cargo build --workspace`. | repo root | FACT pass/fail |
| 7 | After build, `cargo test -p overhang-classifier-default`. | repo root | FACT pass/fail |
| 8 | After build, run AC-5: `pnp_cli slice ... --instrument-stderr 2>&1 | grep overhang`. | repo root | FACT (line showing module loaded) |
| 9 | Post-packet g-code SHA. Compare to P87 baseline. | repo root | SNIPPET (post SHA + diff verdict per AC-7) |
| 10 | Workspace test gate. | repo root | FACT pass/fail + duration + count |

## Data and Contract Notes

- `factor` value range: typically 0.0 < factor ≤ 1.0 (overhang speeds are slower than base). The WIT type is `f32`; the SDK should accept any positive finite value.
- `base_speed_for_role` lookup: the module reads `feedrate_config` for the base speed of the entity's role. For wall-family roles, the base speed is typically `outer_wall_speed` or `inner_wall_speed`; the existing `crates/slicer-runtime/src/gcode_emit.rs::resolve_feedrate` (pre-P86; now in `slicer-gcode`) has the role→base-speed mapping table — copy the relevant subset into the module.
- `factor = overhang_speed / base_speed`: f32 division. AC-7 documents the LSB-precision shift risk.
- Zero-overhang-speed short-circuit: if all four `overhang_*_4_speed` are 0.0, the module returns early without emitting any mutations. Preserves the pre-P84 AC-2 byte-identical baseline for printers that don't configure overhang speeds.

## Locked Assumptions and Invariants

- ADR-0008 (drafted at close): overhang is a FinalizationModule; no new stage; no host fallback.
- No WIT contract change (AC-N3).
- 8 builtins in `runtime_builtins()` (unchanged).
- Default `pnp_cli slice --module-dir modules/core-modules` invocations include the new module; behavior matches the documented batch baseline (byte-identical or AC-7-documented LSB shift).
- Custom invocations without the module produce slice output without overhang annotation; AC-6 documents this as the explicit user opt-out.

## Risks and Tradeoffs

- **Risk: AC-7 SHA divergence beyond LSB precision.** Mitigation: dispatch #9's diff verdict catches any non-F-word divergence; bisect the module logic (most likely culprits: wrong quartile classification, wrong base-speed lookup, or `set-speed-factor` not being multiplicatively applied as expected by `slicer-gcode`'s emit path).
- **Risk: workspace test gate flakes** (large suite). Mitigation: dispatch #10 captures duration and count; re-dispatch specific failing tests individually.
- **Risk: the existing `finalization-default` module claims a role that conflicts with `overhang-classifier-default`'s claims.** Mitigation: the new module's claims should be a unique set (e.g., `overhang-speed-factor`); confirm via the DAG validation that no two modules claim the same role.
- **Tradeoff: f32 LSB precision in `factor = overhang_speed / base_speed`.** Acceptable; AC-7 explicitly documents the trade-off. Real-world printers tolerate f32 feedrate decimal noise.

## Context Cost Estimate

- Aggregate: **M.** Total step count: 9. No L step.
- Largest single step: step 4 (module source body), rated M.
- Highest-risk dispatch: dispatch #10 (workspace test gate). Final checkpoint of the batch.

## Open Questions

`None — change is reversible by deleting the new module dir and reverting the gcode_emit one-line deletion. The AC-7 SHA-shift documentation, if needed, is the only artifact that survives a rollback.`

One ADR planned at close — **ADR-0008** — overhang annotation as `FinalizationModule`, no new stage, no host fallback.
