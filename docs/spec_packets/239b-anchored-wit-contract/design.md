# Design: 239b-anchored-wit-contract

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-schema/wit/deps/ir-types.wit` — the five orphaned anchored records
    (`anchored-entity`, `anchored-geometry-contract`, `anchored-entity-provenance`,
    `anchored-event-runtime-hooks`, `ordered-event-collection`) and the existing
    `resource layer-collection-builder` that gains the one method de-orphaning them.
  - `crates/slicer-schema/src/lib.rs` — `STAGES: &[StageSpec]` (the ninth `world-layer` row) and
    `VALID_STAGES`. The accessors `export_for_stage_id`, `wit_dir_for_stage_id`,
    `package_for_stage_id`, `interface_for_stage_id`, `wit_world_for_stage_id`, and
    `qualified_export_for_stage_id` become total over the new stage for free once the row exists.
  - `crates/slicer-wasm-host/src/host.rs` — **two** distinct surfaces:
    (a) the **bindgen world module**. Every layer world has a `pub mod <world>` wrapping a
    `wasmtime::component::bindgen!` invocation, plus a sibling alias in the file's `pub use`
    block. The exemplars are `pub mod layer_support` / `pub mod layer_support_postprocess` and
    `pub use layer_support::LayerModule as LayerSupportModule`. This packet adds
    `pub mod layer_anchored_events` (world `anchored-events-module`) and
    `pub use layer_anchored_events::LayerModule as LayerAnchoredEventsModule`. **Without this the
    guest can never be instantiated** — there is no generated `LayerModule` type to link or call.
    (b) `HostExecutionContext`'s output-collector block, its `push_layer_collection_builder`
    constructor, and `impl ir::HostLayerCollectionBuilder for HostExecutionContext` (where
    `set_entity_order` and `get_ordered_entities` are implemented today; the new resource method
    joins them).
  - `crates/slicer-wasm-host/src/dispatch.rs` — **two distinct `match stage_id.as_str()` sites,
    both in scope, and they must not be confused with each other:**
    1. **The layer-tier linker/instantiate/call site** — the `match stage_id.as_str()` whose value
       is bound as `let (call_result, mut store, mem_initial_bytes) = ...`. Each of its arms
       (e.g. `"Layer::Infill"`, `"Layer::Support"`) calls `add_wasi_to_linker`, then
       `<world>::LayerModule::add_to_linker`, builds the store via `HostExecutionContextBuilder`,
       and instantiates + calls through `<world>::LayerModule::instantiate`. **This is the site
       that actually runs the guest**, and it needs a `"Layer::AnchoredEvents"` arm. It is the
       consumer of the `host.rs` bindgen module above.
    2. **`deconstruct_layer_ctx`** — signature `Result<Option<LayerStageCommit>, LayerStageError>`,
       a *separate* `match stage_id` that runs **after** the guest has returned and turns the
       accumulated host state into a commit. Its `"Layer::Support" | "Layer::SupportPostProcess"`
       arm is the `Ok(None)`-on-empty exemplar. It gains the producer arm. It is called from two
       sites in that file.
    A third `match stage_id.as_str()` in the same file, binding `let (call_result, mut store) = ...`,
    serves the **prepass** tier and is **explicitly out of scope** — do not add an arm to it.
  - `crates/slicer-wasm-host/src/marshal/native.rs` — the **native twin**: the
    `match stage_export` inside `commit_native_layer_response` (it scrutinises `stage_export`, a
    `&str`, **not** `stage_id`) that must gain the same arm in the same step (the repo's both-legs
    guard). Its existing support arm is `"Layer::Support" | "Layer::SupportPostProcess" => { ... }`
    — one arm serving both stages, which matters for Step 5c (see §Code Change Surface item 7).
  - `crates/slicer-sdk/src/layer_collection_builder.rs` — `set_anchored_event_collection`,
    `anchored_proposal`, `get_anchored_event_collection`, `set_anchored_event_snapshot`. These
    exist and are reachable from guest code today; only the drain is missing.
  - `crates/slicer-macros/src/lib.rs` — `slicer_module`'s per-stage `emit_world_preamble` call
    sites (15 today, 16 after this packet) and `emit_world_preamble`'s `include_str!` of the
    canonical `.wit` files. `strip_package_decl` already rewrites statement-form headers into
    nested `package slicer:X { ... }` blocks for `wit-bindgen 0.57.1`; the new stage file needs
    no special handling. **There is no inline WIT copy to keep in sync** — the macro reads the
    canonical files directly.
- Neighboring tests/fixtures:
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/` — the exemplar
    to copy: 46-line `src/lib.rs`, one `#[slicer_module]` impl, config-parameterized so one
    binary serves both a positive and a negative fixture. Its `Cargo.toml` shows the minimal
    dependency set (`wit-bindgen`, `slicer-sdk`, `slicer-ir`, `slicer-schema`,
    `crate-type = ["cdylib"]`, `[workspace]` sentinel).
  - `crates/slicer-runtime/tests/executor/finalization_mutation_roundtrip_tdd.rs` — the exemplar
    host-side harness: reads the sibling `.component.wasm`, compiles it, and drives a dispatch.
    The new round-trip test lives beside it in the `executor` binary.
  - `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs` — the ADR-0020 drift
    gate this packet must update.
  - `crates/slicer-wasm-host/tests/contract/main.rs` — the both-legs contract home (AC-6).
  - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — home for AC-4.
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — forces an explicit
    user-targetable-vs-host-only classification for every new `STAGE_ORDER` entry.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat
  delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This is not incidental here: this packet edits `crates/slicer-schema/wit/`, so **every** guest — all core-modules and all 23 test-guests — is stale from Step 2 onward and typed instantiation fails for all of them until rebuilt. The `--check` gate is this packet's central failure-attribution tool, not a formality.)

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. (Concretely here: `anchored-geometry-contract` is `planar(s64)` / `z-spanning(tuple<s64, s64>)` — canonical 100 nm units carried as integers. The lift and lower glue MUST move these as `i64` with no scaling, no `f32` hop, and no mm conversion. `z: 3000` means 0.3 mm and must arrive as exactly `3000`; any float round-trip is a correctness bug AC-1 is written to catch.)

- **No schema/version constant is bumped.** Verified across `AnchoredGeometryContract`,
  `AnchoredEntityProvenance`, `AnchoredEntity`, `AnchoredEventRuntimeHooks`, and
  `OrderedEventCollection` in `crates/slicer-ir/src/slice_ir.rs`: none carries a `schema_version`
  field and no `ANCHORED_*_SCHEMA_VERSION` constant exists, so zero tests hard-assert an anchored
  version and there is no version-bump fallout to author.
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is documented in `docs/02_ir_schemas.md` and must
  not be disturbed — this packet adds transport, not IR shape. The mandatory
  version-locking rule therefore applies vacuously; do not invent a bump to satisfy it.
