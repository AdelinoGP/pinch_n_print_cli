# Design: 244-order-locked-extrusion-sequences

## Controlling Code Paths

- Primary code path: `ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`) → WIT
  `extrusion-path3d` (`crates/slicer-schema/wit/deps/types.wit`) → `OrderedEntityView` projection
  (`crates/slicer-runtime/src/layer_executor.rs::project_ordered_entities` and
  `crates/slicer-wasm-host/src/dispatch.rs::project_ordered_entities_from`) → SDK
  `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`) → macro adapter
  (`crates/slicer-macros/src/lib.rs`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs`
  (drives `apply_entity_order_proposal`), `crates/slicer-sdk/tests/finalization_builder_tdd.rs`
  (drives `modify_entity` / `sort_layer_by` / `apply_to`), `crates/slicer-gcode/src/emit.rs`
  (`apply_cross_layer_tool_rotation`), `crates/slicer-runtime/tests/executor/order_lock_tdd.rs`
  (new — InfillPostProcess commit, all-`None` neutrality, remap-wiring tests).
- OrcaSlicer comparison: none — this is host/IR plumbing, not a parity surface.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Schema/version constant: `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is the single source of
  truth; production constructors read the constant, not a literal. The additive bump (1.3.0 → 1.4.0)
  has **no** literal hard-assert anywhere in the tree — the constant-sourced tests
  (`ir_tests.rs::chunk2_ir_schema_versions_are_default_sourced`,
  `visual_debug_postpass_tap_tdd.rs`) compare to the constant and pass automatically. (The plan's
  "sweep the test that hard-asserts the old constant value" is a no-op for this constant; the
  packet-226 `tool_index` precedent's sweep applied to a different constant.)
- `ExtrusionPath3D` becomes a **watched type** (5 named fields, `pub`, under `crates/*/src`) after
  this packet: test literals must use a `..` rest or an `// exhaustive:` waiver per
  `docs/21_data_defaults_and_fixtures.md`. `ExtrusionPath3D` has **no** `Default` impl, so FRU needs
  a fixture base (`slicer_sdk::test_support::extrusion_path3d_base(role)`) or a waiver — not
  `..Default::default()`.

## Code Change Surface

- Selected approach: additive `#[serde(default)] pub order_lock: Option<u64>` on `ExtrusionPath3D`,
  projected onto `OrderedEntityView` end-to-end, with a host-side remap (local → global tags) and
  enforcement at the four mutation points. `None`/absent preserves today's behavior exactly.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/slice_ir.rs` — `ExtrusionPath3D` (line 2364) gains the field; the
    production constructors `variable_width` (line 2341) and `extrusion_line_to_extrusion_path3d`
    (line 2463) gain `order_lock: None`; `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (line 338)
    bumps to `1.4.0`.
  - `crates/slicer-schema/wit/deps/types.wit` — `record extrusion-path3d` (line 19) gains
    `order-lock: option<u64>`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `record ordered-entity-view` (line 260) gains
    `order-lock: option<u64>`.
  - `crates/slicer-runtime/src/layer_executor.rs` — `OrderedEntityView` (line 2336) gains
    `order_lock: Option<u64>`; `project_ordered_entities` (line 2372) populates it from
    `entity.path.order_lock`; the `LayerStageCommit::Infill` / `LayerStageCommit::InfillPostProcess`
    commit arms of `apply` (line 3007) wire `remap_order_locks_to_global` at the output boundary;
    the `ExtrusionPath3D` literals in `append_same_z_entities` (line 452) and `apply` (line 3156)
    gain `order_lock: None`.
  - `crates/slicer-wasm-host/src/dispatch.rs` — `project_ordered_entities_from` (line 2416) populates
    the WIT `OrderedEntityView.order_lock`.
  - `crates/slicer-sdk/src/views.rs` — `OrderedEntityView` (line 807) gains `order_lock: Option<u64>`.
  - `crates/slicer-macros/src/lib.rs` — the `__slicer_populate_layer_collection` adapter (line 3190)
    maps `e.order_lock`; the two `ExtrusionPath3D` WIT→IR converters (lines 1318, 2657) gain
    `order_lock: None` (or map the WIT field).
  - `crates/slicer-wasm-host/src/marshal/leaf.rs` and `crates/slicer-wasm-host/src/host.rs` — the
    `ExtrusionPath3D` WIT→IR converters gain `order_lock`.
  - `crates/slicer-sdk/src/order_lock.rs` (new) — `OrderLockAllocator` (see Data and Contract Notes).
  - `crates/slicer-runtime/src/layer_executor.rs` (or a new `order_lock.rs`) — `remap_order_locks_to_global`.
  - `crates/slicer-ir/src/stage_io.rs` — `LayerStageError::OrderLockViolation { message: String }`.
- Rejected alternatives and reasons:
  - New `ExtrusionRole` variant — rejected: role proliferation, scattered match arms (plan D2).
  - `Custom("…")` string convention — rejected: invisible typing, per-consumer string matching.
  - Entity-group wrapper type in `InfillIR`/`LayerCollectionIR` — rejected: restructures every
    downstream iteration for a guarantee a field carries.
  - Trusting core modules to comply without host enforcement — rejected: third-party optimizers and
    future finalization sorters could silently violate the invariant.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - role: carrier + constant; expected change: field, two
  constructors, version bump.
- `crates/slicer-schema/wit/deps/types.wit` - role: WIT carrier; expected change: `order-lock`.
- `crates/slicer-schema/wit/deps/ir-types.wit` - role: WIT projection; expected change: `order-lock`.
- `crates/slicer-runtime/src/layer_executor.rs` - role: host projection + remap + InfillPostProcess
  enforcement + proposal enforcement; expected change: field, remap fn, validation.
- `crates/slicer-runtime/src/visual_debug_render.rs` - role: literal blast radius (1 literal);
  expected change: `order_lock: None` in `support_geometry_shapes`.
- `crates/slicer-wasm-host/src/dispatch.rs` - role: wasm-host projection; expected change: field.
- `crates/slicer-sdk/src/views.rs` - role: SDK projection; expected change: field.
- `crates/slicer-sdk/src/test_support/fixtures.rs` - role: fixture base literal; expected change:
  `order_lock: None` in `extrusion_path3d_base` (production `src/` literal, compiler-enforced).
- `crates/slicer-macros/src/lib.rs` - role: macro adapter; expected change: field mapping.
- `crates/slicer-wasm-host/src/marshal/leaf.rs`, `crates/slicer-wasm-host/src/host.rs` - role: WIT→IR
  converters; expected change: `order_lock: None`.
- `crates/slicer-sdk/src/order_lock.rs` (new) - role: allocator; expected change: new type.
- `crates/slicer-sdk/src/traits.rs` - role: finalization `apply_to` enforcement + remap wiring at the
  finalization merge; expected change: `modify_entity`/`sort_layer_by` validation + remap call in
  `apply_to`.
- `crates/slicer-gcode/src/emit.rs` - role: literal blast radius (1 literal, `tool_entity` in the
  inline `#[cfg(test)]` mod) + tool-rotation lock-awareness; expected change: `..` rest or
  `// exhaustive:` waiver for the literal (test code — `check-literals` enforces FRU/waiver, not
  exhaustive literals); `apply_cross_layer_tool_rotation` treats locked blocks as units (extends
  the rotation range to the block boundary; reads `entity.path.order_lock` on `PrintEntity`);
  inline `#[cfg(test)]` test `order_lock_tool_rotation_preserves_block`.
- `crates/slicer-ir/src/stage_io.rs` - role: error variant; expected change: `OrderLockViolation`.
- `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs` - role: proposal/remap tests;
  expected change: new tests.
- `crates/slicer-sdk/tests/finalization_builder_tdd.rs` - role: finalization/allocator tests;
  expected change: new tests.
- `crates/slicer-runtime/tests/executor/order_lock_tdd.rs` (new) - role: InfillPostProcess
  enforcement (AC-N1), all-`None` neutrality (AC-3), remap-wiring (AC-7) tests; expected change: new
  file.
- `crates/slicer-runtime/tests/executor/main.rs` - role: aggregator; expected change: `mod order_lock_tdd;`.
- `crates/slicer-core/src/perimeter_utils.rs` - role: literal blast radius (2 literals, both in a
  `#[cfg(test)]` mod); expected change: `..` rest or `// exhaustive:` waiver (test code —
  `check-literals` enforces FRU/waiver, not exhaustive literals).
- `modules/core-modules/wipe-tower/src/lib.rs` - role: literal blast radius (4 literals); expected
  change: `order_lock: None`.
- `modules/core-modules/tree-support/src/lib.rs` - role: literal blast radius (2 literals); expected
  change: `order_lock: None`.
- `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` (new) - role: ADR.
- `CONTEXT.md` - role: glossary; expected change: two terms.
- `docs/02_ir_schemas.md` - role: docs; expected change: IR 10 section.

## Read-Only Context

- `crates/slicer-runtime/src/layer_executor.rs` - lines 2687-2756 only - purpose: `apply_entity_order_proposal`
  / `apply_order_proposal` shape (the validation hook point).
- `crates/slicer-sdk/src/traits.rs` - lines 1338-1560 only - purpose: `apply_to` phases and the
  `modify_entity` / `sort_layer_by` application points.
- `crates/slicer-gcode/src/emit.rs` - lines 942-1000 only - purpose: `apply_cross_layer_tool_rotation`
  cluster-rotation shape.

## Out-of-Bounds Files

- `modules/core-modules/infill-linker/**`, `modules/core-modules/path-optimization-default/**` - the
  lock-honoring consumers are Packet 3; do not edit.
- `modules/core-modules/wave-overhangs/**` - does not exist yet; Packet 4.
- `docs/07_implementation_status.md`, `docs/specs/wave-overhangs-bridge-fill-plan.md` - orchestrator-owned.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.

## Expected Sub-Agent Dispatches

- Question: list every exhaustive `ExtrusionPath3D { … }` literal (no `..` rest) in `crates/*/src`
  and `modules/*/src` that must gain `order_lock: None`; scope: `crates/**/src/**/*.rs`,
  `modules/**/src/**/*.rs`; return: `LOCATIONS` (≤ 20 entries); purpose: pre-bake the production
  literal blast radius for Step 1.
- Question: confirm no test hard-asserts the literal `SemVer { major: 1, minor: 3, patch: 0 }` for
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`; scope: `crates/**/tests/**/*.rs`; return: `LOCATIONS`;
  purpose: confirm the constant-bump fallout is empty (constant-sourced only).

## Data and Contract Notes

- IR contract: `order_lock: Option<u64>` is additive and `#[serde(default)]`; pre-1.4.0 serialized
  fixtures deserialize to `None` (unchanged behavior). The bump is additive-minor per the IR
  Versioning Contract table ("New optional field added → Minor").
- WIT boundary: `order-lock: option<u64>` on both records; the WIT files carry no versioned package
  path (host bindgen and guest macro read them directly), so no WIT version tax.
- Tag semantics (plan D11): **local tags** are `1..2^63-1`, allocated by `OrderLockAllocator`
  (invocation-local, deterministic discovery order, `None` on exhaustion); `Some(0)` is rejected at
  the output boundary. **Global tags** have bit 63 set; the host remaps local → layer-unique global
  at every output boundary (`LayerStageCommit::Infill` commit, `LayerStageCommit::InfillPostProcess` commit, finalization
  merge); unknown global tags in module output are a contract error.
- `OrderLockAllocator` shape: `pub struct OrderLockAllocator { next: u64 }` with
  `pub fn new() -> Self` (starts at 1) and `pub fn allocate(&mut self) -> Option<u64>` (returns
  `Some(next)` then increments; `None` once `next >= 1 << 63`).
- `remap_order_locks_to_global` shape: `pub fn remap_order_locks_to_global(paths: &mut [ExtrusionPath3D], next_global: &mut u64) -> Result<(), String>`
  — `Some(t)` with bit 63 clear → `Some((1 << 63) | *next_global)` and `*next_global += 1`;
  `Some(0)` → `Err`; `Some(t)` with bit 63 set → `Err` unless `t < (1 << 63) | *next_global`
  (already minted); `None` → unchanged.
- Remap wiring: `remap_order_locks_to_global` is called at the output boundaries — the
  `LayerStageCommit::Infill` / `LayerStageCommit::InfillPostProcess` commit arms of `apply`
  (`crates/slicer-runtime/src/layer_executor.rs`) and the finalization merge in `apply_to`
  (`crates/slicer-sdk/src/traits.rs`) — so module output carrying local tags is rewritten to
  layer-unique global tags before it reaches the `LayerCollectionIR`. All-`None` slices are
  unaffected (no producer mints locks yet).
- Enforcement invariant (plan D3, verbatim into ADR-0062): paths sharing a tag within one
  `(layer, object, region)` form an atomic contiguous sequence — adjacent, in authored order and
  point direction; the block may move as a unit. Locks protect sequence and geometry (points,
  widths); speed/flow side mutations remain legal.

## Locked Assumptions and Invariants

- `order_lock` is a per-path marker, not a role and not tied to any one module — any fill holder may
  lock its output.
- All-`None` neutrality is a hard invariant: with no producer minting locks, every existing slice is
  byte-identical to today (AC-3).

## Risks and Tradeoffs

- The `ExtrusionPath3D` literal blast radius is large (100+ literals); it is compiler-enforced for
  `src/` and `check-literals`-enforced for tests. The plan's "production literals gain the field,
  FRU rest in tests" is correct, but `ExtrusionPath3D` has no `Default` impl, so test FRU needs a
  fixture base or a waiver — pre-baked in §Code Change Surface.
- The plan's "sweep the test that hard-asserts the old constant value" is a no-op for
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (no literal hard-assert exists); noted as a grounding
  deviation, not a blocker.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1 field + WIT + projection + schema bump, which owns the literal blast
  radius)
- Highest-risk dispatch and required return format: the production-literal `LOCATIONS` sweep
  (≤ 20 entries).

## Open Questions

None.
