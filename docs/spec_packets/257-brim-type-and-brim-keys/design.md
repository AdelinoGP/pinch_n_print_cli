# Design: brim-type-and-brim-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/skirt-brim/src/lib.rs` — `SkirtBrim::from_config` (typed key reads), `SkirtBrim::run_finalization` (live brim arm, `brim_width > 0` gate), `generate_brim_entities` (bbox rect loops); the `#[slicer_module]` `FinalizationModule` impl is the live host path.
- Neighboring tests/fixtures: `modules/core-modules/skirt-brim/tests/{skirt_brim_tdd.rs, finalization_live_tdd.rs, slicer_module_binding_tdd.rs}`; `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (the schema-guard pattern); `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (real-manifest bound arms); `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (CONFIG_BLOCK assertions via `run_pipeline_with_raw_config`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The live host path is `run_finalization` (builder API); the legacy `process()` arm is test-only (`TODO(packet-41)` in the source). The `no_brim` gate must land on both, because the invariants (AC-2/AC-3) are asserted on `run_finalization` while `process()` remains the module's test-facing wrapper — a gate touching only one arm would make the two paths disagree.
- Config reachability: the host's `ConfigView::from_declared` filter drops keys not declared in the manifest — `from_config` must add the `brim_type` arm (matching `ConfigValue::String`) or the gate silently never fires; the other four keys need no `from_config` arm (they are not read by `lib.rs` in this packet).
- Bound/enum enforcement is host-side and generic: declaring `values = [...]` on an enum key in the manifest is sufficient for `ConfigBoundsIndex` rejection (evidence: `rejects_unknown_support_style_value` in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`); no scheduler code change is in scope, only the new real-manifest test arm.
- CONFIG_BLOCK: `serialize_config_block` emits raw-config keys (sorted) then padding via `emit_config_kv`'s `emitted` BTreeSet dedup — an explicit user `brim_type` value wins and emits exactly once. Do NOT edit `ORCA_CONFIG_PADDING` (packet 254 precedent: padding removal rejected; packet 255 ruling: "padding only fills absent keys"). Padding stays the defaults source because manifest bool/int/float/enum defaults do NOT thread into raw config (only percent/float_or_percent defaults thread via `bounds.schema_defaults()`, packet-185 machinery — none of this packet's five keys are percent-typed).
- Manifest key naming stays snake_case everywhere (repo convention; loader key strings and manifest table names must match).
- <!-- snippet: wasm-staleness --> - Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. This packet's manifest edits are fingerprint inputs (`guest_input_paths` charges `*.toml` under each module dir, `xtask/src/build_guests.rs`); guest rebuild rides the step that edits the manifest.

## Code Change Surface

- Selected approach: declare-then-gate. The manifest declares all five keys with canonical types/defaults/bounds (making them user-configurable, sidecar-ingestible, bound-enforced, and 3MF-classifiable); the single live decision point (`no_brim` suppression) is wired inside `SkirtBrim`; the four keys without in-tree decision points ride as declared-with-gap, each with its canonical consumer named and the missing geometry described in `requirements.md` §Per-Key Canonical Evidence.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/skirt-brim/skirt-brim.toml` — five new `[config.schema.*]` tables (exact fields per AC-1).
  - `modules/core-modules/skirt-brim/src/lib.rs` — `SkirtBrim` gains a `brim_type: BrimType` field (new private enum in the same file, 7 variants mirroring canonical order); `from_config` reads `config.get("brim_type")` (`ConfigValue::String`) with `auto_brim` fallback; `run_finalization`'s brim arm becomes `if self.brim_width > 0.0 && self.brim_type != BrimType::NoBrim`; the legacy `process()` arm gets the identical condition in the same step.
  - `modules/core-modules/skirt-brim/tests/brim_config_schema_tdd.rs` — NEW: TOML-parsing schema guard (clone of the `cooling_config_schema_tdd.rs` pattern against `skirt-brim.toml`).
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` — new tests: `no_brim` suppression (AC-2), default-identity (AC-3), `brim_width = 0` non-interference (AC-N1).
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — new arm loading the real `skirt-brim` manifest: rejects `brim_type = "elephant"` and `brim_object_gap = 3.0` (AC-4).
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — new assertion: with `brim_type` in raw config, `; brim_type = <value>` appears exactly once (AC-5).
  - `docs/15_config_keys_reference.md` — regenerated by `cargo xtask gen-config-docs` (generated file; the only doc edit).