- **Config keys are snake_case.** The test guest's fixture parameters are `anchored_event_count`,
  `emit_malformed_geometry`, `duplicate_proposal` — underscores, never hyphens, in every Rust
  `config.get_*` call (`CLAUDE.md` §Config Key Naming Convention).
- **ADR-0059 conformance (`docs/adr/0059-support-families-and-anchored-entities.md`, accepted).**
  This packet is the transport half of that decision and conforms to it, rather than amending or
  contradicting it. Three clauses bind directly:
  1. *"each worker returns ordered event collections"* — the WIT method transports a whole
     `ordered-event-collection`, not a flat `list<anchored-entity>`, and the commit carries
     `Vec<OrderedEventCollection>`. The shape is the ADR's, not a convenience choice.
  2. *"a planar entity ... is anchored to the upper global layer"* — carried verbatim by the
     `anchor-global-layer-index: u32` field of `ordered-event-collection`; AC-1 asserts the value
     `7` survives the boundary.
  3. *"path validation follows each entity's declared planar or Z-spanning contract instead of
     the model-layer Z envelope"* — the producer arm validates **per declared contract**, which
     is why this packet carries **two** geometry negatives: AC-N1 (planar) and AC-N4 (Z-spanning).
     A producer arm that checks only the planar case is non-conformant even though it would pass
     AC-N1.
  Nothing here amends the ADR: the packet adds no execution-ordering, no cooling-accounting, and
  no raft semantics. Those clauses are exercised by `239a-anchored-host-seams` and
  `239c-support-layer-height-producer`.
- **The geometry validator is DUPLICATED into `slicer-wasm-host`, not reused — a crate-graph
  fact, not a style choice.** The canonical checks live in `validate_anchored_entity`
  (`crates/slicer-runtime/src/layer_executor.rs`), which is out of bounds here **and**
  unreachable: the dependency edge runs `slicer-runtime` → `slicer-wasm-host`
  (`crates/slicer-runtime/Cargo.toml` declares `slicer-wasm-host`; the reverse dependency does
  not exist), so a producer arm in `slicer-wasm-host` cannot call into `slicer-runtime`. This
  packet therefore authors `validate_anchored_entity_geometry` in
  `crates/slicer-wasm-host/src/marshal/out.rs` carrying **duplicated string literals**
  (`anchored entity planar z mismatch`, `anchored entity z-span violation`) and duplicated
  tolerance logic against `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`. Promotion of
  the validator to a crate both can see (e.g. `slicer-ir`) was considered and **rejected for this
  packet**: it would move a symbol out of an out-of-bounds file and change apply-side behaviour,
  which is `239a-anchored-host-seams` territory. Record the duplication honestly — do not write
  closure language implying the two sites share an implementation, and if the wording in
  `layer_executor.rs` ever changes, both copies drift and AC-N1/AC-N4 are the tripwire.
