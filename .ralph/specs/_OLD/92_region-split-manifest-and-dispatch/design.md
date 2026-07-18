# Design: 92_region-split-manifest-and-dispatch

## Controlling Code Paths

- Primary code paths: `crates/slicer-scheduler/src/manifest.rs` (parser extensions, new `LoadErrorKind` variants, new `LoadedModule.region_splits` + `region_split_semantics` fields), `crates/slicer-scheduler/src/region_split.rs` (NEW — aggregation module), `crates/slicer-scheduler/src/execution_plan.rs` (propagation surface — `CompiledModuleStatic.region_split_semantics`), `crates/slicer-schema/src/lib.rs` (priority constants), and the per-layer dispatch guard at `crates/slicer-runtime/src/layer_executor.rs:362` (filter call site; block lines 357-364) inside `execute_single_layer_inner` — preceding `instrumentation.on_module_start` and the existing `runner.run_stage(...)` call at line 394. The filter helper itself (`pub fn module_invocation_allowed_on_layer`) lives at the bottom of `layer_executor.rs` at line 1326.
- Neighboring tests or fixtures: `crates/slicer-scheduler/tests/` (new fixture directory `region_split_manifests/` with synthetic TOML), `crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs` (NEW — per-layer filter integration test). No existing test is modified. No empty-polygon guard test (descoped — see `packet.spec.md` §Scope Boundaries).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (PrintApply cross-product structure).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Determinism invariant: `aggregated_region_split` is a `BTreeMap`, not a `HashMap`. The canonical variant-chain order depends on deterministic iteration; a `HashMap` would produce non-deterministic chains across process runs, breaking byte-identical g-code reproducibility.
- Validation invariant: every new `LoadErrorKind` variant uses the existing `LoadError { kind, path, field, message }` envelope so the manifest path and field are always carried. Errors without source identification are usability bugs; the existing error variants follow this convention (see `crates/slicer-scheduler/src/manifest.rs:437-450`).
- Filter granularity invariant: the dispatch filter is per-(module × layer), NOT per-(module × region). The host has no per-region invocation site; the existing `runner.run_stage(stage_id, layer, &live_module, input)` call at `layer_executor.rs:394` is the only invocation point and hands the module the whole layer. Per-region filtering remains module-internal and is not this packet's responsibility.
- Behavior preservation invariant: with no core module declaring `[[region_split]]`, the filter never excludes anyone; with empty `variant_chain` on every region (P1a default; P1c populates), the filter has nothing to match against; net effect is "every existing test passes byte-identically" — verified by AC-10's baseline-compare command against Step 0's `P91_BASELINE_SHA`.

## Code Change Surface

