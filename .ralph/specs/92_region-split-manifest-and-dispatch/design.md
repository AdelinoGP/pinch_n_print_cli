# Design: 92_region-split-manifest-and-dispatch

## Controlling Code Paths

- Primary code paths: `crates/slicer-scheduler/src/manifest.rs` (parser extensions + new validators), `crates/slicer-scheduler/src/` (aggregation logic — likely a new sibling module `region_split.rs` or extension of an existing scheduler-startup module), `crates/slicer-schema/src/lib.rs` (priority constants), and the dispatch hook at `crates/slicer-runtime/src/layer_executor.rs:494-528`.
- Neighboring tests or fixtures: `crates/slicer-scheduler/tests/` (new fixture directory `region_split_manifests/` with synthetic TOML), `crates/slicer-runtime/tests/integration/` (new test files for the dispatch filter + empty-polygon guard). No existing test is modified.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (PrintApply cross-product structure).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Determinism invariant: `aggregated_region_split` is a `BTreeMap`, not a `HashMap`. The canonical variant-chain order depends on deterministic iteration; a `HashMap` would produce non-deterministic chains across process runs, breaking byte-identical g-code reproducibility.
- Validation invariant: every new `ManifestParseError` variant carries the manifest path. Errors without source identification are usability bugs; the existing error variants follow this convention (see `crates/slicer-scheduler/src/manifest.rs`'s existing errors).
- Filter-vs-guard order invariant: the empty-polygon guard fires BEFORE the host filter (paint-transparent modules still skip empty regions). The simpler order (guard → filter) is also the cheaper one (no string set lookup on empty regions).
- Behavior preservation invariant: with no core module declaring `[[region_split]]`, the filter never excludes anyone; with empty `variant_chain` on every region (P1a default; P1c populates), the filter has nothing to match against; net effect is "every existing test passes byte-identically".

## Code Change Surface

- Selected approach: build the parser-side and registry first, then the validator suite (each validator has its own test fixture), then the aggregation function, then the dispatch hook. Tests are added alongside each step (TDD-style for the validators).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-schema/src/lib.rs`** (or wherever public constants live):
    - `pub const CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)] = &[("material", 100), ("fuzzy_skin", 200)];`
    - `pub const COMMUNITY_PRIORITY_FLOOR: u32 = 1000;`
    - Both with doc-comments explaining the registry semantics.
  - **`crates/slicer-scheduler/src/manifest.rs`** (or equivalent — locate the manifest types crate via `Grep` for `ManifestEntry`):
    - New struct `pub struct RegionSplitDeclaration { pub semantic: String, pub priority: u32, pub value_type: RegionSplitValueType }`. Derives: `Debug, Clone, PartialEq, Eq, serde::Deserialize`.
    - New enum `pub enum RegionSplitValueType { Flag, ToolIndex, CustomString }`. Derives: `Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize` with `#[serde(rename_all = "snake_case")]`.
    - `ManifestEntry` gains `pub region_splits: Vec<RegionSplitDeclaration>` (default empty).
    - Update the TOML parser's deserialize wrapper to extract `[[region_split]]` arrays.
    - New `ManifestParseError` variants: `DuplicateRegionSplitSemantic { semantic, manifest_path, first_line, second_line }`, `ScalarValueTypeNotAllowedInRegionSplit { semantic, manifest_path }`, `CommunityPriorityBelowFloor { semantic, given_priority, floor: u32, manifest_path }`, `CorePriorityMismatch { semantic, given_priority, expected_priority, manifest_path }`. Each variant carries enough context to surface in user-facing diagnostics.
    - Validators wired into the post-deserialize pass (the existing manifest-load pipeline has a similar pass for other invariants — extend it).
  - **`crates/slicer-scheduler/src/region_split.rs`** (NEW file, ≤ 200 LOC):
    - `pub struct AggregatedRegionSplitEntry { pub priority: u32, pub value_type: RegionSplitValueType, pub declaring_modules: Vec<ModuleId> }`.
    - `pub fn aggregate_region_splits(manifests: &[ManifestEntry]) -> BTreeMap<String, AggregatedRegionSplitEntry>` — produces the canonical aggregation.
    - Cross-manifest tied-priority WARN emission via the existing structured-event channel (`progress_events` per `docs/09`).
    - `pub fn canonical_variant_chain_order(agg: &BTreeMap<String, AggregatedRegionSplitEntry>) -> Vec<String>` — convenience accessor returning semantic names in `(priority, name)` order.
  - **`crates/slicer-runtime/src/layer_executor.rs:494-528`** — the dispatch hook:
    - Each module dispatch site within the per-region loop:
      ```rust
      // BEFORE invoking module:
      if region.polygons.is_empty() {
          emit_debug_event(EmptyPolygonGuard { layer, region: region.key.clone() });
          continue;
      }
      if !module_invocation_allowed(&module_meta, &region.key.variant_chain) {
          continue;
      }
      ```
    - `fn module_invocation_allowed(meta: &ModuleMetadata, chain: &[(String, PaintValue)]) -> bool`: returns true if `meta.region_splits.is_empty()` (paint-transparent), OR if any `(s, _) ∈ chain` has `s == d.semantic` for some `d ∈ meta.region_splits`. Implementation note: build a small HashSet of declared semantics on module-load and store on the `ModuleMetadata` to avoid per-region O(N*M) cost.
  - **`crates/slicer-scheduler/tests/fixtures/region_split_manifests/`** (NEW directory):
    - `basic.toml` — one valid `[[region_split]]` entry for AC-1.
    - `duplicate_semantic.toml` — two entries with same `semantic` for AC-3.
    - `scalar_value_type.toml` — `value_type = "scalar"` for AC-4.
    - `community_below_floor.toml` — community semantic at priority 250 for AC-5.
    - `core_priority_mismatch.toml` — `material` at priority 100000 for AC-6.
    - `priority_type_mismatch.toml` — `priority = "abc"` for AC-N3.
    - `tied_priorities/manifest_a.toml`, `tied_priorities/manifest_b.toml` — two manifests with distinct semantics tied at priority 1500 for AC-7.
  - **`crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs`** (NEW):
    - Builds the synthetic scenario (two modules, two regions, varying variant_chain), asserts the recorded invocation set matches AC-9's expectation.
  - **`crates/slicer-runtime/tests/integration/empty_polygon_dispatch_guard.rs`** (NEW):
    - Builds a region with `polygons.is_empty()`, asserts module is not invoked (AC-10).
