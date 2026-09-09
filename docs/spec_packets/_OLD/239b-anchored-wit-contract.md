---
status: implemented
packet: 239b-anchored-wit-contract
supersedes: 239-support-independent-layer-z
task_ids:
  - TASK-508
  - TASK-509
  - TASK-510
  - TASK-511
  - TASK-512
  - TASK-513
  - TASK-514
---

# 239b-anchored-wit-contract

## Goal

Wire the five orphaned anchored records in `crates/slicer-schema/wit/deps/ir-types.wit` into a
real WIT interface, world, and registered stage — with host lift glue, a `deconstruct_layer_ctx`
producer arm and its native twin, and SDK drain glue — so a guest module can transmit an
`ordered-event-collection` across the component boundary and the host receives it as
`LayerStageCommit::AnchoredEvents`.

## Problem Statement

The anchored-event substrate is **production-dead at the component boundary**. Packet 239
claimed the substrate "already carries everything needed"; a swarm run measured that claim false
and split 239 into three packets. This packet owns finding **F7**, the one the split plan marked
CRITICAL, plus the transport half of **F6**:

- **F7 — the WIT records are orphaned.** `crates/slicer-schema/wit/deps/ir-types.wit` (package
  `slicer:ir-handles`, interface `ir-handles`) declares five anchored records —
  `anchored-entity`, `anchored-geometry-contract` (variant `planar(s64)` |
  `z-spanning(tuple<s64, s64>)`), `anchored-entity-provenance`, `anchored-event-runtime-hooks`,
  and `ordered-event-collection` (fields `anchor-global-layer-index: u32`,
  `events: list<anchored-entity>`, `runtime-hooks`). A grep across the whole `wit/` tree found
  them referenced by **zero** interfaces, **zero** worlds, and **zero** function signatures.
  `crates/slicer-macros/src/lib.rs` and `crates/slicer-wasm-host/src/` contain **zero** lift or
  lower glue for them. A guest module cannot transmit anchored work at all today.
- **F6 — the module-commit path is dead on the guest side.** The SDK helpers
  `LayerCollectionBuilder::set_anchored_event_collection` and
  `LayerCollectionBuilder::anchored_proposal` (`crates/slicer-sdk/src/layer_collection_builder.rs`)
  exist, store a proposal in guest memory, and are drained by nothing. The proposal never leaves
  the guest.

This is one coherent slice because a transport contract is only meaningful end to end: a record
that is declared but not exported by any world, a world with no host lift, a lift with no
producer arm, and a producer arm with no guest drain are each individually untestable. The
falsifiable unit is a byte-for-byte round trip.

**What this packet does NOT do, stated because it changes the packet's shape.**
`LayerStageCommit::AnchoredEvents(Vec<OrderedEventCollection>)` **already exists** in
`crates/slicer-ir/src/stage_io.rs`, already maps to the stage-id string `"Layer::AnchoredEvents"`
via `LayerStageCommit::stage_id`, and is **already handled apply-side** in
`crates/slicer-runtime/src/layer_executor.rs`. What is missing is only that no host arm ever
*constructs* it and no guest can *produce* one. This packet therefore adds a **producer path**;
it does not add a commit variant.

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
