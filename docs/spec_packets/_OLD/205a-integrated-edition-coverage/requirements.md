# Requirements: 205a-integrated-edition-coverage

## Packet Metadata

- Grouped task IDs: `ADR-0056`, `ADR-0057` (no `docs/07_implementation_status.md` TASK row exists for this program; do not invent one, and do not edit `docs/07` while the parallel 194-199 session is active)
- Backlog source: `docs/specs/multi-edition-distribution-plan.md` §"Also unscheduled" (the "follow-on packet (205a+)" note)
- Packet status: `draft`
- Aggregate context cost: `L`

## Problem Statement

ADR-0057's Integrated edition is defined as "every core module integrated". Packet 204 piloted three; packet 205 deliberately made `cargo xtask dist --edition integrated` fail loudly with a named list until coverage is complete. The plan's "Also unscheduled" note requires a follow-on packet (205a+) to integrate the remaining core modules. This packet is that follow-on for the sixteen modules whose native dispatch stages are already committed by packet 202's transport. It does not close the plan: two modules (`path-optimization-default`, `machine-gcode-emit`) map to native transports that return a fatal error today, and integrating them requires packet 205b to complete those transports first.

## In Scope

- Sixteen modules integrated into `crates/slicer-integrated-modules/` behind per-module cargo features: `fuzzy-skin`, `gyroid-infill`, `infill-linker`, `layer-planner-default`, `lightning-infill`, `overhang-classifier-default`, `part-cooling`, `rectilinear-infill`, `seam-placer`, `seam-planner-default`, `skirt-brim`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`, `tree-support`, `wipe-tower`.
- Sixteen per-module dual-dispatch parity contract tests, grouped by stage family, each independently red or green, reusing packet 204's seam and comparator.
- New parity comparators for the stage families 204 did not demonstrate (`PostPass::LayerFinalization`, `PrePass::SeamPlanning`, `PrePass::LayerPlanning`), each with self-tests proving non-vacuity.
- Sixteen `integrated-<name>` passthrough features on `crates/pnp-cli/Cargo.toml` (packet-205 AC-7 form).
- Proof that the coverage gate now reports exactly the two transport-blocked modules.
- Doc edits per `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- `path-optimization-default` and `machine-gcode-emit` — their native transports return a fatal error today; packet 205b completes those transports and integrates them.
- Editing `dist/editions.toml` or changing edition membership — this packet only makes more modules registry-available.
- Editing any `modules/core-modules/<name>/src/lib.rs` — symbol lookups only.
- Dispatch routing, macro emission, marshalling (202); the CLI surface (203); `cargo xtask dist` (205).
- ADR-0057 phase 4 (platform builds) — explicitly deferred.
- `cargo test --workspace` in CI — the existing narrow-crate strategy is untouched.

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read; Decision items 4-5.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read; the Integrated-edition "every core module" clause.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — long; direct read of §Decision only.
- `docs/spec_packets/204-hybrid-pilot-parity/packet.spec.md` and `design.md` — whole files; the pattern to replicate.
- `crates/slicer-wasm-host/src/marshal/native.rs` — the committed-vs-fatal stage dispatch.
- `CONTEXT.md` — terms **Integrated module**, **External module**; delegate a `FACT` lookup.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`. Measurable refinements not restated in their Given/When/Then text:
  - AC-1/AC-2 must derive the expected registration count from the union of the pilot set and this packet's set (the registered set), never a literal count — a hardcoded count rots the moment a module is added or removed.
  - AC-3 is the packet's core deliverable: sixteen independently red-or-green parity gates. A single combined test that cannot attribute a failure to a module is a defect.
  - AC-5 is the coverage-gate proof: it must name exactly `path-optimization-default` and `machine-gcode-emit`, and must NOT name any of the sixteen.
  - AC-6 verifies the feature **body**, not the feature name.
- Negative: `AC-N1` through `AC-N3`. `AC-N1` proves the new per-module gates are not vacuous; `AC-N2` proves the integration never bypasses a user's external override; `AC-N3` proves the coverage gate still fires for any module this packet does not cover.
- Cross-packet impact: `crates/pnp-cli/Cargo.toml` gains sixteen features whose targets are this packet's `slicer-integrated-modules` features. `dist/editions.toml` and `xtask/src/editions.rs` are read but never edited. Nothing in `crates/slicer-runtime`, `crates/slicer-scheduler`, or `crates/slicer-wasm-host` (beyond the read-only marshal) is touched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids` | AC-2 | FACT pass/fail |
| `sh -c 'for t in integrated_parity_fuzzy_skin integrated_parity_gyroid_infill integrated_parity_infill_linker integrated_parity_layer_planner integrated_parity_lightning_infill integrated_parity_overhang_classifier integrated_parity_part_cooling integrated_parity_rectilinear_infill integrated_parity_seam_placer integrated_parity_seam_planner integrated_parity_skirt_brim integrated_parity_support_surface_ironing integrated_parity_top_surface_ironing integrated_parity_traditional_support integrated_parity_tree_support integrated_parity_wipe_tower; do cargo test -p slicer-runtime --test contract -- $t >/dev/null 2>&1 || { echo "FAIL: $t"; exit 1; }; done; echo PASS'` | AC-3 | FACT `PASS` / failing test name |
| `cargo test -p xtask editions_config_declares_three_editions` | AC-4 | FACT pass/fail |
| AC-5's `sh -c` command (see `packet.spec.md`) | coverage gate names exactly the two blocked modules | FACT `PASS` / `FAIL` |
| AC-6's `sh -c` command (see `packet.spec.md`) | pnp-cli passthrough feature **bodies** delegate correctly | FACT `PASS` / `FAIL` |
| AC-7's `sh -c` command (see `packet.spec.md`) | no rayon in the sixteen crates | FACT `PASS` / `FAIL` |
| `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_loop` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` | AC-N2 | FACT pass/fail |
| `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature` | AC-N3 | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness before any parity run | FACT clean / `STALE:` list |

## Step Completion Expectations

- **The native-transport scope is load-bearing.** No step may integrate a module whose stage the native marshal does not commit. If a parity test hits a native-path fatal `Err`, that module belongs in 205b, not here — do not weaken the gate to admit it.
- **The coverage-gate proof is the packet's headline.** AC-5 must show the gate now names exactly the two transport-blocked modules. A gate that still names any of the sixteen means a passthrough feature or registry entry is missing.
- **Each parity gate is independently red or green.** A combined test that cannot attribute a failure is a defect.
- **Re-derive, never quote, the module set.** No step may write the sixteen module names into code, CI, or docs from memory; every consumer reads them from the registry or from `dist/editions.toml` via `load_editions`.

## Context Discipline Notes

- `crates/slicer-wasm-host/src/marshal/native.rs` — read the stage dispatch (lines ~390-870) only; never the whole file.
- `docs/01_system_architecture.md` is large. Locate §"Producing the tier-4 layout: `cargo xtask dist`" by heading text and read a bounded window around it.
- `modules/core-modules/<name>/src/lib.rs` — read only the `#[slicer_module]` type and SDK trait, by `rg`; never the whole file.
- `target/`, `Cargo.lock`, and `modules/core-modules/*/wit-guest/Cargo.lock` are never loaded.