- Rejected alternatives that were considered and why they were not chosen:
  - **Single-semantic-per-manifest (top-level `region_split = { ... }` table instead of `[[region_split]]` array)**: a module declaring both `material` and `custom_X` would need two manifests. Rejected — arrays scale.
  - **Aggregate by `Vec<Tuple>` instead of `BTreeMap`**: O(N) lookup per dispatch decision instead of O(log N). Rejected — dispatch is hot.
  - **Per-manifest `priority_floor` override config key**: lets a module bypass the community floor. Rejected — the floor is a contract guard against priority squatting; making it tunable defeats the purpose. The path to a sub-1000 priority is to land the semantic in `CORE_REGION_SPLIT_PRIORITIES` via a packet (a deliberate, reviewed step).
  - **Move the filter logic into each module's run-loop**: rejected because (a) every paint-aware module would duplicate it and (b) the empty-polygon guard especially is a universal concern.

## Files in Scope (read + edit)

- `crates/slicer-scheduler/src/manifest.rs` (or equivalent located via Grep) — role: manifest parser + validators; expected change: new struct/enum, new error variants, new validation logic.
- `crates/slicer-scheduler/src/region_split.rs` (NEW) — role: aggregation logic; expected change: new file.
- `crates/slicer-schema/src/lib.rs` (or equivalent constants file) — role: priority registry; expected change: two new consts.
- `crates/slicer-runtime/src/layer_executor.rs` (range 480-540 ONLY) — role: dispatch hook; expected change: insert guard + filter before module invocation.
- `crates/slicer-scheduler/tests/fixtures/region_split_manifests/` (NEW) — role: validator test fixtures; expected change: 7 new tiny TOML files.
- `crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs` (NEW) — role: dispatch-filter test; expected change: new file.
- `crates/slicer-runtime/tests/integration/empty_polygon_dispatch_guard.rs` (NEW) — role: guard test; expected change: new file.

Above the ≤ 3 guideline; this packet adds three system pieces (manifest, aggregation, dispatch) plus their tests. The per-step plan keeps each step to ≤ 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — §"P1b" only (range-read).
- `docs/03_wit_and_manifest.md` — §"Module Manifest TOML Schema" only (range-read; file may be > 300 lines).
- `docs/04_host_scheduler.md` — §"Module Dispatch" only (range-read).
- `docs/09_progress_events.md` — read in full only if event-schema details aren't covered by a SUMMARY (file is ~200 lines).
- An existing core-module manifest e.g. `modules/core-modules/seam-planner-default/seam-planner-default.toml` — read as a template for TOML field placement (each manifest is ≤ 60 lines).
- `crates/slicer-runtime/src/layer_executor.rs` — read ONLY lines 480-540 via ranged Read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- Production module sources under `modules/core-modules/*/src/` — no module code changes in this packet.
- `crates/slicer-runtime/src/dispatch.rs` — distinct from `layer_executor.rs`; not in scope (dispatch.rs handles WASM-host dispatch wiring, not per-region invocation control).
- `pnp-cli/**` — no CLI change.