- **WIT/Type Changes Checklist binds every step**: after any `.wit` edit, search all
  `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type; verify type
  identity across the component boundary (a mismatch such as `list<ordered-event-collection>` on
  one side and a single record on the other surfaces as a linking failure, not a type error);
  run `cargo build --tests`; and edit **only** the canonical sources under
  `crates/slicer-schema/wit/`.

## Code Change Surface

- **Selected approach.**
  1. **De-orphan by reference, not by duplication.** Add one method to the *existing*
     `resource layer-collection-builder` in `ir-types.wit`:
     `set-anchored-event-collection: func(collection: ordered-event-collection) -> result<_, string>`.
     `ordered-event-collection` transitively references the other four records, so a single
     method reference pulls all five into every world that imports `slicer:ir-handles/ir-handles`.
  2. **Declare a per-stage package** at
     `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit`, shaped
     exactly like `deps/layer-support/layer-support.wit`:
     package `slicer:layer-anchored-events@1.0.0`; interface `anchored-events` with
     `run: func(layer-index: layer-idx, regions: list<slice-region-view>,
     collection: layer-collection-builder, config: config-view) -> result<_, module-error>;`;
     world `anchored-events-module` importing `slicer:common/host-services`,
     `slicer:common/profiling`, `slicer:config/config-types`, `slicer:ir-handles/ir-handles` and
     exporting `anchored-events`. Per the established pattern, the output builder is a **resource
     declared in `ir-types.wit` and passed as a `run` parameter**, never declared in the stage
     file.
  3. **Register the stage** in `STAGES`, `VALID_STAGES`, and `STAGE_ORDER`. The `stage_id`
     `"Layer::AnchoredEvents"` is not invented — it is the exact string
     `LayerStageCommit::stage_id` already returns for the `AnchoredEvents` variant, so
     registration makes the enum and the table agree rather than introducing a new vocabulary
     term.
  4. **Lift on the host** following the `support-output-builder` chain verbatim:
     `AnchoredEventsCollected` accumulator → `HostExecutionContext.anchored_events_output` field
     → the resource-method impl that writes into it → `convert_anchored_events` in
     `marshal/out.rs` → re-export from `marshal/mod.rs`.
  5. **Make the guest callable, then produce on both legs.** Two sub-stages, because they are
     different kinds of edit: **(5a)** the `pub mod layer_anchored_events` `bindgen!` block and
     its `pub use` alias in `host.rs`, plus the `"Layer::AnchoredEvents"` arm at the layer-tier
     linker/instantiate/call `match stage_id.as_str()` in `dispatch.rs` — this is what actually
     runs the guest; **(5b)** the `"Layer::AnchoredEvents"` producer arm in `deconstruct_layer_ctx`
     and its twin in `marshal/native.rs`, which land together under the both-legs guard.
  6. **Drain in the SDK**: the generated shim reads `anchored_proposal()` at the end of a
     `run_anchored_events` dispatch and calls the new WIT method; the native mirror and the test
     capture sink gain the same path.
  7. **Widen the `layer-support` world to two builders** (the approved resolution of the former
     `[BLOCK]`; see §Open Questions). `run` in
     `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` gains
     `collection: layer-collection-builder` after `output: support-output-builder`, mirroring
     `layer-path-optimization.wit`'s two-builder `run` verbatim. This is a **breaking** change to
     an existing world, so every co-moving surface lands with it in one step: the SDK trait
     method, the macro glue on both legs, the native mirror, the host invocation arm, and both
     production support modules. Enumerated exhaustively in the list below and budgeted as
     Step 5c, with the reachability proof as Step 5d.
- **Exact functions, traits, manifests, tests, and fixtures.**
  - WIT: `resource layer-collection-builder` (new method), new file
    `layer-anchored-events.wit` (package `slicer:layer-anchored-events@1.0.0`, interface
    `anchored-events`, world `anchored-events-module`).
  - `slicer_schema::STAGES` (one row), `slicer_schema::VALID_STAGES` (one entry).
  - `slicer_scheduler::execution_plan::STAGE_ORDER` (one entry among the `Layer::*` block).
  - `slicer_macros::slicer_module` — one new `emit_world_preamble("anchored-events-module",
    "anchored_events", ...)` call site with its own `include_str!`; `crates/slicer-macros/build.rs`
    — one new `rerun-if-changed` path.
  - `xtask/src/wit_verify.rs` `#[cfg(test)]` module — the three `20` expectations become `21`
    (macro `include_str!` count, the canonical-set comparison, and the `build.rs` watch-set
    count). **This 20→21 is owed entirely to the NEW `layer-anchored-events.wit` file.** The
    `layer-support.wit` widening (item 7) is a signature-only edit to a file already in all three
    sets, so it changes **no** count; the two edits must never be conflated. Verified against
    this tree: every `20` in `xtask/src/wit_verify.rs` counts distinct `.wit` **file paths** —
    the macro's `include_str!` path set, the canonical-set comparison, and `build.rs`'s
    `rerun-if-changed` watch set — and none of them inspects a `run` signature.
  - **The `layer-support` two-builder widening (item 7) — complete blast radius**, every entry
    verified by direct read against this tree:
    - `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` — the `use` list gains
      `layer-collection-builder` and `run` gains the `collection` parameter.
    - `crates/slicer-sdk/src/traits.rs` — `LayerModule::run_support`'s default-body signature
      gains `_collection: &mut LayerCollectionBuilder`, placed after `_output`, matching
      `LayerModule::run_path_optimization`'s existing parameter order.
      `LayerModule::run_support_postprocess` is **not** touched: `Layer::SupportPostProcess` uses
      the separate `layer-support-postprocess` world and is out of scope.
    - `crates/slicer-macros/src/lib.rs` — three call sites: (i) `build_layer_support_glue`'s
      `Guest::run` signature and the `run_support` call it forwards to, which must construct /
      forward the `LayerCollectionBuilder` the same way `build_layer_path_optimization_glue` does.
      **The `set-anchored-event-collection` drain call is Step 6's, not Step 5c's**: Step 5c wires
      the parameter through only, and the drain lands on both legs together in Step 6, which is why
      AC-8 stays red until Step 6. (ii) the native `"run_support"` arm of the
      stage-method `match`, which builds the SDK builders and fills `NativeLayerResponse`;
      (iii) the `NativeLayerResponse` construction in that arm.
      `emit_world_preamble("support-module", "support", …)` itself needs no change — it re-reads
      the same `include_str!`ed WIT.
    - `crates/slicer-sdk/src/native.rs` — **yes, the non-wasm mirror needs the parameter.**
      `NativeLayerResponse.support` is `Option<SupportOutputBuilder>` today and carries no
      collection; it must convey the proposal, following the `NativePathOptimizationOutput
      { output, collection }` precedent in the same file (either a sibling
      `NativeSupportOutput { output, collection }` or an added field). Note the churn gate:
      `NativeLayerResponse` has five named fields, so any test-side literal of it needs `..` or
      an `// exhaustive:` waiver (`docs/21_data_defaults_and_fixtures.md`); production literals
      stay exhaustive.
    - `crates/slicer-wasm-host/src/dispatch.rs` — the `"Layer::Support"` arm of the **layer-tier**
      `match stage_id.as_str()` (the one bound as
      `let (call_result, mut store, mem_initial_bytes) = ...`). It gains a
      `push_layer_collection_builder` call and one more `own(collection)` argument to
      `call_run`, copying what the `"Layer::PathOptimization"` arm already does. This is **not** a
      new arm — it is two added lines in an existing one.
    - `crates/slicer-wasm-host/src/dispatch.rs` — `deconstruct_layer_ctx`'s
      `"Layer::Support" | "Layer::SupportPostProcess"` arm, only if a support-stage anchored
      proposal must surface as a commit. **Scope note:** `deconstruct_layer_ctx` returns **one**
      commit per dispatch, so a `Layer::Support` dispatch that also carries an anchored proposal
      cannot emit both `Support` and `AnchoredEvents` from that one return. AC-8 therefore asserts
      the proposal **arrives host-side** in the accumulator and converts to
      `LayerStageCommit::AnchoredEvents` with its `s64` Z intact; how a *production* pipeline
      routes both commits from one support dispatch is `239c`'s to specify, not this packet's.
    - `crates/slicer-wasm-host/src/marshal/native.rs` — the native leg's
      `NativeLayerResponse` consumer, `commit_native_layer_response`, must read the new collection
      field, or the two legs diverge and AC-6's both-legs invariant is violated for support.
      **Shared-arm note (load-bearing):** the relevant arm of its `match stage_export` is
      `"Layer::Support" | "Layer::SupportPostProcess" => { ... }` — a single arm serving **both**
      stages. A collection carried only by `Layer::Support` therefore requires **splitting that
      arm** so only the Support half reads the collection. That split is **in scope for Step 5c**
      and is budgeted there. Splitting the match arm does **not** constitute touching
      `LayerModule::run_support_postprocess`, which remains untouched per the bullet above: the
      `SupportPostProcess` half keeps its present behaviour verbatim.
    - `modules/core-modules/tree-support/src/lib.rs` and
      `modules/core-modules/traditional-support/src/lib.rs` — the two production `Layer::Support`
      guests (confirmed by `id = "Layer::Support"` in `tree-support.toml` and
      `traditional-support.toml`). Both override `run_support`, so both signatures must gain the
      parameter. Neither needs to *use* it in this packet — 239c is the producer.
      `modules/core-modules/support-surface-ironing` binds `Layer::SupportPostProcess` and is
      **not** affected.
    - **Permitted follow-on tests, re-derived against this tree** by grepping `fn run_support`
      definitions and `.run_support(` call sites (excluding `run_support_postprocess` and
      `run_support_geometry`). A bare `SupportOutputBuilder::new()` does not break; a
      `.run_support(...)` call or a `fn run_support` impl does. Edit only if the arity change
      breaks them:
      `modules/core-modules/tree-support/tests/slicer_module_binding_tdd.rs`,
      `modules/core-modules/traditional-support/tests/slicer_module_binding_tdd.rs`,
      `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs` (a **local
      helper** `fn run_support(points, angle, line_width)` that builds `SupportOutputBuilder::new()`
      and calls `TraditionalSupport::run_support` directly),
      `modules/core-modules/traditional-support/tests/{traditional_support_tdd,traditional_family_tdd,enforcer_blocker_tdd}.rs`,
      `modules/core-modules/tree-support/tests/{tree_support_tdd,tree_family_tdd,enforcer_blocker_tdd}.rs`,
      `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`,
      `crates/slicer-runtime/tests/integration/{traditional_support_family,tree_support_family}.rs`,
      `crates/slicer-sdk/tests/layer_module_tdd.rs`, and
      `crates/slicer-macros/tests/{slicer_module_tdd,binding_surface_tdd}.rs`.
      **Not broken, do not edit:**
      `modules/core-modules/support-surface-ironing/tests/{ironing_tdd,ironing_scanline_parity_tdd}.rs`
      and `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — they build a
      `SupportOutputBuilder` but never call `run_support`.
    - `crates/pnp-cli/src/module_new.rs` — **advisory, not a compile break.** Its `"Layer::Support"`
      scaffold template is a string literal spelling the old five-parameter `run_support`; left
      stale, `module new` emits non-compiling modules, against the `#[slicer_module]`/scaffold
      lock-step that `crates/slicer-macros/src/lib.rs`'s header comment asserts. Permitted to
      update in Step 5c; a deliberate deferral must be recorded, not implicit.
    - **Test guests:** there is no `Layer::Support` test guest in
      `crates/slicer-wasm-host/test-guests/` today — verified by directory scan;
      `dispatch-layer-support-postprocess-guest` binds the *postprocess* world and
      `sdk-support-diagnostic-guest` implements `run_support_geometry` (a prepass stage). AC-8's
      new `support-anchored-reach-guest` is therefore the first, and is authored in Step 5d.
      **Binding mechanism (verified):** it implements `LayerModule::run_support` under
      `#[slicer_module]`, which detects the stage from the method name. There is **no** manifest
      TOML — none exists under `crates/slicer-wasm-host/test-guests/`, and
      `xtask/src/build_guests.rs` sets `stage_id: None` unconditionally for `GuestTree::TestGuest`;
      `parse_stage_id_from_module_manifest` serves `modules/core-modules/<name>/<name>.toml` only.
      The **test** supplies `"Layer::Support"`, as `LayerStageRunner::run_stage`'s `stage_id`
      argument (`crates/slicer-wasm-host/src/traits.rs`) and as `LoadedModuleBuilder::new`'s
      `stage` argument (`crates/slicer-scheduler/src/manifest.rs`).
    - **Guest artifacts:** every `.wasm` under `modules/core-modules/` and
      `crates/slicer-wasm-host/test-guests/` must be rebuilt; `cargo xtask build-guests --check`
      must exit `0` before any typed-instantiation failure is attributed elsewhere.
  - `slicer_sdk::traits::LayerModule::run_anchored_events` (default no-op body).
  - `slicer_sdk::builders`-adjacent: `LayerCollectionBuilder::{set_anchored_event_collection,
    anchored_proposal}` (existing; drained), `crates/slicer-sdk/src/native.rs` mirror,
    `crates/slicer-sdk/src/test_support/capture.rs` sink.
  - `slicer_wasm_host::marshal::accumulators::AnchoredEventsCollected` (new),
    `slicer_wasm_host::marshal::out::convert_anchored_events` (new),
    `impl ir::HostLayerCollectionBuilder for HostExecutionContext` (one method),
    `HostExecutionContext` (one field + `_mut` accessor),
    `slicer_wasm_host::host::layer_anchored_events` (new `bindgen!` module for world
    `anchored-events-module`) with the `LayerAnchoredEventsModule` re-export alias.
  - `slicer_wasm_host::marshal::out::validate_anchored_entity_geometry` (new; the
    `slicer-wasm-host`-local **duplicate** of the planar and z-span checks — see §Architecture
    Constraints for why reuse is impossible).
  - `dispatch.rs`: the layer-tier linker/instantiate/call `match stage_id.as_str()` arm **and**
    the `deconstruct_layer_ctx` arm; plus the native `match stage_export` inside
    `commit_native_layer_response` (`crates/slicer-wasm-host/src/marshal/native.rs`).
  - Tests, each with its authoring file **and** its registration site:
    - `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs` (AC-1/2/3,
      AC-N1/N2/N3/N4) — **must also be registered as `mod anchored_events_roundtrip_tdd;` in
      `crates/slicer-runtime/tests/executor/main.rs`**, which is how that binary compiles the
      file. An unregistered file never compiles and the AC reports zero tests.
    - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` (AC-4) — standalone target, no
      registration site; the new `anchored_events_stage_is_fully_declared` test is appended
      beside the existing `export_for_stage_id_is_total_over_stages_and_rejects_unknown`.
    - `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs` (AC-5) — already
      registered in `crates/slicer-runtime/tests/contract/main.rs`; no new `mod` line.
    - `crates/slicer-wasm-host/tests/contract/anchored_events_both_legs_tdd.rs` (AC-6) — **must
      also be registered as `mod anchored_events_both_legs_tdd;` in
      `crates/slicer-wasm-host/tests/contract/main.rs`**.
  - Guest: `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/`
    (`Cargo.toml` + `src/lib.rs` + a `[workspace]` sentinel). Test-guests are **discovered by
    directory scan** from `tg_root` in `xtask/src/build_guests.rs`; there is no hardcoded guest
    list to append to.
- **Rejected alternatives and reasons.**
  - *A dedicated `anchored-events-builder` resource in `ir-types.wit`.* Rejected: it would
    duplicate `layer-collection-builder`'s accumulator plumbing (a second
    `Host*Builder` impl, a second `push_*` constructor, a second resource-table entry) for no
    contract gain, and it would strand the existing SDK helpers on
    `LayerCollectionBuilder`, which is exactly the type they already live on.
  - *No new world; drain the proposal from the existing `Layer::PathOptimization` dispatch.*
    Rejected: `deconstruct_layer_ctx` returns `Result<Option<LayerStageCommit>, _>` — **one**
    commit per dispatch. A path-optimization dispatch already commits
    `LayerStageCommit::PathOptimization`, so it cannot also emit `AnchoredEvents`. The commit
    enum's one-variant-per-stage shape forces a dedicated stage.
  - *Reuse the `Layer::Support` world and **infer** anchored intent from the support output.*
    Rejected: it makes anchored transmission an implicit side effect, defeats `Ok(None)`-on-empty
    semantics (AC-N2), and would put support-payload semantics into a transport packet — that is
    239c's territory. **Not contradicted by item 7:** item 7 hands the support world an
    *explicit* `layer-collection-builder`, so a support guest that emits anchored work says so
    by calling `set-anchored-event-collection`; nothing is inferred from support geometry, and a
    support guest that never calls it produces no anchored proposal.
  - *Move the drain onto `support-output-builder`.* Rejected — see §Open Questions; it would
    confine anchored events to support stages and narrow the generic substrate packets 219-223
    built.
  - *Ship a dedicated anchored-events core module for support-derived anchored work.* Rejected —
    see §Open Questions; the one-stage-per-module manifest rule turns it into authoring and
    wiring a whole sibling module.
  - *Serialize `OrderedEventCollection` as a JSON string through an existing `string` channel.*
    Rejected: it discards WIT type identity, silently permits schema drift, and the `s64`
    coordinate would round-trip through text — exactly the failure AC-1 exists to prevent.
  - *Add the `run_anchored_events` method to a new SDK trait.* Rejected: `slicer_module` selects
    the world from the detected **stage method** (it scans the impl block against `STAGES`),
    and `tier_for_trait`/the cross-world guardrail already accept `LayerModule` for
    `TIER_LAYER` stages. A new trait would add a tier vocabulary entry for no benefit.

## Files in Scope (read + edit)

Primary three:

- `crates/slicer-schema/wit/deps/ir-types.wit` — role: the orphaned records and the builder
  resource live here; expected change: one method on `resource layer-collection-builder`.
- `crates/slicer-schema/src/lib.rs` — role: canonical stage table; expected change: one
  `StageSpec` row plus one `VALID_STAGES` entry.
- `crates/slicer-wasm-host/src/dispatch.rs` — role: the wasm leg, both invocation and production;
  expected change: **two** `"Layer::AnchoredEvents"` arms, in two different `match` statements.
  (i) One arm in the **layer-tier linker/instantiate/call site** — the
  `match stage_id.as_str()` bound as `let (call_result, mut store, mem_initial_bytes) = ...` —
  performing `add_wasi_to_linker`, `layer_anchored_events::LayerModule::add_to_linker`,
  `HostExecutionContextBuilder` store construction, and
  `layer_anchored_events::LayerModule::instantiate` + call. This is **not** a one-to-three-line
  mechanical edit; it is a full copy of the `"Layer::Support"` arm and is budgeted as its own step
  (Step 5a). (ii) One arm in **`deconstruct_layer_ctx`**, converting the accumulated proposal into
  `LayerStageCommit::AnchoredEvents` (Step 5b). The prepass-tier `match stage_id.as_str()` bound
  as `let (call_result, mut store) = ...` is **out of scope** and gains no arm.
- `crates/slicer-wasm-host/src/host.rs` — role: also the **bindgen world module home**; expected
  change: one `pub mod layer_anchored_events` `bindgen!` block (copied from `pub mod layer_support`)
  plus one `pub use layer_anchored_events::LayerModule as LayerAnchoredEventsModule;` alias in the
  file's `pub use` block, in addition to the accumulator field and resource-method impl. Without
  the bindgen module there is no `LayerModule` type for the invocation arm to link against.

Justified extras — each is mechanical (one entry, one line, or one copied pattern), and each is
budgeted into a named step rather than discovered by a later `cargo check`:

- `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit` (new file, ~20
  lines, copied from the `layer-support` shape) — Step 2.
- `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — AC-4's authoring home (one new test
  appended to a standalone target) — Step 2.
