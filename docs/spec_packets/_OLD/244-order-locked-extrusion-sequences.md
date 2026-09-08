---
status: implemented
packet: 244-order-locked-extrusion-sequences
task_ids:
  - TASK-354
---

# 244-order-locked-extrusion-sequences

## Goal

Land the generic `order_lock` contract — the `ExtrusionPath3D.order_lock: Option<u64>` carrier, its
WIT/`OrderedEntityView` projection, the SDK local-tag allocator, and host-side tag remapping plus
enforcement at all four mutation points — provably changing nothing for existing slices (all-`None`
paths take the old-equivalent branches).

## Problem Statement

Wave-overhang bridge fill (Packet 4) produces extrusion paths whose print order and direction are
physically load-bearing: fronts must be deposited anchored-first, and chained zigzag runs break if
reversed. Two downstream stages destroy such sequences today — the infill linker re-clips, chains,
and reverses bridge-role paths, and path optimization nearest-neighbor permutes role groups and may
reverse entities. A dedicated `ExtrusionRole` variant was rejected (one module's need hardcoded into
every consumer's match arms); a `Custom("…")` string convention was rejected (invisible typing,
per-consumer string matching). This packet lands the typed carrier — `ExtrusionPath3D.order_lock:
Option<u64>` — plus its WIT/`OrderedEntityView` projection, the SDK local-tag allocator, and the
host-side remap + enforcement contract, so Packet 3 can make consumers honor it and Packet 4 can
mint it. It changes nothing for existing slices: every path is `None` today, and all-`None` paths
take the old-equivalent branches.

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