## Expected Sub-Agent Dispatches

- "Locate the manifest TOML parser entry point in `crates/slicer-scheduler/src/`; return FILE:LINE for `fn parse_manifest` or equivalent" — purpose: pinpoint the parser before edit.
- "Run `rg -nE 'ManifestParseError|ManifestParseError::' crates/slicer-scheduler/src/`; return LOCATIONS (≤ 15 entries)" — purpose: locate the error enum.
- "Run `cargo test -p slicer-scheduler region_split 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: validator gate.
- "Run `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: AC-9.
- "Run `cargo test -p slicer-runtime --test integration empty_polygon_dispatch_guard 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: AC-10.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode && sha256sum /tmp/p92-wedge.gcode`; return FACT (sha256)" — purpose: AC-11 byte-identical.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-12.

## Data and Contract Notes

- IR or manifest contracts touched: manifest TOML grammar widens (new optional top-level array). `ManifestEntry` shape changes (new field with default-empty Vec). Existing manifests not declaring `[[region_split]]` deserialize identically.
- WIT boundary considerations: none directly. The `ModuleMetadata` carried into the WASM host is unchanged in *content* (no new WIT type); only the host-side scheduler representation gains a field.
- Determinism or scheduler constraints: `BTreeMap` aggregation is the determinism choice. Lex tiebreaker on equal priority makes order reproducible across runs.

## Locked Assumptions and Invariants

- **Empty-polygon guard fires BEFORE the host filter**: this order is the contract. Reverse-order tests would falsely pass on benign inputs but fail on a region whose `variant_chain` matches the filter AND whose polygons are empty (filter would allow, then module sees empty polygons unexpectedly).
- **`aggregated_region_split` is `BTreeMap`, not `HashMap`**: the BTreeMap iteration order IS the canonical variant-chain order. `HashMap` would silently corrupt this.
- **Core priorities are not user-overridable**: a manifest stating `material = 100` is fine; `material = 999` is rejected. The path to changing a core priority is a code change to `CORE_REGION_SPLIT_PRIORITIES`, which is a reviewable packet.
- **Scalar value type is forbidden in region-split**: D13. The error variant is explicit (`ScalarValueTypeNotAllowedInRegionSplit`); never silently convert to another value type.

## Risks and Tradeoffs

- **Risk: the new TOML grammar extension breaks an existing manifest's parse** if it accidentally collides with an existing top-level array of the same name. Mitigation: `[[region_split]]` is a deliberately namespaced name; grep existing core-module manifests confirms no collision.
- **Risk: the dispatch filter introduces measurable per-region overhead** even when no module declares anything. Mitigation: pre-cache each module's declared-semantics `HashSet` at module-load; the per-region check is `HashSet::iter().any(|s| chain.iter().any(|(cs, _)| cs == s))`, O(|S|+|chain|), tiny.
- **Risk: a manifest validator surfaces a confusing error** for a power user authoring a community module. Mitigation: each error variant includes the manifest path, the field name, the actual value, and the expected value/floor; structured enough for a user-facing diagnostic.
- **Tradeoff: cross-manifest WARN vs. ERROR for tied priorities.** Errors would force resolution at module-author time; warnings keep the system functional. We chose WARN (D6 + lex-tiebreaker) on the principle that tied priorities are usually accidental + auto-resolvable; loud-but-non-fatal surfacing is better than fatal blocking.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4 — the dispatch hook + its two integration tests).
- Highest-risk dispatch: the AC-11 byte-identical hash check vs. P91 baseline. If P91's closure log didn't capture the post-P91 SHA, this packet must capture it at activation (Step 0 baseline) so the comparison is meaningful.

## Open Questions

- `[FWD]` — Where exactly does the existing manifest-load pipeline emit `ManifestParseError`s? The first dispatch in Step 1 locates the function and the error-enum file. Resolvable mid-flight.
- `[FWD]` — Does `slicer-runtime/src/layer_executor.rs` carry a `ModuleMetadata` struct that the filter can hang off, or does each dispatch site re-look-up the metadata? Step 4's pre-edit dispatch confirms.
- `[BLOCK]` — None.
