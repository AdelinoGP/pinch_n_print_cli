# Preflight Report: 261-raft-keys

Reviewed: 2026-08-31 · Mode: --preflight · Symbol-inventory dispatched: 2 (canonical reads ×2 — raft key declarations/consumers + soluble-interface nuance; remaining S5/S6/S7 items verified directly during authoring)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | — |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04/104); draft packet 240-support-raft referenced as *draft* (`status: draft` verified in `docs/spec_packets/240-support-raft/packet.spec.md`), never claimed as a satisfied dependency; packets 254/255/257/258/259/260 cited as authoring precedent only |
| S2 Deviation-ID conformance | PASS | no deviation IDs created, superseded, closed, or referenced (zero DEV-/D- tokens in the five files) |
| S3 Schema-version computed | PASS | no schema/version constants touched (zero `*_SCHEMA_VERSION` tokens) |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced (zero ADR tokens) |
| S5 Shipped-symbol existence/shape | PASS | `SupportPlanner::from_config` + raft `from_config` reads (`modules/core-modules/tree-support-planner/src/lib.rs` :1571-1590) + `push_raft_plan` (:1725); `RaftPlan` + `SupportPlanEntry` (crates/slicer-ir/src/slice_ir.rs :1399/:1363 — both derive `PartialEq`); `make_planner_config`/`overhang_plate_fixture`/`tree_analysis`/`make_layer_plan`/`make_region_segmentation` + `raft_and_interface_layers_emit_expected_entry_count` (orca_parity_tdd.rs :1481/:1521/:1621/:1490/:1503/:163); `cooling_config_schema_tdd.rs` guard pattern (part-cooling/tests); `load_module_from_paths` + `rejects_unknown_support_style_value`/`rejects_max_bridge_length_below_min` (config_bounds_enforcement_tdd.rs :84/:81/:125); `serialize_config_block`/`emit_config_kv`/`SUPPORT_CONFIG_DEFAULTS`/`ORCA_CONFIG_PADDING` (crates/slicer-gcode/src/serialize.rs — `("raft_layers", "0")` at :498, neither raft key in either table); `get_float_opt` (crates/slicer-scheduler/src/manifest.rs :1276) + `NumericBounds` None-as-unbounded (config_resolution.rs :17); `ConfigResolutionError::OutOfRange`/`TypeMismatch` (config_resolution.rs :217/:267); `guest_input_paths` (xtask/src/build_guests.rs :951 — fingerprints module manifests, confirming the wasm-staleness constraint); `tree-support-planner.toml` `[config.schema.max_bridge_length]` no-max float form (:209) — all verified by dispatch or direct authoring-time read |
| S6 WIT/IR identifier drift | PASS | `raft-plan` record + fields (`crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit` :36-41 — `raft-layers`, `raft-first-layer-density`, `base-raft-layers`, `interface-raft-layers`); `ConfigView` (orca_parity_tdd.rs :199) verified; no other WIT identifiers named |
| S7 Test-target wiring | PASS | module tests dir is file-per-binary with NO aggregator (no `tests/main.rs`); auto-discovery coexists with the explicit `[[test]]` entry (empirically confirmed: `cargo test -p tree-support-planner --test orca_parity_tdd --no-run` builds the binary); net-new `raft_config_schema_tdd.rs` lands as its own `--test` binary; scheduler/runtime additions go into existing binaries (no new integration files, no `main.rs` registration needed) |
| S8 ADR conformance | PASS | ADR-0009 (raft role/claim pattern) governs raft *rendering* — this packet declares config keys only, no role/claim/IR/WIT changes; no contradiction |
| (existing) AC runnable command | PASS | 5 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`raft_config_schema_tdd` net-new auto-discovered; `orca_parity_tdd` builds and exposes the raft-plan harness; `config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. `OutOfRange`/`TypeMismatch`; `gcode_header_thumbnail_config_blocks_tdd.rs` drives CONFIG_BLOCK emission — proven at packet 258/259/260 authoring) |
| (existing) Doc Impact Statement | PASS | key-presence greps `rg -q 'raft_contact_distance'` + `rg -q 'raft_expansion'` + deviation-block row-count probe (27, sed-pattern matched against the real generated doc block at authoring — 27 data rows measured) |

## Corrections made during preflight

1. **Canonical "ignored for soluble interface" nuance.** The reference tooltip
   says `raft_contact_distance` is "ignored for soluble interface"; the
   canonical read proved this is **GUI-only** (`ConfigManipulation.cpp`
   disables the field for soluble support) — no slicing branch forces the gap
   to 0 on solubility. The only suppression is `zero_gap_interface_raft`
   (`raft_z_gap == 0.0 || zero_topZ_contact`). Recorded in the per-key
   evidence table so the implementer does not invent a soluble branch.
2. **Owner narrowed, not corrected.** The tier table's owner `support-planner`
   is confirmed, but only `tree-support-planner` has raft surface (raft config
   cluster + `RaftPlan` emission); `traditional-support-planner` has none. The
   packet declares in `tree-support-planner.toml` and pins the traditional
   omission (AC-N2) — the 04 owner column stays unchanged.
3. **Tier re-adjudication.** Both keys are re-adjudicated declared-with-gap
   (decision points absent — no raft geometry generator in-tree), mirroring
   P11's pattern-key finding; recorded in the ticket closure, not the 04 tier
   column.
4. **Canonical bounds adopted outright.** Net-new declarations take canonical
   bounds (min 0, no max) — the in-tree `max_bridge_length` table is the
   no-max float precedent; no declared-bounds divergence is created (unlike
   packet 260's kept port `max = 2.0` on pre-existing keys).
5. **Test-target auto-discovery verified empirically.** The explicit `[[test]]`
   entry in the module Cargo.toml does not disable auto-discovery — the
   `--no-run` build of `orca_parity_tdd` confirmed the binary exists. The
   net-new guard file needs only the `toml = "0.8"` dev-dependency
   (add-if-absent, Step 1), no `[[test]]` registration.
6. **Deviation-row count re-derived.** The generated deviations block measures
   **27 data rows** at authoring (packet 260's alignment is draft, not yet
   implemented — the block still shows the two 0.4 rows); AC-5 pins 27
   post-packet as the observable (both declared defaults match canonical).

## Accepted FORWARD-DEPs

- `raft_contact_distance` / `raft_expansion` declarations ← consumed by draft
  packet 240-support-raft's `com.core.raft-default` (its AC-5 declares the
  same keys in its manifest and wires them to geometry; names/shapes
  reconciled against the canonical `PrintConfig.cpp` `def()`s — coFloat 0.1 /
  1.5, min 0, no max). This packet is the config-reachability half; 240's
  wire-or-record decision for the four support-module manifests is this
  packet's recorded input.

## Verdict

**PREFLIGHT PASS** (0 blockers, 0 high)