- Rejected alternatives and reasons:
  - Removing the `brim_type`/`brim_object_gap` padding entries once declared — rejected: defaults don't thread into raw config for these types, so removal would blank the keys from CONFIG_BLOCK output at defaults (regression vs today) and contradict the packet-254/255 rulings.
  - Wiring `brim_object_gap` as a bbox-level inset (shrink the innermost loop by the gap) — rejected for this packet: it invents divergent semantics (gap-to-bbox, not gap-to-object) without a canonical counterpart; the honest state is declared-with-gap until contour-based brim exists.
  - Full mode matrix implementation (ears/painted/inner) — rejected: no geometry backend; inventing placeholder geometry would break the parity standard (invariants would pin invented behavior).
  - Co-declaring the keys into `machine-gcode-emit.toml` — rejected: that pattern is for cross-module placeholder reads (packet 253's header/footer keys); these five keys are consumed only by `skirt-brim` and the CONFIG_BLOCK serializer (which reads raw config, not module ConfigView).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `modules/core-modules/skirt-brim/skirt-brim.toml` - role: owner manifest (declaration of record); expected change: +5 `[config.schema.*]` tables.
- `modules/core-modules/skirt-brim/src/lib.rs` - role: the wired decision point; expected change: +`BrimType` enum (~12 lines), +1 field, +2 read arms, +2 gate conditions.
- `modules/core-modules/skirt-brim/tests/brim_config_schema_tdd.rs` - role: schema guard (new file); expected change: new test file (pattern-copied).
- Extra (justified — each is a narrow test file append, not a logic change): the scheduler and runtime integration test files named in §Code Change Surface; `modules/core-modules/skirt-brim/Cargo.toml` (dev-deps only: `toml = "0.8"` for the schema guard, mirroring part-cooling); `docs/15` regeneration via xtask (no hand edits).

## Read-Only Context

- `modules/core-modules/skirt-brim/src/lib.rs` - full file (~421 lines) at Step 2 only - purpose: exact current gate/constructor shape; do not re-read in later steps.
- `crates/slicer-gcode/src/serialize.rs` - lines 315–545 only - purpose: CONFIG_BLOCK emission + padding dedup semantics for AC-5 placement.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full file (~120 lines) - purpose: the schema-guard test pattern to copy.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - the real-manifest arms only (`manifest_declared_bound_rejects_out_of_range_value`, `rejects_unknown_support_style_value`) - purpose: the loader+bounds arm pattern for AC-4.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented` — see `requirements.md` §OrcaSlicer Reference Obligations)
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (edit) - out of bounds; padding stays, dedup suffices
- `crates/slicer-scheduler/src/**` - out of bounds (generic enforcement needs no change)
- `crates/slicer-ir/**`, `crates/slicer-schema/wit/**`, other modules' manifests - out of bounds (no IR/WIT/cross-module surface)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: "does the new `brim_config_schema_tdd` guard compile and assert exactly AC-1's five tables?"; scope: `modules/core-modules/skirt-brim/tests/brim_config_schema_tdd.rs` + `skirt-brim.toml`; return: `FACT pass/fail`; purpose: Step 1 exit.
- Question: `cargo xtask build-guests --check` exit code after manifest+src edits; scope: `xtask` run only; return: `FACT exit code`; purpose: Steps 2, 3 staleness gates.
- Question: doc-15 regenerated tables contain the five keys with canonical defaults; scope: `cargo xtask gen-config-docs --check`; return: `FACT exit code`; purpose: Step 4.
- Question: preflight verdict; scope: this packet directory; return: `FACT PREFLIGHT PASS|BLOCKED + findings`; purpose: final gate (Step 5).

## Data and Contract Notes

- IR/manifest contracts: manifest `[config.schema]` tables are the declaration of record consumed by `load_module_from_paths` (`crates/slicer-scheduler/src/manifest.rs`) → `ConfigBoundsIndex` (bounds + enum validation) → `ConfigView::from_declared` filter; no IR schema or wire-version change (values are `ConfigValue::String`/`Float`/`Bool` in the existing map — no enum variant added).
- WIT boundary: unchanged — `ConfigView::get` is a plain key-value lookup (`crates/slicer-ir/src/slice_ir.rs`); no `[slicer_module]` macro, WIT, or guest-code change for new keys.
- Determinism/scheduler constraints: `brim_type` is read once at `from_config` (pure function of config); the gate introduces no ordering or layer-parallel hazards (module already declares `layer-parallel-safe = false` for other reasons — unchanged).

## Locked Assumptions and Invariants

- The five declared defaults are canonical-identical (verified against `PrintConfig.cpp` at authoring time; see `requirements.md` §Per-Key Canonical Evidence) — no deviation rows.
- Default-path identity: with no explicit `brim_type`, `run_finalization` output is unchanged (AC-3) — the packet's only default-behavior risk is the AC-5 dedup path, which preserves current CONFIG_BLOCK content.
- Padding table contents are frozen (packet-254/255 precedent).
- None of the five keys are percent-typed, so no packet-185 `schema_defaults` threading is exercised by this packet; do not add percent machinery speculatively.

## Risks and Tradeoffs

- Risk: `inner_only`/`outer_and_inner`/`brim_ears`/`painted` modes are accepted by validation but all resolve to the outer bbox-loop path (degraded, recorded) — users could expect mode-distinct behavior. Mitigation: gap rows are explicit in `requirements.md`; the eventual mode-aware packets are the consumer of record.
- Risk: the scheduler's generic enum validation (`brim_type` values list) must match the manifest exactly; a typo ("outer-only") would enforce the wrong vocabulary. Mitigation: AC-1's guard pins the exact 7-string list, copied verbatim from canonical order.
- Tradeoff: declaring without wiring leaves four keys user-visible but inert — the honest alternative (not declaring) would keep 3MF values silently dropped and the queue entry unstartable; declared-with-gap matches established packet precedent (253's `dont_slow_down_outer_wall`, 254's 12-key disposition).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `S` (Step 2, the gate edit, is bounded by the ~60-line relevant range of `lib.rs` already excerpted in this design)
- Highest-risk dispatch and required return format: AC-4's scheduler arm — `FACT pass/fail` with bounded failure SNIPPETS ≤ 20 lines (the integration bucket must re-register the new arm if the file uses an explicit `main.rs` registry — verified pattern exists: `integration/main.rs` registries).

## Open Questions

- None. (`[BLOCK]`: none — no activation blockers; authoring-time user rulings are recorded in `requirements.md` §Out of Scope.)