- `crates/slicer-runtime/tests/executor/main.rs` — one `mod anchored_events_roundtrip_tdd;` line;
  without it the new test file never compiles — Step 3.
- `crates/slicer-wasm-host/tests/contract/anchored_events_both_legs_tdd.rs` (new) +
  `crates/slicer-wasm-host/tests/contract/main.rs` (one `mod` line) — Step 5b.
- `crates/slicer-scheduler/src/execution_plan.rs` — one `STAGE_ORDER` entry — Step 2.
- `crates/slicer-macros/src/lib.rs` + `crates/slicer-macros/build.rs` + `xtask/src/wit_verify.rs`
  — the three cross-checked declaration-model surfaces; they must move together — Step 2.
- `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs` — ADR-0020 gate 8→9 —
  Step 2.
- `crates/slicer-ir/src/stage_io.rs` — **comment-only**: `LayerStageCommit::stage_id`'s doc
  comment says "The non-`None` set is exactly the eight `world-layer` stages"; it becomes nine,
  matching the ADR-0020 gate this packet moves — Step 2.
- `crates/slicer-sdk/src/traits.rs` — one default trait method — Step 3.
- `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/**` (new crate) +
  `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs` (new test) — Step 3.
- `crates/slicer-wasm-host/src/marshal/accumulators.rs`,
  `crates/slicer-wasm-host/src/marshal/out.rs`, `crates/slicer-wasm-host/src/marshal/mod.rs`,
  `crates/slicer-wasm-host/src/host.rs` — Step 4 (accumulator field, `_mut` accessor, resource
  method, converter, **and** the `pub mod layer_anchored_events` `bindgen!` block plus its
  `pub use` alias).
