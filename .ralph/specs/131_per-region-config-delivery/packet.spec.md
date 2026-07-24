---
status: implemented
packet: 131_per-region-config-delivery
task_ids:
  - TASK-256
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 131_per-region-config-delivery

## Goal

Deliver per-region config to guest modules: replace the first-match global `ConfigView`
derivation with `RegionKey`-matched resolution from `RegionMapIR`'s interned pool, expose it
through a config accessor on the region views (additive WIT bump), and open the roadmap's
golden carve window with a baseline survey.

## Scope Boundaries

This packet changes config *delivery* (dispatch derivation + view accessor + SDK surface) and
performs the golden survey/carve that packet 136 later restores. It does NOT split any
geometry (packet 132), does not change any config *values* or defaults, and does not touch
any infill algorithm. Multi-region layers may legitimately change output (they currently read
an arbitrary region's config); single-region output must be byte-identical.

## Prerequisites and Blockers

- Depends on: FORWARD-DEP on draft `130_infill-postprocess-contract` (adjacent WIT churn;
  this packet's world-version bump is computed as +0.1 from whatever 130 lands as, not a
  hardcoded `1.1.0 → 1.2.0` — 130 is `status = draft` at authoring time, 2026-07-10; the live
  `world-layer` version today is still `1.0.0`).
- Unblocks: `132_modifier-region-split` (sub-regions are useless without per-region config),
  `133_infill-linker-module` (per-region spacing), `134`/`135` (modules read per-region
  density from day one).
- Activation blockers: packet 130 must reach `status: implemented` before this packet
  activates (serial WIT-churn ordering; version arithmetic is otherwise undefined).

## Acceptance Criteria

- **AC-1. Given** a layer whose `RegionMapIR` pool holds two entries for the same layer with
  `infill_density` 0.15 and 0.40, **when** a test guest reads `infill_density` through the
  region-view config accessor inside its per-region loop, **then** it reads 0.15 for the
  first region's key and 0.40 for the second — not the same value twice. | `cargo test -p slicer-runtime --test contract -- per_region_config_two_densities 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the host dispatch, **when** deriving a region's config, **then** the entry
  is selected by full `RegionKey` match (object + region + variant chain), not by
  `.find(|key| key.global_layer_index == layer.index)` first-match — the first-match
  derivation at `crates/slicer-wasm-host/src/dispatch.rs:1640` is gone. (The looser substring
  `global_layer_index == layer` also matches two unrelated sites — `:1378`
  `push_perimeter_regions`'s seam-plan lookup, `:1680` a separate `held_claims_map`
  resolution — both out of this packet's scope; the AC greps the exact `.find(...)`
  expression, not the substring.) | `rg -n '\.find\(\|key\| key\.global_layer_index == layer\.index\)' crates/slicer-wasm-host/src/dispatch.rs | wc -l | grep -qx '0' && echo GONE`
- **AC-3. Given** the canonical WIT, **when** greping the region views, **then** both
  `slice-region-view` and `perimeter-region-view` expose a `config: func() -> config-view`
  method (locked shape — reuses the existing `slicer:config/config-types.config-view`
  resource via a new `use` in the `ir-handles` interface, rather than duplicating six
  per-key getters on two resources). | `rg -c 'config: func\(\) -> config-view' crates/slicer-schema/wit/deps/ir-types.wit | grep -qx '2' && echo OK`
- **AC-4. Given** the pre-change baseline, **when** the carve survey completes, **then**
  `.ralph/specs/131_per-region-config-delivery/carve-list.md` exists with one `### <test
  path>` heading per carved test, each immediately followed by a `- Reason: ...` line and a
  `- Baseline: <SHA/assertion>` line (heading count == Baseline-line count, so every carved
  entry actually records its baseline). | `test -s .ralph/specs/131_per-region-config-delivery/carve-list.md && [ "$(rg -c '^### ' .ralph/specs/131_per-region-config-delivery/carve-list.md)" = "$(rg -c '^- Baseline: ' .ralph/specs/131_per-region-config-delivery/carve-list.md)" ] && echo OK`

## Negative Test Cases

- **AC-N1. Given** a single-region layer, **when** a module reads any config key through the
  new accessor and through its module `ConfigView`, **then** the values are identical (the
  only `RegionKey` is the first match — behavior unchanged). | `cargo test -p slicer-runtime --test contract -- per_region_config_single_region_unchanged 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** `resources/regression_wedge.stl` (unpainted, no modifiers — single region
  per layer) sliced with default config, **when** `wedge_per_region_config_delivery_byte_identical`
  computes the SHA-256 hex digest of the emitted g-code (via `sha2::{Sha256, Digest}`,
  already a `slicer-runtime` dependency), **then** it equals the pre-packet baseline digest
  captured in Step 1 and hardcoded as a literal constant in that test. | `cargo test -p slicer-runtime --test e2e -- wedge_per_region_config_delivery_byte_identical 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/modifier-region-infill.md` — §Phase M2 (short file; load in full).
- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — Decision point 3 (short; full).
- `docs/02_ir_schemas.md` — delegate; `RegionMapIR` section only.
- `CLAUDE.md` §WIT/Type Changes Checklist + §Guest WASM Staleness — binding.

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` §region views — config accessor + world version note — `rg -q 'region.*config accessor|per-region config' docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md` §per-region config — how a module reads config inside its region
  loop — `rg -q 'per-region config' docs/05_module_sdk.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
