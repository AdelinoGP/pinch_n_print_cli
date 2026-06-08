# Design: 91_paint-pipeline-schema-scaffolding

## Controlling Code Paths

- Primary code paths: `crates/slicer-ir/src/slice_ir.rs` (IR type additions, schema-version bumps, helper accessors), `crates/slicer-ir/src/resolved_config.rs` (BTreeMap migration + Hash derive), `crates/slicer-runtime/src/builtins/*.rs` (seven `BuiltinProducer` constant updates), four production call sites in `crates/slicer-runtime/src/`, and the `HashablePaintValue` deletion in `crates/slicer-core/src/algos/paint_segmentation.rs`.
- Neighboring tests or fixtures: `crates/slicer-ir/tests/` for new IR-shape unit tests; `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` MAY observe the schema change but their assertions are unchanged (they target the *future* populated `variant_chain`, which is still empty after this packet).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- IR-version bump invariant: `BuiltinProducer.min_ir_schema` stays at `1.0.0` (not bumped to `2.0.0`) — admitting both 1.x and 2.x lets in-flight test guests that still compile against 1.x keep working through the scaffolding window. `max_ir_schema` stays at `4.0.0` (unchanged) so headroom for future bumps remains.
- Hash invariant on `ResolvedConfig`: the implementation MUST hash f32 fields via `to_bits()`, never via float equality. The interner identity check uses the derived Hash; two configs that differ only in a NaN payload's bit pattern would otherwise collide. Document this as "consistent within one process; not portable across architectures with differing NaN bit patterns" in the type's doc-comment.
- Behavior preservation invariant: every existing test passes byte-identically. The interner produces a 1-entry `configs` Vec on legacy single-config flows; `ConfigId(0)` resolves to that single config; `RegionMapIR::config_for` returns the same `&ResolvedConfig` reference layout the previous direct-inline shape did.

## Code Change Surface

- Selected approach: land IR shape changes in dependency order — first the BTreeMap migration (Hash prerequisite), then the Hash derives on PaintValue and ResolvedConfig, then the field additions on RegionKey / SlicedRegion / RegionMapIR / RegionPlan, then the schema-version bumps, then the production call-site migrations, then the HashablePaintValue deletion, then the boundary_paint → segment_annotations sweep across tests. Each sub-step keeps the workspace compilable at HEAD.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-ir/src/resolved_config.rs`**:
    - `ResolvedConfig.extensions: HashMap<String, ConfigValue>` → `BTreeMap<String, ConfigValue>` (or whatever the current value type is).
    - Add `#[derive(Hash, Eq, PartialEq)]` to `ResolvedConfig` (the existing derives keep). If any f32 field blocks the derive, write a manual `impl Hash` that calls `to_bits()` on each f32 field.
    - Doc-comment on `Hash`: "consistent within one process; not portable across architectures with differing NaN bit patterns".
    - Any constructor / builder that touched `extensions` must use `BTreeMap` ops (`.insert`, `.get`, `.iter` — most are interchangeable; the deterministic ordering is the upside).
  - **`crates/slicer-ir/src/slice_ir.rs`** (largest concentration of changes):
    - Add `impl Hash for PaintValue` (Scalar via `to_bits`, Custom via `String`, others by discriminant). If `PaintValue` already derives Hash, verify the Scalar variant doesn't poison the derive.
    - Add `#[derive(...Hash, Eq, PartialEq...)]` (or manual impl) on `PaintValue`.
    - Add field `variant_chain: Vec<(String, PaintValue)>` to `RegionKey`. Default impl produces empty Vec. `Hash + Eq` derives still apply.
    - Add `pub struct ConfigId(pub u32);` newtype with `#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]`.
    - Add field `configs: Vec<ResolvedConfig>` to `RegionMapIR`.
    - Change `RegionPlan.config: ResolvedConfig` → `RegionPlan.config: ConfigId`.
    - Add accessor `impl RegionMapIR { pub fn config_for(&self, key: &RegionKey) -> &ResolvedConfig { &self.configs[self.entries[key].config.0 as usize] } pub fn intern_config(&mut self, rc: ResolvedConfig) -> ConfigId { if let Some(i) = self.configs.iter().position(|c| c == &rc) { ConfigId(i as u32) } else { self.configs.push(rc); ConfigId((self.configs.len() - 1) as u32) } }}` (or equivalent). The simple linear scan is acceptable for now — the interner population is bounded by distinct configs per print job (low single-digit thousands at most). A future packet may upgrade to a HashSet-based interner if profiling demands.
    - Rename `SlicedRegion.boundary_paint` → `SlicedRegion.segment_annotations`. Field type unchanged.
    - Add `SlicedRegion.variant_chain: Vec<(String, PaintValue)>` (empty default).
    - Bump `SLICE_IR_SCHEMA: SemVer { major: 1, minor: 0, patch: 0 }` → `{ major: 2, minor: 0, patch: 0 }`.
    - Bump `REGION_MAP_IR_SCHEMA: SemVer { major: 1, minor: 0, patch: 0 }` → `{ major: 2, minor: 0, patch: 0 }`.
  - **`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`, `paint_segmentation_producer.rs`, `slice_producer.rs` (if exists), `shell_classification_producer.rs`, `support_geometry_producer.rs`, `mesh_producer.rs`, `mesh_analysis_producer.rs`**: every producer whose `ir_reads` or `ir_writes` mentions `SliceIR` or `RegionMapIR` keeps `min_ir_schema: { major: 1, minor: 0, patch: 0 }` and updates `max_ir_schema: { major: 4, minor: 0, patch: 0 }` (no change to max if it was already 4). The bump comes from the *current* type being 2.x; the producer admits both old and new.
  - **`crates/slicer-runtime/src/prepass_slice.rs:275`** — `plan.config` direct read → `region_map.config_for(&key)`.
  - **`crates/slicer-runtime/src/slice_postprocess_prepass.rs:348, 390`** — same migration at both sites.
  - **`crates/slicer-runtime/src/layer_executor.rs:783`** — same migration.
  - **`crates/slicer-runtime/src/dispatch.rs:1975, 2009`** — same migration.
  - **`crates/slicer-core/src/algos/paint_segmentation.rs:117`** — delete `struct HashablePaintValue` + any `From`/`Into` impls; rewrite the call site (a `HashMap` keyed by `HashablePaintValue` likely sits nearby) to key on `PaintValue` directly.
  - **~20 test files mentioning `boundary_paint`**: mechanical `replace_all` from `boundary_paint` to `segment_annotations`. Files unknown until Step 1's residual-grep dispatch.