- `crates/slicer-wasm-host/src/dispatch.rs` — the layer-tier linker/instantiate/call arm — Step 5a
  (**not** mechanical; a full copy of the `"Layer::Support"` arm).
- `crates/slicer-wasm-host/src/dispatch.rs` (producer arm in `deconstruct_layer_ctx`) +
  `crates/slicer-wasm-host/src/marshal/native.rs` — Step 5b (both-legs twin, one commit).
- `crates/slicer-sdk/src/layer_collection_builder.rs`, `crates/slicer-sdk/src/native.rs`,
  `crates/slicer-sdk/src/test_support/capture.rs` — Step 6.
- **The `layer-support` two-builder widening — Step 5c** (breaking WIT change; all co-moving
  surfaces land in one commit, full justification in §Code Change Surface item 7):
  `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` (the `use` list and the `run`
  signature); `crates/slicer-sdk/src/traits.rs` (`LayerModule::run_support` only);
  `crates/slicer-macros/src/lib.rs` (`build_layer_support_glue` plus the native `"run_support"`
  arm); `crates/slicer-sdk/src/native.rs` (the `NativeLayerResponse` support carrier);
  `crates/slicer-wasm-host/src/dispatch.rs` (two added lines inside the existing
  `"Layer::Support"` layer-tier arm — `push_layer_collection_builder` and one more `own(...)`
  argument to `call_run`); `crates/slicer-wasm-host/src/marshal/native.rs`
  (`commit_native_layer_response`, the `NativeLayerResponse` consumer — **including splitting its
  shared `"Layer::Support" | "Layer::SupportPostProcess"` arm**); `modules/core-modules/tree-support/src/lib.rs`
  and `modules/core-modules/traditional-support/src/lib.rs` (both `run_support` overrides);
  permitted follow-on, only if the `run_support` arity change breaks them, the full re-derived
  list in §Code Change Surface item 7 — both `tests/slicer_module_binding_tdd.rs` files,
  `modules/core-modules/traditional-support/tests/{support_fill_geometry_tdd,traditional_support_tdd,traditional_family_tdd,enforcer_blocker_tdd}.rs`,
  `modules/core-modules/tree-support/tests/{tree_support_tdd,tree_family_tdd,enforcer_blocker_tdd}.rs`,
  `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`,
  `crates/slicer-runtime/tests/integration/{traditional_support_family,tree_support_family}.rs`,
  `crates/slicer-sdk/tests/layer_module_tdd.rs`, and
  `crates/slicer-macros/tests/{slicer_module_tdd,binding_surface_tdd}.rs`; plus the advisory,
  non-compile-breaking `crates/pnp-cli/src/module_new.rs` scaffold-template string.
