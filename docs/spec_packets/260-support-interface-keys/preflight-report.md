# Preflight Report: 260-support-interface-keys

Reviewed: 2026-08-31 · Mode: --preflight · Symbol-inventory dispatched: 3 (canonical reads ×2, in-tree S5/S7 sweep ×1; remaining S5 items verified directly during authoring)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | — |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04/104) + packet 238c referenced as the *implemented* origin of the spacing-key wiring (`status: implemented` verified in `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md`); packets 254/255/257/258/259 cited as authoring precedent only, never claimed as satisfied dependencies |
| S2 Deviation-ID conformance | PASS | no deviation IDs created, superseded, closed, or referenced (zero DEV-/D- tokens in the five files) |
| S3 Schema-version computed | PASS | no schema/version constants touched (zero `*_SCHEMA_VERSION` tokens) |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced (zero ADR tokens) |
| S5 Shipped-symbol existence/shape | PASS | `TraditionalSupport`/`TreeSupport` structs, `from_config`, `pitches_mm`, `DEFAULT_INTERFACE_SPACING_MM` (both modules/core-modules/{traditional-support,tree-support}/src/lib.rs — const 0.4 verified at :51/:60, `pitches_mm` at :361/:102); `slicer_core::support_regularize::{body_density,interface_density,bottom_interface_density}` (:18/:28/:38); `config_bounds_enforcement_tdd.rs` + `ConfigBoundsIndex::from_modules`, `resolve_global_config`, `ConfigResolutionError::OutOfRange` (crates/slicer-scheduler, `OutOfRange` re-exported from `crates/slicer-ir/src/resolved_config.rs`); `gcode_header_thumbnail_config_blocks_tdd.rs`, `support_family_closure.rs`, `orca-matched-config.json` (value `0.4` at line 21); `cooling_config_schema_tdd.rs` guard pattern; `serialize_config_block`/`emit_config_kv`/`SUPPORT_CONFIG_DEFAULTS` (crates/slicer-gcode/src/serialize.rs — none of the four keys in either table); `guest_input_paths` (xtask/src/build_guests.rs — fingerprints module `*.toml` + `src/`, confirming the wasm-staleness constraint); `tree-support-planner.toml` `[config.schema.support_style]` enum form — all verified by dispatch or direct authoring-time read |
| S6 WIT/IR identifier drift | PASS | `SupportPlanIR`/`SupportIR` (ir-access of both support tomls), `ExtrusionRole::SupportInterface` (traditional-support lib.rs role assignment), `ConfigView` (both `from_config` signatures) verified; no WIT identifiers named |
| S7 Test-target wiring | PASS | module tests dirs are file-per-binary with NO aggregator (verified: no `tests/main.rs` in either module); auto-discovery coexists with the explicit `[[test]]` entries (empirically confirmed: `cargo test -p traditional-support --test traditional_support_tdd --no-run` builds the binary); net-new `support_config_schema_tdd.rs` ×2 land as their own `--test` binaries; scheduler/runtime additions go into existing binaries (no new integration files, no `main.rs` registration needed) |
| S8 ADR conformance | PASS | no ADR governs the support-interface keys; the retained `-1` mirror predates the packet and the alignment direction conforms to the map's standardise-to-Orca ruling (ticket 07); no IR/WIT/claim changes |
| (existing) AC runnable command | PASS | 6 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`traditional_support_tdd`/`tree_support_tdd` build and expose the `interface_paths` fixtures (helpers at :94/:99); `config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. enum `TypeMismatch`; `gcode_header_thumbnail_config_blocks_tdd.rs` drives CONFIG_BLOCK emission; `support_family_closure.rs` runs the real fixture) |
| (existing) Doc Impact Statement | PASS | key-presence greps `rg -q 'support_interface_pattern'` + `rg -q 'support_interface_loop_pattern'` + deviation-block row-count probe (27 → 25, sed-pattern matched against the real generated doc block at authoring) |

## Corrections made during preflight

1. **Canonical type correction for `support_interface_loop_pattern`.** The
   canonical read established the key is **coBool** (default `false`), not an
   enum — the requirements table, AC-1, and the wiring notes record the type
   correction explicitly so the implementer declares `type = "bool"`.
2. **Canonical default correction for `support_interface_spacing` (0.5, not
   0.4).** The canonical read overturned the port's declared 0.4 (its own
   comments claim Orca's default is 0.4 — mis-derived). Surfaced to the human;
   user ruling 2026-08-31: **align to 0.5**. Rippled through AC-1/2/6, scope,
   requirements (problem statement, per-key evidence, wiring notes), design
   (change surface, locked assumptions, risks), and implementation-plan Step 2.
3. **Mirror-sentinel adjudication.** The canonical read proved
   `support_bottom_interface_spacing` has **no -1 sentinel** in Orca (the
   sentinel belongs to `support_interface_bottom_layers`). Surfaced to the
   human; user ruling 2026-08-31: **keep as recorded divergence**. AC-3 pins
   the mirror as a witness, AC-4 keeps `-1.0` legal in the bounds index, and
   manifest comments document it.
4. **Owner correction (tier table vs code).** The tier table's owner
   `support-planner` is the claim held by the two planner modules, but neither
   planner reads interface config; the decision points are in `traditional-support`
   and `tree-support`. The packet declares in the decision-point modules and
   records the correction (ticket 18 closure updates the 04 row).
5. **Deviation-row count re-derived.** The generated deviations block measures
   **27 data rows** at authoring (not "27 at 105-closure" quoted loosely);
   AC-6 pins 25 post-alignment as the observable, with the no-`support_interface_spacing`-row probe.
6. **Test-target auto-discovery verified empirically.** The explicit `[[test]]`
   entries in both module Cargo.tomls do not disable auto-discovery — the
   `--no-run` build of `traditional_support_tdd` confirmed the binary exists.
   The net-new guard files need only the `toml = "0.8"` dev-dependency
   (add-if-absent, Step 1), no `[[test]]` registration.

## Accepted FORWARD-DEPs

- None — packet depends only on resolved wayfinder tickets and queue ordering.

## Verdict

**PREFLIGHT PASS** (0 blockers, 0 high)