- Rejected alternatives that were considered and why they were not chosen:
  - **`Vec<Arc<ResolvedConfig>>` instead of `Vec<ResolvedConfig>`**: reduces memory if many regions share the same config, but breaks the simple `&ResolvedConfig` accessor signature and complicates serialization. Acceptable if profiling later shows the Vec is hot, but defer to a future packet.
  - **Keep `boundary_paint` name and reinterpret semantics in docs only**: rejected because a downstream module reader looking at `SlicedRegion.boundary_paint` would have no way to know its scope changed. The rename is the documentation.
  - **Bump `BuiltinProducer.min_ir_schema` to 2.0.0**: would force all guests to recompile against 2.x before this packet closes. Rejected — admitting 1.x and 2.x simultaneously is the safer migration; we tighten to 2.x only after every consumer has been upgraded (a future packet, if needed).
  - **Skip BTreeMap migration; write manual `Hash` impl for ResolvedConfig that hashes HashMap by sorting at hash-time**: technically possible but expensive (allocations on every hash) and fragile (any iteration-order-dependent code regression-tests would flake). Rejected.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` — role: primary IR type definitions; expected change: ~10 field additions, ~5 derive updates, ~2 schema constant bumps, ~2 new accessors.
- `crates/slicer-ir/src/resolved_config.rs` — role: ResolvedConfig type; expected change: HashMap → BTreeMap on `extensions`, Hash derive.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — role: producer constant; expected change: schema admission update.
- `crates/slicer-runtime/src/builtins/paint_segmentation_producer.rs` — role: producer constant; expected change: schema admission update.
- `crates/slicer-runtime/src/builtins/<other producer files>` — role: each ≤ 60 LOC; expected change: schema admission update where SliceIR or RegionMapIR is named.
- `crates/slicer-runtime/src/prepass_slice.rs` — role: prepass slice driver; expected change: one line near :275.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — role: slice postprocess driver; expected change: two lines near :348 and :390.
- `crates/slicer-runtime/src/layer_executor.rs` — role: layer executor; expected change: one line near :783.
- `crates/slicer-runtime/src/dispatch.rs` — role: dispatch logic; expected change: two lines near :1975 and :2009.
- `crates/slicer-core/src/algos/paint_segmentation.rs` — role: paint-segmentation kernel; expected change: HashablePaintValue wrapper deletion near :117.
- ~20 test files (precise list determined by Step 1's grep) — role: tests referencing `boundary_paint`; expected change: mechanical rename.

Aggregate exceeds the "≤ 3 primary files" guideline because this is a sweeping schema scaffold — every change is small in delta. The per-step plan in `implementation-plan.md` keeps each step to ≤ 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — read §"P1a — Schema scaffolding" only (lines 290-430 approx).
- `docs/02_ir_schemas.md` — read sections naming the modified types only. If file > 300 lines, delegate a SUMMARY of "all IR types touched by this packet + their schema-version semantics".
- `docs/05_module_sdk.md` — read §"BuiltinProducer" only.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — delegate a SUMMARY, never load.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate any parity check.
- `target/`, `Cargo.lock`, generated code — never load.
- Any path under `modules/core-modules/*/src/**` — this packet does not change module source. The guest WASM rebuilds because the IR shape changes; the implementer does not edit module bodies.
- `crates/slicer-runtime/src/wasm_host.rs`, `dispatch.rs` outside the two narrow call-site line ranges — delegate any inspection.
- `crates/pnp-cli/**` — not touched by this packet; no command-line surface change.

## Expected Sub-Agent Dispatches

- "Run `rg -nE '\bboundary_paint\b' crates/ modules/ docs/ .ralph/`; return LOCATIONS (≤ 30 entries)" — purpose: full inventory before the rename sweep.
- "Run `rg -nE '\bplan\.config\b' crates/slicer-runtime/src/`; return LOCATIONS (≤ 20 entries)" — purpose: confirm the four roadmap-cited call sites are still where the roadmap says + catch any drift.
- "Run `rg -nE 'HashablePaintValue' crates/`; return LOCATIONS (≤ 10 entries)" — purpose: confirm the wrapper deletion catches every site.
- "Run `cargo check --workspace --all-targets`; return FACT pass/fail with the first compile error if any" — purpose: gate after each major shape change.
- "Run `cargo test -p slicer-ir 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: validate IR-level shape.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-N2.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p91-wedge.gcode && sha256sum /tmp/p91-wedge.gcode`; return FACT with the sha256 hash" — purpose: AC-10 byte-identical check.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289`; return SUMMARY confirming `PaintedRegion` / `FuzzySkinPaintedRegion` uses a parent region pointer + paint discriminator" — purpose: parity confirmation.

## Data and Contract Notes

- IR or manifest contracts touched: `SliceIR` 1.0.0 → 2.0.0; `RegionMapIR` 1.0.0 → 2.0.0. Breaking renames + field additions. All current `BuiltinProducer`s admit 1.x and 2.x simultaneously through the scaffolding window.
- WIT boundary considerations: `slicer-ir` types feed `slicer-macros` via `slicer-schema` → guest bindgen output. Guests must rebuild after this packet; that's why `cargo xtask build-guests --check` is in AC-N2.
- Determinism or scheduler constraints: the BTreeMap migration makes config iteration deterministic (previously the HashMap iteration order varied across runs). Any production code that relied on a *specific* HashMap iteration order is now deterministic; any that relied on iteration-order *variation* would break, but no such code is expected (deliberately-non-deterministic code is anti-pattern).

## Locked Assumptions and Invariants

- **Behavior preservation**: every existing test must pass byte-identically. Any g-code diff against the pre-packet baseline is a bug.
- **Hash invariant**: `ResolvedConfig::hash` produces consistent values within one process. Cross-architecture or cross-process portability is NOT guaranteed (NaN bit patterns differ). The interner is scoped to a single prepass invocation, so this is acceptable.
- **Empty-default invariant**: `RegionKey::default()` and `SlicedRegion::default()` produce empty `variant_chain`. Any code path that constructed these with non-empty variant_chain before this packet did not exist (P1c is the first packet to populate it).
- **Schema admission invariant**: `BuiltinProducer.min_ir_schema` is the LOWEST schema admitted, not the current. Keeping it at 1.0.0 is by design — admits old test guests during the scaffolding window.

## Risks and Tradeoffs

- **Risk: a g-code diff appears against the pre-packet baseline** because BTreeMap iteration order differs from the prior HashMap order in a path that emits g-code. Mitigation: AC-10 captures the baseline before this packet starts and compares post-packet; any diff blocks closure until root-caused. Most likely culprits: `extensions` iteration in a producer or g-code emitter.
- **Risk: a guest module's bindgen output silently keeps the old IR shape** because its `target/` directory wasn't cleaned. Mitigation: AC-N2's `--check` catches this; if STALE: is reported, rebuild without `--check` and re-test.
- **Risk: a downstream call site reads `RegionPlan.config` via a method this packet didn't catch** because the grep dispatch missed a pattern (e.g., `let cfg = &plan.config;` instead of `plan.config.method()`). Mitigation: the `cargo check --workspace --all-targets` gate catches type errors; the change from `ResolvedConfig` to `ConfigId` is the most error-surfacing single edit in the packet.
- **Tradeoff: the linear-scan interner is O(N) per intern call.** For N up to a few thousand distinct configs (per print job), this is fine. A HashSet-based interner could be O(1) per intern but requires `ResolvedConfig: Hash + Eq` and a separate lookup structure. Defer until profiling.

## Context Cost Estimate

- Aggregate: `M` (the work is mechanical but broad).
- Largest single step: `M` (Step 3 — the IR field additions in `slice_ir.rs`).
- Highest-risk dispatch: the AC-10 baseline comparison (`sha256sum /tmp/p91-wedge.gcode` vs pre-packet baseline). The pre-packet baseline must be captured BEFORE Step 1, not after — if captured post-packet by accident, the comparison is meaningless.

## Open Questions

- `[FWD]` — Are the four production call-site line numbers (275, 348, 390, 783, 1975, 2009) still accurate at implementation time? They were captured from the roadmap; the first dispatch (`rg -n 'plan\.config' crates/slicer-runtime/src/`) confirms current line numbers.
- `[FWD]` — Does `ResolvedConfig` currently have any field that cannot be wrapped in a Hash-friendly form (e.g., a `Vec<f32>` that needs bit-by-bit hashing)? Step 2 dispatch confirms; if so, a manual `impl Hash` substitutes for the derive. Resolvable mid-flight.
- `[BLOCK]` — None. The packet has no activation blocker beyond no-other-packet-active.