- **The support-stage reachability proof — Step 5d**:
  `crates/slicer-wasm-host/test-guests/support-anchored-reach-guest/**` (new crate; `Cargo.toml`
  + `src/lib.rs` + a `[workspace]` sentinel — discovered by the `tg_root` directory scan in
  `xtask/src/build_guests.rs`, so no guest list is appended);
  `crates/slicer-runtime/tests/executor/support_anchored_reach_tdd.rs` (new; AC-8's home) +
  `crates/slicer-runtime/tests/executor/main.rs` (one `mod support_anchored_reach_tdd;` line —
  without it the file never compiles and AC-8 reports a green run over zero tests).
- `docs/02_ir_schemas.md`, `docs/07_implementation_status.md`,
  `docs/specs/support-parity-gap-register.md`,
  `docs/specs/support-independent-layer-z-split-plan.md` — Step 7.

## Read-Only Context

- `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` — whole file (~20 lines) —
  purpose: the exemplar stage-package shape to copy verbatim. **Also edited in Step 5c** (the
  `run` widening); read-only for Steps 1-5b and 6-7.
- `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` — whole
  file (~20 lines) — purpose: the two-builder `run` precedent
  (`output: gcode-output-builder, collection: layer-collection-builder`) that Step 5c copies,
  including `use`-list placement and parameter order.
- `crates/slicer-schema/wit/deps/ir-types.wit` — the `anchored-*` / `ordered-event-collection`
  record block and the `resource layer-collection-builder` / `resource support-output-builder`
  blocks only — purpose: exact field names and the resource-method convention.
- `crates/slicer-schema/src/lib.rs` — over 300 lines — the `StageSpec` definition, the first two
  `STAGES` rows, and the `VALID_STAGES` constant only — purpose: row shape.
- `crates/slicer-ir/src/stage_io.rs` — over 300 lines — the `LayerStageCommit` enum and its
  `stage_id()` impl only — purpose: confirm `AnchoredEvents` and `"Layer::AnchoredEvents"` exist
  and need no change.
- `crates/slicer-wasm-host/src/dispatch.rs` — over 300 lines — `deconstruct_layer_ctx`'s header
  and the `"Layer::Support" | "Layer::SupportPostProcess"` arm only — purpose: the
  `Ok(None)`-on-empty pattern AC-N2 asserts.
- `crates/slicer-wasm-host/src/host.rs` — over 300 lines — the `HostExecutionContext`
  output-collector block, `push_layer_collection_builder`, and
  `impl ir::HostLayerCollectionBuilder for HostExecutionContext` only — purpose: where the new
  field and method go.
- `crates/slicer-runtime/src/layer_executor.rs` — over 2400 lines — `validate_anchored_entity`
  only, by delegated `SNIPPETS` ≤ 20 lines — purpose: the exact AC-N1 **and** AC-N4 error wording
  (`anchored entity planar z mismatch` / `anchored entity z-span violation`) to **duplicate** into
  `slicer-wasm-host`, since the crate graph forbids calling it. **Do not read the rest; the apply
  side is out of scope and this file is never edited by this packet.**
- `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` +
  `Cargo.toml` — both whole (46 lines + ~15 lines) — purpose: the guest template.
- `xtask/src/build_guests.rs` — the `tg_root` directory-scan block only — purpose: confirm
  no guest list needs appending.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load. This packet has no canonical port.
- `target/`, `Cargo.lock`, generated bindgen output, vendored dependencies,
  `crates/slicer-wasm-host/test-guests/target/`, and the 12 leftover per-guest `target/` dirs on
  disk (cosmetic, not a broken arrangement) — never load.
- `crates/slicer-runtime/src/pipeline.rs`, `crates/pnp-cli/src/visual_debug.rs`,
  `crates/slicer-runtime/src/layer_executor.rs`'s executor-routing functions
  (`execute_per_layer_with_anchored_events`, `execute_per_layer_with_committed_anchored_events`,
  `append_same_z_entities`, `is_same_z_entity`, `execute_anchored_event_collections`) —
  `239a-anchored-host-seams` owns them.
- `modules/core-modules/**` — **narrowed by the §Open Questions resolution.** Still out of bounds
  for behaviour: no module gains a production anchored producer here; that is 239c's. The **only**
  permitted edits are the mechanical `run_support` signature widenings in
  `modules/core-modules/tree-support/src/lib.rs` and
  `modules/core-modules/traditional-support/src/lib.rs`, plus the arity-only repairs to those two
  crates' test files enumerated in §Code Change Surface item 7
  (`slicer_module_binding_tdd.rs`, `support_fill_geometry_tdd.rs`, `traditional_support_tdd.rs`,
  `traditional_family_tdd.rs`, and both crates' `enforcer_blocker_tdd.rs`, `tree_support_tdd.rs`,
  `tree_family_tdd.rs`) **if and only if** the signature change breaks them — no assertion may be
  weakened, only the call arity updated. Step 5c owns these because the breaking WIT change forces
  them. `modules/core-modules/support-surface-ironing/**` stays fully out of bounds — it binds
  `Layer::SupportPostProcess`, a different world, and its tests build a `SupportOutputBuilder`
  without ever calling `run_support` (verified), so nothing there breaks.
- `crates/slicer-gcode/src/emit.rs` and the `GCodeEmitter` impls — untouched.
- Other packet directories under `docs/spec_packets/` — never modify (Packet Safety).
- `docs/15_config_keys_reference.md` and any module manifest `.toml` — this packet declares no
  config key.
- Unrelated crates — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: enumerate every `match stage_id` site and every table that must gain a
  `"Layer::AnchoredEvents"` entry (dispatch legs, `STAGES`, `VALID_STAGES`, `STAGE_ORDER`,
  `HOST_ONLY_STAGES`, ADR-0020 meta-test array, any `stage_id()` exhaustive match); scope:
  `crates/**/*.rs` + `xtask/src/*.rs`; return: `LOCATIONS` ≤ 20; purpose: Step 1's inventory and
  Step 2's blast-radius list.
- Question: confirm the five anchored records are still referenced by zero interfaces, zero
  worlds, and zero function signatures across the whole `wit/` tree; scope:
  `crates/slicer-schema/wit/**`; return: `FACT` plus the reference count per record; purpose:
  Step 1 grounding — do not assume the split plan's F7 is still true without re-checking.
- Question: the three hard-coded `20` expectations in `xtask/src/wit_verify.rs`'s test module and
  the `.wit` literal in `crates/slicer-macros/build.rs`; scope: those two files; return:
  `LOCATIONS` ≤ 10; purpose: Step 2's declaration-model reconciliation.
- Question: `validate_anchored_entity`'s exact rejection messages for the `Planar` and
  `ZSpanning` cases; scope: `crates/slicer-runtime/src/layer_executor.rs`; return: `SNIPPETS`
  ≤ 20 lines; purpose: AC-N1's and AC-N4's asserted substrings, to be **duplicated** into
  `slicer-wasm-host` (the crate graph forbids calling the original).
- Question: return the complete `"Layer::Support"` arm of the layer-tier
  `match stage_id.as_str()` bound as `let (call_result, mut store, mem_initial_bytes) = ...` in
  `crates/slicer-wasm-host/src/dispatch.rs`, together with the `pub mod layer_support` `bindgen!`
  block and its `pub use ... as LayerSupportModule` alias in `crates/slicer-wasm-host/src/host.rs`;
  scope: those two files; return: `SNIPPETS` ≤ 60 lines; purpose: Step 5a copies the
  add-to-linker / build-store / instantiate / call shape verbatim. Explicitly confirm in the
  return which of the file's `match stage_id.as_str()` statements was read, so the prepass-tier
  match (bound as `let (call_result, mut store) = ...`) is not mistaken for it.
- Question: confirm there is no component-model, serialization, or IPC boundary for support or
  anchored events anywhere in canonical; scope: `OrcaSlicerDocumented/`; return: `FACT`;
  purpose: Step 7's honest discharge of the weak parity obligation.

## Data and Contract Notes

- **IR/manifest contracts.** No IR type changes. `LayerStageCommit::AnchoredEvents(Vec<OrderedEventCollection>)`
  and its `"Layer::AnchoredEvents"` mapping already exist in `crates/slicer-ir/src/stage_io.rs`,
  and the apply side already handles the variant in `crates/slicer-runtime/src/layer_executor.rs`.
  **This packet adds a producer path, not a commit variant.** No manifest gains a stage or a
  config key.
- **WIT boundary.** One additive method on an existing resource plus one new per-stage package.
  Both are additive to the guest-facing contract: existing guests that never call
  `set-anchored-event-collection` are unaffected in behaviour — but every guest binary is
  nonetheless **stale** the moment the `.wit` text changes, because the embedded world no longer
  matches the canonical one. Type identity across the boundary must be checked explicitly: the
  WIT method takes a single `ordered-event-collection`, the host accumulator holds
  `Option<OrderedEventCollection>`, and the commit carries `Vec<OrderedEventCollection>` — the
  Vec is built by the producer arm from the single drained proposal, so the one-to-many hop
  happens exactly once, on the host, in `convert_anchored_events`.
- **Determinism/scheduler constraints.** `Layer::AnchoredEvents` slots into `STAGE_ORDER` inside
  the `Layer::*` block. Its position must be deterministic and explicitly classified by
  `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` as user-targetable
  (i.e. added to `VALID_STAGES`, **not** to that test's `HOST_ONLY_STAGES`): a guest module is
  precisely the thing this stage exists for, so a host-only classification would contradict the
  packet's goal. The stage produces at most one commit per (layer, module) dispatch, so it adds
  no new ordering non-determinism.

## Locked Assumptions and Invariants

- **`s64` is carried as `i64` with no scaling anywhere on the path.** `planar(3000)` means
  0.3 mm and must arrive as exactly `3000`. This is a hard lock; AC-1 asserts it.
- **One proposal per dispatch.** `set_anchored_event_collection` rejects a second call within one
  dispatch and the host commits nothing in that case (AC-N3). The existing SDK guard is
  preserved, not relaxed.
- **`Ok(None)` on empty output.** The producer arm follows the `Layer::Support` /
  `Layer::Infill` convention: no proposal ⇒ no commit ⇒ arena untouched (AC-N2).
- **Both legs or neither.** The wasm arm and the native twin land in the same commit. A
  `LayerStageCommit::AnchoredEvents` produced by one leg and not the other is a defect, not a
  staged rollout (AC-6).
- **ADR-0020 stays enforceable.** After this packet the `world-layer` stage count is 9 and every
  production `LayerStageCommit` variant maps to a registered stage. The meta-test's assertion is
  updated, never weakened or `#[ignore]`d.
- **No version constant moves.** Reversibility: the whole change is additive to WIT and to two
  constant tables; reverting is a clean deletion plus a guest rebuild.

## Risks and Tradeoffs

- **Every guest goes stale at Step 2.** From that point, any component-instantiation or
  dispatch failure in Steps 3-7 is a stale-guest failure until `cargo xtask build-guests --check`
  says otherwise (exit 0). The predictable failure mode is an implementer attributing a typed-
  instantiation error to their own arm and rewriting working code. Mitigation: the gate command
  is a per-step verification line, not just an AC.
- **Step 2 is the packet's largest and carries an eight-file blast radius.** If the implementer
  measures it as `L` rather than `M`, the split boundary is: **2a** = the two `.wit` files plus
  `STAGES`/`VALID_STAGES`/`STAGE_ORDER`; **2b** = the macro call site, `build.rs`, the
  `wit_verify` counts, and the ADR-0020 gate. The workspace does **not** compile green between
  2a and 2b (the `20`-vs-`21` audit and the 8-vs-9 meta-test both fail), so 2b must follow
  immediately and the pair must be treated as one atom for verification purposes. This is the
  reason the default shape keeps them together.
- **The easiest surface to miss is the one that runs the guest.** A stage can be fully declared
  (`STAGES`, `VALID_STAGES`, `STAGE_ORDER`, `.wit`, macro preamble) and fully drained
  (`deconstruct_layer_ctx`) and still never execute, because `dispatch.rs` has *two* layer-relevant
  `match stage_id.as_str()` statements and only one of them instantiates the component. A packet
  that lists only `deconstruct_layer_ctx` produces a stage that type-checks, passes AC-4 and AC-5,
  and returns `Ok(None)` forever — a silent no-op with green declaration gates. Mitigation: Step 5a
  exists solely to own the linker/instantiate/call arm and the `host.rs` bindgen module it links
  against, and its exit condition is a guest that demonstrably runs, not a workspace that compiles.
- **Step 5 is split into 5a and 5b, both `M`, sharing `TASK-512`.** They are not independently
  shippable: 5a makes the guest callable but nothing consumes its output, and 5b's arms are
  unreachable without 5a. Verify them as one atom; the both-legs guard applies **within 5b**
  (wasm producer arm + native twin in one commit) and is unaffected by the split.
- **Adding a ninth `Layer::*` stage is a scheduler-visible change.** A stage that no module
  targets is inert at runtime, but it appears in `STAGE_ORDER`, in DAG output, and in
  `pnp_cli dag` introspection. Downstream artifacts (docs/04 stage tables, any golden that
  captures the stage list) may need reblessing; classify any drift explicitly rather than
  regenerating goldens silently.
- **`Layer::AnchoredEvents` is currently a production commit variant with no registered stage.**
  The ADR-0020 meta-test passes today only because the variant is absent from its `production`
  array — the gate has a hole exactly the shape of this packet's gap. Registering the stage
  closes it. An implementer who "fixes" the meta-test by removing the variant instead of adding
  the row would be regressing, not converging.
- **The transport has no production producer after this packet.** That is by design (239c owns
  the producer), but it means the contract is only exercised by a test guest. Do not let closure
  language imply that anchored work now flows through a real slice.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — the declaration and its eight-file cross-checked blast radius)
- Highest-risk dispatch and required return format: the Step 1 match-site / table inventory —
  `LOCATIONS` ≤ 20 entries. If that inventory is incomplete, Step 2 discovers the remainder as
  red audit tests rather than as planned edits.

## Open Questions

- `[FWD]` **`run` parameter set for the `anchored-events` interface.** The design specifies
  `(layer-index, regions: list<slice-region-view>, collection: layer-collection-builder,
  config: config-view)` by analogy with `support`. Whether `regions` is genuinely needed by an
  anchored-events module — or whether the leaner
  `(layer-index, collection, config)` is sufficient — is implementer-resolvable once the test
  guest is written. Recommendation: keep `regions`, because 239c's producer will need region
  context and removing a `run` parameter later is a breaking WIT change while ignoring one is
  free. Record the choice in the Step 2 commit message.
- `[FWD]` **`STAGE_ORDER` position.** The design places `Layer::AnchoredEvents` inside the
  `Layer::*` block; the natural slot is after `Layer::SupportPostProcess` and before
  `Layer::PathOptimization`, so anchored work is proposed after support geometry exists but
  before path optimization consumes the entity list. Implementer may confirm against
  `docs/04_host_scheduler.md` (delegated SUMMARY; that file owns the canonical `STAGE_ORDER`) and adjust; no packet change needed if the
  reasoning is recorded.
- `[FWD]` **Double-call error wording.** `set_anchored_event_collection`'s existing message names
  `run-path-optimization`, which is now wrong. Step 6 should update it to name the anchored-events
  dispatch. AC-N3 deliberately asserts the *rejection and zero commits*, not the message text, so
  the wording change cannot break the AC.
- `[RESOLVED]` **The drain this packet builds is made reachable from a `Layer::Support` guest by
  giving the `layer-support` world's `run` a second builder parameter.** Raised while
  `239c-support-layer-height-producer` was being authored; 239c recorded the mirror-image
  question and both packets now record this same resolution, so the shared seam is agreed.

  **Decision (approved; do not re-litigate).** Add `collection: layer-collection-builder` to
  `run` in `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`, so the interface
  reads `run: func(layer-index: layer-idx, regions: list<slice-region-view>,
  paint: paint-region-layer-view, output: support-output-builder,
  collection: layer-collection-builder, config: config-view) -> result<_, module-error>;` —
  **exactly** the two-builder shape
  `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` already
  uses, whose `run` takes both `output: gcode-output-builder` and
  `collection: layer-collection-builder`.

  **Rationale.** (a) It is additive to one WIT file — one parameter, one world, no new resource
  and no new package. (b) It follows an existing in-tree precedent rather than inventing a
  shape: `layer-path-optimization` proves a stage world may carry a stage-specific output
  builder *and* the generic `layer-collection-builder` side by side. (c) It keeps anchored
  transport **generic** — the drain stays on `layer-collection-builder`, the type
  `crates/slicer-sdk/src/layer_collection_builder.rs` already hosts
  `set_anchored_event_collection` / `anchored_proposal` on, which matches ADR-0059's "each
  worker returns ordered event collections" framing. Any future stage that needs the drain
  gains it the same way, by taking the same parameter.

  **Rejected alternatives (recorded, not open).**
  - *Move the drain onto `support-output-builder`* (declare `set-anchored-event-collection` on
    that resource instead). Rejected: it would confine anchored events to support stages and
    narrow the generic substrate packets 219-223 built, and it would split the method away from
    the SDK type that already owns `set_anchored_event_collection` / `anchored_proposal`.
  - *A dedicated anchored-events module in `modules/core-modules/`*, with support-derived
    anchored work routed to it. Rejected: the one-stage-per-module manifest rule (fact 1 below)
    turns this into authoring and wiring a whole sibling module — `Cargo.toml`, manifest,
    `src/lib.rs`, tests, DAG edges — plus a stated mechanism for it to see support-plan output,
    for what is a re-emission hop. The `Layer::AnchoredEvents` stage this packet declares
    remains available for genuinely independent anchored producers; it is simply not the route
    a support renderer must take.

  **The three facts below stand as recorded and are what the decision resolves.** Each was
  verified by direct read against this tree:
  1. **A module manifest declares exactly one stage.**
     `crates/slicer-scheduler/src/manifest.rs` parses the manifest with a single
     `required_stage(&root, manifest_path, "stage.id")` call returning one `StageId`; there is no
     list form and no second stage field. A module is therefore bound to one stage for its whole
     lifetime.
  2. **The `layer-support` world's `run` never sees a `layer-collection-builder`.**
     `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` declares
     `run: func(layer-index: layer-idx, regions: list<slice-region-view>,
     paint: paint-region-layer-view, output: support-output-builder, config: config-view)
     -> result<_, module-error>;` — the output handle is `support-output-builder`.
  3. **Exactly one existing stage world receives a `layer-collection-builder`.** A grep of
     `crates/slicer-schema/wit/` finds `layer-collection-builder` in only two places: its
     declaration as a `resource` in `deps/ir-types.wit`, and
     `deps/layer-path-optimization/layer-path-optimization.wit`, whose `run` takes
     `output: gcode-output-builder, collection: layer-collection-builder`. That is stage
     `Layer::PathOptimization` and nothing else.

  **Consequence before the decision, stated plainly: a `Layer::Support` guest could not reach
  `set-anchored-event-collection`.** The method hangs off `layer-collection-builder`, which was
  never handed to a support-stage `run`, and fact 1 forbids
  `modules/core-modules/tree-support` or `modules/core-modules/traditional-support` from *also*
  declaring `Layer::AnchoredEvents` to obtain one. **That is exactly the gap the decision
  closes**: after the widening, a `Layer::Support` guest receives a `layer-collection-builder`
  handle on every `run` and can call `set-anchored-event-collection` directly. The reachable set
  becomes `Layer::PathOptimization` (which already commits its own variant and so cannot also
  commit `AnchoredEvents` — see §Code Change Surface, rejected alternatives), the new
  `Layer::AnchoredEvents` stage, **and `Layer::Support`**.

  This packet's pre-existing ACs are unaffected — they are driven by a purpose-built
  `Layer::AnchoredEvents` test guest, which already receives the builder. AC-8 (added by this
  resolution) is what proves the support-stage route actually carries content; see
  `packet.spec.md`.

  **Blast radius owned by this packet.** Widening a stage world's `run` is a breaking WIT change,
  so this packet owns every co-moving surface in one step (Step 5c) with the reachability proof
  in Step 5d — see §Code Change Surface and `implementation-plan.md`. It does **not** change the
  number of `.wit` files, so the `20` expectations in `xtask/src/wit_verify.rs` are untouched by
  it; the separate 20→21 change this packet already owns belongs to the **new**
  `layer-anchored-events.wit` file and must not be conflated with this signature edit.

  The packet stays `status: draft`; this question no longer blocks activation.