- Selected approach: build the parser-side and registry first, then the validator suite (each validator has its own test fixture), then the aggregation function, then the per-layer dispatch guard. Tests are added alongside each step (TDD-style for the validators).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-schema/src/lib.rs`**:
    - `pub const CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)] = &[("material", 100), ("fuzzy_skin", 200)];`
    - `pub const COMMUNITY_PRIORITY_FLOOR: u32 = 1000;`
    - Both with doc-comments explaining the registry semantics.
  - **`crates/slicer-scheduler/src/manifest.rs`** (`LoadedModule` at line 29; `LoadError` at 437; `LoadErrorKind` at 450; `ingest_manifest` at 532):
    - New struct `pub struct RegionSplitDeclaration { pub semantic: String, pub priority: u32, pub value_type: RegionSplitValueType }`. Derives: `Debug, Clone, PartialEq, Eq, serde::Deserialize`.
    - New enum `pub enum RegionSplitValueType { Flag, ToolIndex, CustomString }`. Derives: `Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize` with `#[serde(rename_all = "snake_case")]`.
    - `LoadedModule` gains `pub region_splits: Vec<RegionSplitDeclaration>` (default empty) and a derived `pub region_split_semantics: HashSet<String>` cached at module-load for O(1) lookup in the dispatch guard.
    - Update the TOML deserialization in `ingest_manifest` to extract `[[region_split]]` arrays.
    - New `LoadErrorKind` variants (added to the enum at `manifest.rs:450`; existing variants — `NotImplemented`, `Io`, `TomlParse`, `Schema`, `MissingWasm`, `Validation` — untouched): `DuplicateRegionSplitSemantic { semantic, first_line, second_line }`, `ScalarValueTypeNotAllowedInRegionSplit { semantic }`, `CommunityPriorityBelowFloor { semantic, given_priority, floor: u32 }`, `CorePriorityMismatch { semantic, given_priority, expected_priority }`. `LoadError.path` and `.field` already carry manifest source identification.
    - Reuse `LoadErrorKind::Schema` for missing-required-field (no new variant); reuse `LoadErrorKind::TomlParse` for malformed-type (no new variant).
    - Validators wired into the existing post-deserialize validation pass at `crates/slicer-scheduler/src/validation.rs` (follow the `LoadDiagnostic` pattern at `:333, 342`; for hard errors, return a `LoadError`).
  - **`crates/slicer-scheduler/src/region_split.rs`** (NEW file, ≤ 200 LOC):
    - `pub struct AggregatedRegionSplitEntry { pub priority: u32, pub value_type: RegionSplitValueType, pub declaring_modules: Vec<ModuleId> }`.
    - `pub fn aggregate_region_splits(modules: &[LoadedModule], diagnostics: &mut Vec<LoadDiagnostic>) -> BTreeMap<String, AggregatedRegionSplitEntry>` — produces the canonical aggregation and pushes WARN diagnostics for tied priorities.
    - Cross-manifest tied-priority WARN: `LoadDiagnostic { level: DiagnosticLevel::Warning, path, field, message }` pushed into the caller-provided vec (NOT the runtime `ProgressEvent` channel). Mirrors `manifest.rs:493`, `validation.rs:342`.
    - `pub fn canonical_variant_chain_order(agg: &BTreeMap<String, AggregatedRegionSplitEntry>) -> Vec<String>` — convenience accessor returning semantic names in `(priority, name)` order.
  - **`crates/slicer-runtime/src/layer_executor.rs`** — the per-layer dispatch guard at line 362 (filter call site; block lines 357-364) inside `execute_single_layer_inner`:
    - Insertion at the top of the per-module loop body, BEFORE `instrumentation.on_module_start` (line 366), the WASM-handle lookup, and the existing `let run_result = runner.run_stage(&stage.stage_id, layer, &live_module, input);` call at line 394 — so skipped modules are absent from instrumentation and audit records:
      ```rust
      // Per-layer host filter (packet 92): skip this module on this layer
      // if it declares [[region_split]] semantics and no region's
      // variant_chain matches any of them. The `continue` is placed
      // BEFORE on_module_start so the skipped module is truly absent
      // from the instrumentation and audit log.
      if !module_invocation_allowed_on_layer(module.region_split_semantics(), arena.slice()) {
          continue;
      }
      // ... existing on_module_start / live_module construction / run_stage ...
      ```
    - `pub fn module_invocation_allowed_on_layer(declared: &HashSet<String>, slice: Option<&SliceIR>) -> bool` (at `layer_executor.rs:1326`): returns `true` if `declared.is_empty()` (paint-transparent default), OR if `slice` is `None` (conservative-allow when no SliceIR available — rare), OR if any region in `slice.regions` has a `variant_chain` entry whose semantic is in `declared`. Cost: O(|regions| × |chain entries|) — bounded and cheap; the `HashSet` lookup is O(1). See D-92-2 for the signature trade-off vs the iterator-typed version originally pseudocoded.
    - NO empty-polygon guard inserted here. Per the audit, that guard has no per-region host invocation site and is descoped (see `packet.spec.md` §Scope Boundaries and the §Locked Assumptions and Invariants empty-polygon entry for where the guard is owned).
  - **`crates/slicer-scheduler/tests/fixtures/region_split_manifests/`** (NEW directory):
    - `basic.toml` — one valid `[[region_split]]` entry for AC-1.
    - `duplicate_semantic.toml` — two entries with same `semantic` for AC-3.
    - `scalar_value_type.toml` — `value_type = "scalar"` for AC-4.
    - `community_below_floor.toml` — community semantic at priority 250 for AC-5.
    - `core_priority_mismatch.toml` — `material` at priority 100000 for AC-6.
    - `priority_type_mismatch.toml` — `priority = "abc"` for AC-N3 (`LoadErrorKind::TomlParse`).
    - `tied_priorities/manifest_a.toml`, `tied_priorities/manifest_b.toml` — two manifests with distinct semantics tied at priority 1500 for AC-7.
  - **`crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs`** (NEW):
    - Exercises `module_invocation_allowed_on_layer` directly with synthetic `HashSet<String>` (declared semantics) and `SliceIR` (regions with synthetic `variant_chain` entries) inputs. Four `#[test]` functions cover the (declared × layer) matrix and an edge case:
      - `(M_A, Layer_1)` — allowed (region_X carries `variant_chain = [("material", ToolIndex(2))]`)
      - `(M_A, Layer_2)` — filtered (no region's `variant_chain` matches `material`)
      - `(M_B, Layer_1)` — allowed (paint-transparent default)
      - `(M_B, Layer_2)` — allowed (paint-transparent default)
      - `slice = None` — conservative-allow edge case
    - The test fixture must construct non-empty `variant_chain` programmatically because no production code path populates it today.
    - The dispatch-loop wiring at `layer_executor.rs:362` (the `if !module_invocation_allowed_on_layer(...) { continue; }` guard) is verified by **code inspection**, not by driving `execute_single_layer_inner` through a mock `LayerStageRunner`. See D-92-6 in `packet.spec.md` §Deviations. A mock-runner integration test was explicitly deferred — the cost is not justified for the 4-line regression surface.
- Rejected alternatives that were considered and why they were not chosen:
  - **Per-region host dispatch refactor** (refactoring `runner.run_stage` to be per-region): rejected as out of scope. Materially larger than P92's M budget; touches every module's invocation contract and possibly WIT bindings. Candidate follow-up if P95 closure reveals a need.
  - **Single-semantic-per-manifest (top-level `region_split = { ... }` table instead of `[[region_split]]` array)**: a module declaring both `material` and `custom_X` would need two manifests. Rejected — arrays scale.
  - **Aggregate by `Vec<Tuple>` instead of `BTreeMap`**: O(N) lookup per dispatch decision instead of O(log N). Rejected — dispatch is hot.
  - **Per-manifest `priority_floor` override config key**: lets a module bypass the community floor. Rejected — the floor is a contract guard against priority squatting; making it tunable defeats the purpose. The path to a sub-1000 priority is to land the semantic in `CORE_REGION_SPLIT_PRIORITIES` via a packet (a deliberate, reviewed step).
  - **New `MissingField` / `TypeMismatch` `LoadErrorKind` variants**: rejected as enum bloat. Existing `Schema` (with `field` populated) covers missing-field semantics; existing `TomlParse` (with the toml-deserializer's structured message) covers malformed-type. Reuse, don't add.
  - **WARN emission via runtime `ProgressEvent` channel (`docs/09_progress_events.md`)**: rejected. Manifest-load diagnostics belong to the scheduler load-time channel (`LoadDiagnostic`), not the runtime slice-event channel.
  - **Universal empty-polygon dispatch guard at the layer-executor level**: rejected as architecturally impossible (no per-region host invocation site). Hand-off to P93/P95 noted.

## Files in Scope (read + edit)

- `crates/slicer-scheduler/src/manifest.rs` — role: parser + validators + `LoadedModule` + `LoadErrorKind`; expected change: new struct/enum, 4 new error variants, validation wiring.
- `crates/slicer-scheduler/src/validation.rs` — role: host of the post-deserialize validation pass that the new validators plug into; expected change: extend the existing pass with the four region-split checks.
- `crates/slicer-scheduler/src/region_split.rs` (NEW) — role: aggregation logic; expected change: new file.
- `crates/slicer-scheduler/src/lib.rs` — role: declare new `region_split` module; expected change: one-line `pub mod region_split;`.
- `crates/slicer-schema/src/lib.rs` — role: priority registry; expected change: two new consts.
- `crates/slicer-runtime/src/layer_executor.rs` (range 355-405 PLUS lines 1318-1344 for the helper) — role: per-layer dispatch guard at line 362 (filter call site; block lines 357-364) preceding `runner.run_stage(...)` at line 394; expected change: insert filter guard plus the `pub fn module_invocation_allowed_on_layer` helper at line 1326.
- `crates/slicer-scheduler/tests/fixtures/region_split_manifests/` (NEW) — role: validator test fixtures; expected change: 7 new tiny TOML files.
- `crates/slicer-scheduler/tests/region_split_manifest_tdd.rs` (NEW) — role: validator integration tests; expected change: new file.
- `crates/slicer-scheduler/tests/region_split_aggregation_tdd.rs` (NEW) — role: aggregation + WARN tests; expected change: new file.
- `crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs` (NEW) — role: per-layer filter integration test; expected change: new file.

Above the ≤ 3 guideline; this packet adds three system pieces (manifest schema, aggregation, per-layer dispatch filter) plus their tests. The per-step plan keeps each step to ≤ 3 file edits.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — §"P1b" only (range-read; carries the empty-polygon descope note added at refinement).
- `docs/03_wit_and_manifest.md` — §"Module Manifest TOML Schema" only (range-read; file may be > 300 lines).
- `docs/04_host_scheduler.md` — §"Module Dispatch" only (range-read).
- An existing core-module manifest e.g. `modules/core-modules/seam-planner-default/seam-planner-default.toml` — read as a template for TOML field placement (each manifest is ≤ 60 lines).
- `crates/slicer-runtime/src/layer_executor.rs` — read ONLY lines 355-405 via ranged Read (50 lines covering the per-module dispatch loop body, the filter call site at line 362, and the `runner.run_stage` call at line 394). Additionally read lines 1318-1344 for the filter helper definition.
- `crates/slicer-scheduler/src/manifest.rs` — range-read targeted at lines 25-75 (`LoadedModule` field list), 410-470 (`DiagnosticLevel` / `LoadDiagnostic` / `LoadError` / `LoadErrorKind`), and 525-560 (`ingest_manifest`). Do not read the whole file.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- Production module sources under `modules/core-modules/*/src/` — no module code changes in this packet.
- `crates/slicer-runtime/src/dispatch.rs` — distinct from `layer_executor.rs`; not in scope.
- `pnp-cli/**` — no CLI change.
- `docs/09_progress_events.md` — NOT in scope; the AC-7 WARN does not use the runtime `ProgressEvent` channel.
- `run_paint_annotation` (lines 626-722) and `assemble_ordered_entities` (lines 746+) in `layer_executor.rs` — those touch `variant_chain` placeholders but are not the dispatch site. Do not edit them in this packet.

## Expected Sub-Agent Dispatches

- "Run `rg -nE 'pub struct LoadedModule|pub enum LoadErrorKind|fn ingest_manifest' crates/slicer-scheduler/src/`; return LOCATIONS (≤ 10 entries)" — purpose: confirm the line numbers haven't drifted before edit.
- "Run `rg -nE 'DiagnosticLevel::Warning' crates/slicer-scheduler/src/`; return LOCATIONS" — purpose: mirror the existing WARN pattern in the new aggregator.
- "Run `cargo check -p slicer-schema`; return FACT pass/fail" — purpose: Step 2 gate.
- "Run `cargo check -p slicer-scheduler`; return FACT pass/fail" — purpose: Step 3 gate.
- "Run `cargo test -p slicer-scheduler region_split 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: AC-1, AC-3..AC-7, AC-8, AC-N2, AC-N3.
- "Run `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: AC-9.
- "Run the AC-10 baseline-compare shell command (see `packet.spec.md` AC-10); return FACT exit code" — purpose: AC-10 byte-identical.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-11.

## Data and Contract Notes

- IR or manifest contracts touched: manifest TOML grammar widens (new optional top-level array). `LoadedModule` shape changes (new field with default-empty Vec + derived HashSet cache). Existing manifests not declaring `[[region_split]]` deserialize identically.
- WIT boundary considerations: none directly. The module descriptor crossing into the WASM host is unchanged in WIT-visible *content* — only the host-side `LoadedModule` representation gains a field.
- Determinism or scheduler constraints: `BTreeMap` aggregation is the determinism choice. Lex tiebreaker on equal priority makes order reproducible across runs.
- Error-emission location (resolved during refinement): `LoadError` is constructed and returned from `manifest.rs:475+ (load_module_from_paths)` and `manifest.rs:483+ (load_modules_from_roots)`. `LoadDiagnostic`s are pushed into `&mut Vec<LoadDiagnostic>` parameters along the call chain (pattern at `manifest.rs:493`, `validation.rs:333,342`, `execution_plan.rs:223`). New region-split validators follow these patterns.

## Locked Assumptions and Invariants

- **Filter granularity is per-(module × layer)**: the host's only per-stage invocation site is `runner.run_stage(stage_id, layer, &live_module, input)` at `layer_executor.rs:394`. Per-region filtering remains module-internal and is not changed by this packet. True per-region host dispatch would require refactoring `runner.run_stage` and is explicitly out of scope.
- **The dispatch guard reads from `module.region_split_semantics()`** — the cached `HashSet<String>` on `CompiledModuleStatic` (propagated from `LoadedModule.region_split_semantics` at plan-build time; see D-92-5). The runtime does NOT carry a separate `ModuleMetadata` struct. The per-module descriptor in scope at the filter site (`layer_executor.rs:362`) is `&CompiledModuleStatic` via the per-module loop variable; `&CompiledModuleLive` is only constructed later at line 374 if the filter passes. No new struct is introduced for the filter.
- **`aggregated_region_split` is `BTreeMap`, not `HashMap`**: the BTreeMap iteration order IS the canonical variant-chain order. `HashMap` would silently corrupt this.
- **Core priorities are not user-overridable**: a manifest stating `material = 100` is fine; `material = 999` is rejected. The path to changing a core priority is a code change to `CORE_REGION_SPLIT_PRIORITIES`, which is a reviewable packet.
- **Scalar value type is forbidden in region-split**: D13. The error variant is explicit (`LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`); never silently convert to another value type.
- **WARN channel is `LoadDiagnostic`**: tied-priority warnings flow through `&mut Vec<LoadDiagnostic>` at module-load, NOT through the runtime `ProgressEvent` channel.
- **`MissingField` and `TypeMismatch` are NOT new variants**: missing-required-field uses existing `LoadErrorKind::Schema` with `field` populated; malformed-type uses existing `LoadErrorKind::TomlParse` (the toml-deserializer already surfaces field/expected/actual).
- **Empty-polygon guard is NOT in this packet**: the host has no per-region invocation site for it. The guard is owned by P95 (paint-segmentation port), which has the polygons in hand via `replace_slice_ir`. P93 keeps D15 unconditional emission (locked during the P93 refinement pass that resolved Audit 2). "IR construction" in the prior P92 audit response referred specifically to P95's paint-segmentation output (`SlicedRegion` polygons), NOT to P93's `RegionMapIR.entries` emission — the kernel cannot predict polygon emptiness from `ActiveRegion` alone.

## Risks and Tradeoffs

- **Risk: the new TOML grammar extension breaks an existing manifest's parse** if it accidentally collides with an existing top-level array of the same name. Mitigation: `[[region_split]]` is a deliberately namespaced name; grep existing core-module manifests confirms no collision.
- **Risk: the per-layer dispatch filter introduces measurable per-layer overhead** even when no module declares anything. Mitigation: the early-out `if module.region_split_semantics.is_empty() { return true }` short-circuits before any iteration; the only work for paint-transparent modules is one is_empty check.
- **Risk: a manifest validator surfaces a confusing error** for a power user authoring a community module. Mitigation: each `LoadError` carries `path`, `field`, and `message`; the new `LoadErrorKind` variant payloads supply the structured detail (priority, floor, expected, etc.).
- **Risk: AC-9 test fixture diverges from real production data flow**. The fixture constructs non-empty `variant_chain` synthetically because no production code populates it yet (P1c does). Mitigation: when P1c lands and starts populating `variant_chain`, a follow-up smoke test should exercise the per-layer filter against real production paths.
- **Tradeoff: cross-manifest WARN vs. ERROR for tied priorities.** Errors would force resolution at module-author time; warnings keep the system functional. We chose WARN (D6 + lex-tiebreaker) on the principle that tied priorities are usually accidental + auto-resolvable; loud-but-non-fatal surfacing is better than fatal blocking.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4 — the validator suite with 6 fixtures + tests).
- Highest-risk dispatch: the AC-10 byte-identical comparison vs. the Step 0 baseline. Step 0 MUST capture `P91_BASELINE_SHA=<hex>` into `closure-log.md` for AC-10's shell command to succeed.

## Open Questions

- (All resolved at refinement; see §Locked Assumptions and Invariants for the answers.)
- Former `[FWD]` "Where exactly does the existing manifest-load pipeline emit `ManifestParseError`s?" — RESOLVED: errors flow through `LoadError`/`LoadErrorKind` at `crates/slicer-scheduler/src/manifest.rs:437/450`; WARN diagnostics flow through `LoadDiagnostic { level: DiagnosticLevel::Warning, ... }` pushed into `&mut Vec<LoadDiagnostic>`.
- Former `[FWD]` "Does `slicer-runtime/src/layer_executor.rs` carry a `ModuleMetadata` struct?" — RESOLVED: no separate `ModuleMetadata` exists. The runtime accesses the per-module descriptor via the loop variable `&CompiledModuleStatic` (in scope at `layer_executor.rs:362`) carrying `region_split_semantics: HashSet<String>` propagated from `LoadedModule` at plan-build (`execution_plan.rs:741`). The filter hangs off this propagated field. See D-92-5.
- `[BLOCK]` — None.
