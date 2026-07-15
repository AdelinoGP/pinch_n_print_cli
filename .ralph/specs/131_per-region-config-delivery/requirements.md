# Requirements: 131_per-region-config-delivery

## Packet Metadata

- Grouped task IDs:
  - `TASK-256`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`RegionMapIR` holds a full per-`RegionKey` `ResolvedConfig` pool
(`crates/slicer-ir/src/slice_ir.rs:1268`), but no guest module can reach it: the dispatch
builds ONE global `ConfigView` from whichever `RegionKey` a BTreeMap iterator yields first for
the layer (`crates/slicer-wasm-host/src/dispatch.rs:1633-1637`). Two consequences: (a) the
flagship modifier-infill use case (different densities per sub-region, ADR-0030) is
impossible — density can never vary per region; (b) a latent bug — painted multi-region layers
read an *arbitrary* region's config today. `extensions` (used by the tool-index precedence
elsewhere in the roadmap) lives on the interned `ResolvedConfig`, resolved via
`RegionMapIR::config_for(&RegionKey)` (`slice_ir.rs:1306`), not on `RegionMapIR` directly. Every downstream infill packet (132–136) assumes
per-region config exists; this packet is the delivery mechanism. It also opens the roadmap's
golden carve window (D6): fixing (b) legitimately changes multi-region fixture output, so the
survey and carve happen here, restored + re-blessed in packet 136.

## In Scope

- Host: `RegionKey`-matched per-region `ResolvedConfig` resolution from the `RegionMapIR`
  pool at dispatch; retire the first-match derivation
  (`dispatch.rs:1629-1650`, the exact expression at line 1640).
- WIT: `config: func() -> config-view` accessor on `slice-region-view` AND
  `perimeter-region-view` (`crates/slicer-schema/wit/deps/ir-types.wit`, reusing the existing
  `slicer:config/config-types.config-view` resource via a new `use` in `ir-handles`); affected
  world version bumped by +0.1 from whatever `130_infill-postprocess-contract` lands as (FORWARD-DEP
  — 130 is `status: draft` at authoring time; mirror the bump on any other world exposing
  these views). Additive only.
- SDK: `SliceRegionView` / `PerimeterRegionView` gain the config accessor; macros glue.
- Contract tests: two-density echo (AC-1), single-region equivalence (AC-N1); wedge SHA guard
  (AC-N2).
- Golden survey + carve: enumerate SHA-pinned / output-shape tests affected by multi-region
  config correction; capture pre-change baselines; mark carved tests
  `#[ignore = "carved: infill-parity D6; restored in packet 136"]`; author
  `carve-list.md` in this packet's directory.
- Full guest rebuild ceremony.

## Out of Scope

- Geometry splits (packet 132), linking (133), module algorithm changes (134/135).
- Any config *value* or default change; any new config key.
- Restoring/re-blessing carved goldens (packet 136).
- The module-level `ConfigView` parameter — it stays (modules may still read module-scoped
  config); the region accessor is additive, not a replacement.

## Authoritative Docs

- `docs/specs/modifier-region-infill.md` §Phase M2 — short; load in full.
- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — short; load in full.
- `docs/02_ir_schemas.md` — delegate; `RegionMapIR` section only (>1000 lines).
- `docs/03_wit_and_manifest.md` — rg-targeted view sections only.
- `CLAUDE.md` §WIT/Type Changes Checklist, §Guest WASM Staleness, §Test-output tee rule.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-4` in `packet.spec.md`. Refinements: AC-1's two densities are
  exactly 0.15/0.40 to make misrouting unmistakable; AC-2 is a structural grep scoped to the
  exact `.find(|key| key.global_layer_index == layer.index)` expression (the two unrelated
  lookalike sites at `dispatch.rs:1378,1680` must not be conflated with it), paired with
  AC-1's behavioral proof; AC-3 locks the accessor shape to `config: func() -> config-view`
  (reuses the existing `config-view` resource rather than duplicating per-key getters) so the
  grep has a fixed, satisfiable target; AC-4's carve-list entries are a machine-checkable
  `### <path>` / `- Reason:` / `- Baseline:` format so heading count == baseline-line count.
- Negative cases: `AC-N1` (single-region equivalence), `AC-N2` (wedge byte-identical via a
  new `wedge_per_region_config_delivery_byte_identical` test that hardcodes the Step-1-captured
  SHA-256 digest as a literal constant and re-hashes post-packet g-code against it — the
  strongest no-regression signal available since the wedge is single-region throughout; no
  prior wedge test compares against a stored baseline, so this test is new, not reused).
- Cross-packet impact: opens the D6 carve window maintained by 132–135 and closed by 136;
  packet 133's linker will read per-region spacing through this accessor.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test contract -- per_region_config 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1 + AC-N1 | FACT + counts |
| `rg -n '\.find\(\|key\| key\.global_layer_index == layer\.index\)' crates/slicer-wasm-host/src/dispatch.rs \| wc -l` | AC-2 structural grep (expect 0; the exact expression, not the loose substring) | FACT count |
| `rg -c 'config: func\(\) -> config-view' crates/slicer-schema/wit/deps/ir-types.wit` | AC-3 structural grep (expect 2) | FACT count |
| `cargo test -p slicer-runtime --test e2e -- wedge_per_region_config_delivery_byte_identical 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N2 wedge SHA-256 baseline guard | FACT |
| `cargo check --workspace --all-targets` | sweep complete | FACT |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` | guest freshness | FACT clean/STALE |
| `test -s .ralph/specs/131_per-region-config-delivery/carve-list.md && echo OK` | AC-4 carve-list exists | FACT |

## Step Completion Expectations

- Cross-step invariant: the pre-change baseline capture (Step 1) MUST complete before any
  code edit — a baseline captured after the dispatch change is worthless. No later step may
  re-run baseline capture. Step 1's baseline capture includes the wedge default-config g-code
  SHA-256 digest (via `sha2::{Sha256, Digest}`, already a `slicer-runtime` dependency); Step 4
  hardcodes that digest as a literal constant in `wedge_per_region_config_delivery_byte_identical`.
- Carved tests must remain enumerable: every `#[ignore]` added by this packet carries the
  exact string `carved: infill-parity D6` so packet 136 can find them mechanically.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-wasm-host/src/dispatch.rs` (read only ~1600-1730), `docs/02_ir_schemas.md`
  (RegionMapIR section only), the e2e/golden test files (delegate the survey — the
  implementer never reads test bodies, only adjudicates the returned inventory).
- Likely temptation reads: `crates/slicer-ir/src/resolved_config.rs` in full — skip; only the
  `RegionMapIR` pool types matter, and they live in `slice_ir.rs:1268`
  (resolve via `RegionMapIR::config_for(&RegionKey)` at `slice_ir.rs:1306`).
- Sub-agent return-format hints: the golden survey dispatch returns LOCATIONS (test path +
  the SHA/assertion string it pins, ≤25 entries); baseline capture returns FACT per fixture
  (SHA strings).
