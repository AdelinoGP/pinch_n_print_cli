# Design: top-fill-order-and-calibration-order

## Controlling Code Paths

- Primary code path: `slicer_sdk::surface_fill_order` (new) → the three center-based fill modules packet 264 ships → `InfillRegion::solid_infill` → `remap_infill_order_locks_from` / `validate_infill_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`) → `assemble_ordered_entities_with_support_identities` → `OrderedEntityView::order_lock` → `coalesce_locked_candidates` in `modules/core-modules/path-optimization-default`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/order_lock_tdd.rs` (host order-lock tests from packet 244; test binary `executor`, aggregated by `crates/slicer-runtime/tests/executor/main.rs`), `modules/core-modules/path-optimization-default`'s `locked_block_is_single_non_reversible_candidate` / `locked_block_never_split_or_reversed` / `all_none_locks_neutrality`, `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` (test binary `e2e`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **`order_lock` expresses a block as a contiguous run of the *same* tag.** Both `validate_infill_order_locks` and `coalesce_locked_candidates` identify a block as a maximal run of equal tags; `lock_block` must therefore assign one tag to every path in the slice, not one tag per path. A per-path tag would produce N single-path blocks and silently lose the atomicity.
- **Local tags have bit 63 clear; global tags have it set.** `OrderLockAllocator::allocate` refuses to cross `1 << 63`, and `remap_order_locks_to_global` rejects tag `0`. The module emits local tags; only the host promotes them. Never emit a global tag from a module.
- **Widening the remap must widen the tag source in lockstep.** `next_global_infill_tag` derives the starting global counter from the maximum existing global tag; if it keeps scanning `sparse_infill` only while `remap_infill_order_locks_from` writes into four vectors, a later stage will reissue a tag already in use on `solid_infill`. The two functions change in the same step.
- **ADR-0062 conformance — the widening in Steps 2–3 is conformance, not a scope change.** ADR-0062 (`docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md`) states the host "remaps local tags to layer-unique global tags (bit 63 set) **at every output boundary**" and enforces the invariant "at every mutation point". The shipped `sparse_infill`-only implementation is narrower than the ADR it landed under; this packet closes that gap rather than amending the ADR. No `D-<pkt>-ADR-0062-AMENDED` deviation is needed, and none may be filed — filing one would wrongly record the ADR as changed.
- **ADR-0063 attaches non-ordering semantics to `order_lock` — the packet must satisfy them, not just the ordering ones.** ADR-0063 (`docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md`) makes locked paths **self-clipping**: the producer guarantees the entire swept footprint lies inside its legal domain, the infill linker neither clips nor links them, and the linker differences that swept footprint out of untagged fill of every role in the same region. Two obligations follow, and neither is optional:
  1. The center-based modules must emit locked top/bottom fill entirely inside their own partition polygon. This is expected to hold trivially — they fill the exposed-surface polygon the host already partitioned — but it is a producer guarantee the packet is now making, so AC-14 asserts it rather than assuming it.
  2. Locking top fill makes the linker carve its swept footprint out of untagged fill of the same region. Top solid fill and sparse infill occupy disjoint partition polygons, so the carve is expected to be a no-op; AC-14 pins that. If it is **not** a no-op, the packet has changed sparse-infill geometry as a side effect of an ordering key, which is out of scope and must stop for re-scoping rather than be absorbed.
- Schema/version constants and event-specific locking: **not applicable** — this packet adds no field and bumps no constant. `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` stays at `1.4.0`, and `docs/02_ir_schemas.md`'s version paragraph is edited for *scope wording only*. If an implementer finds themselves editing that constant, the change has left this packet's scope — stop and re-scope.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- **Selected approach:** one shared SDK ordering helper called by all three center-based modules, rather than three per-module implementations. This mirrors canonical, where `FillPlanePath::fill_surface` holds the branch once for every plane-path fill, and gives one test surface (`surface_fill_order_tdd`) for four ordering modes instead of twelve module-level cases. User-selected, 2026-09-03.
- **Exact functions, traits, manifests, tests, and fixtures:**
  - New: `crates/slicer-sdk/src/surface_fill_order.rs` — `SurfaceFillOrder` enum + `FromStr`-style parser, `order_center_based_fragments`, `lock_block`. Registered as `pub mod surface_fill_order;` in `crates/slicer-sdk/src/lib.rs` (alphabetically after `profile`), and re-exported from `crates/slicer-sdk/src/prelude.rs` alongside the existing `order_lock` exports.
  - Edited: `remap_infill_order_locks_from`, `next_global_infill_tag`, `validate_infill_order_locks` in `crates/slicer-runtime/src/layer_executor.rs` — each gains an iteration over the four `InfillRegion` vectors. `slicer_runtime_order_lock_remap` is unchanged; it already takes an arbitrary `&mut [ExtrusionPath3D]`.
  - Edited (packet 264 deliverables): `modules/core-modules/archimedean-chords-infill/{archimedean-chords-infill.toml,src/lib.rs}`, `modules/core-modules/concentric-infill/{concentric-infill.toml,src/lib.rs}`, `modules/core-modules/octagram-spiral-infill/{octagram-spiral-infill.toml,src/lib.rs}`.
  - New tests: `crates/slicer-sdk/tests/surface_fill_order_tdd.rs`, `modules/core-modules/archimedean-chords-infill/tests/{calibration_order_tdd.rs,surface_fill_order_config_schema_tdd.rs}`, plus new cases appended to `crates/slicer-runtime/tests/executor/order_lock_tdd.rs`, a new `crates/slicer-runtime/tests/contract/solid_infill_order_lock_tdd.rs` registered with a `mod solid_infill_order_lock_tdd;` line in `crates/slicer-runtime/tests/contract/main.rs`, and a new `crates/slicer-scheduler/tests/integration/per_object_calibration_order_tdd.rs` registered in `crates/slicer-scheduler/tests/integration/main.rs` (test binary `scheduler_integration`).
  - New fixture: an annulus ExPolygon (square with a square hole) built in-test from `Point2::from_mm`. A convex square clips the Archimedean spiral to one fragment and cannot exercise any ordering; every ordering AC needs the hole.
- **Rejected alternatives and reasons:**
  - *Per-module ordering implementations* — three copies of a subtle geometric branch (longest-by-length, inside-out reversal, source-path order) and three test surfaces. Rejected on maintenance and drift grounds.
  - *Wiring the calibration flag only into `archimedean-chords-infill` and leaving `top_surface_fill_order` unwired* — would make the fill-order keys declaration-only on two of their three canonical patterns, which map Authoring rule 1 prohibits outright.
  - *Widening the lock to every top fill, matching canonical's pattern-agnostic `FillBase`* — rejected and recorded as a deliberate divergence; see `requirements.md` §Recorded Divergence.
  - *Extending only `solid_infill` and leaving `ironing` / `internal_bridge_infill` narrow* — rejected (user ruling, 2026-09-03). `assemble_ordered_entities_with_support_identities` already walks all four, so a partial widening keeps the entity view and the remap inconsistent and invites a third packet.
  - *A `ResolvedConfig` field for the fill-order keys* — unnecessary. They are module-owned decision keys reaching the module through its `[config.schema]` declaration and `ConfigView::from_declared`, which is how packet 264 delivers `top_surface_density`. Adding a `ResolvedConfig` field would also drag in `overlay_resolved`'s 29-of-83 field narrowing (wayfinder tickets 118/126) for no benefit.

## Files in Scope (read + edit)

- `crates/slicer-sdk/src/surface_fill_order.rs` — role: the ordering kernel and block-locking helper; expected change: new file, plus a one-line `pub mod` in `lib.rs` and a re-export line in `prelude.rs`.
- `crates/slicer-runtime/src/layer_executor.rs` — role: host order-lock remap, tag allocation, and cross-module validation; expected change: three functions widened from one `InfillRegion` vector to four.
- `modules/core-modules/archimedean-chords-infill/src/lib.rs` and its `.toml` — role: the only module that can reach the calibration flag; expected change: three `[config.schema]` entries and a call into the SDK helper on the top/bottom solid emission path.

Extras beyond three primary files are justified: `concentric-infill` and `octagram-spiral-infill` receive the *same* two-key declaration and the *same* helper call as `archimedean-chords-infill`, with no packet-specific logic of their own. They are mechanical repetitions of Step 5, budgeted as one `S` step, not independent design surface. If they turn out to need per-module branching, that is a signal to split the packet.

## Read-Only Context

- `crates/slicer-sdk/src/order_lock.rs` — whole file (61 lines) — purpose: the `GLOBAL_BIT` convention, `OrderLockAllocator::allocate`'s exhaustion behaviour, and `remap_order_locks_to_global`'s tag-`0` rejection, all of which `lock_block` must match.
- `crates/slicer-ir/src/slice_ir.rs` — locate `pub struct InfillRegion` and `pub order_lock` by symbol, ±40 lines each — purpose: the four vector names and the `Option<u64>` carrier.
- `modules/core-modules/path-optimization-default/src/lib.rs` — locate `coalesce_locked_candidates` by symbol, ±40 lines — purpose: confirm a block is a maximal run of *equal* tags before `lock_block` is written.
- `docs/02_ir_schemas.md` — the `ExtrusionPath3D.order_lock` / schema `1.4.0` paragraph only, located by grep — purpose: the exact wording to correct.
- `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` and `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` — whole files (both short) — purpose: the two Architecture Constraints above; these are the normative contracts on `order_lock` and must be read before Step 2 and Step 5.
- `docs/spec_packets/264-top-bottom-surface-keys/packet.spec.md` — via bounded SUMMARY dispatch only — purpose: the three modules' names, claim declarations, and emission entry points.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load. The oracle path is named in the wayfinder map's Notes and must be re-derived from there.
- `D:\slicerProject\Orca(pnp_gui)` — never use as a canonical oracle at all: GUI-only fork, FFF slicing pipeline removed, every file this packet cites is absent from it.
- `docs/spec_packets/264-top-bottom-surface-keys/design.md` and `implementation-plan.md` — another packet's internals; SUMMARY-dispatch its `packet.spec.md` instead. Never modify any file in 264's directory.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `docs/specs/orca-feature-gap/map.md` in full — delegate the two named Notes bullets.
- `crates/slicer-runtime/src/layer_executor.rs` **as a whole-file read** — symbol-located windows only.

## Expected Sub-Agent Dispatches

- Question: does any acceptance criterion in packet 244 assert that order-lock remap/validation is *restricted* to `sparse_infill`?; scope: `docs/spec_packets/244-order-locked-extrusion-sequences/packet.spec.md`; return: `FACT`; purpose: Step 2 — if yes, stop and re-scope rather than silently contradicting a closed packet.
- Question: what are the three center-based modules' crate names, manifest filenames, and the function that emits their top/bottom solid paths?; scope: `docs/spec_packets/264-top-bottom-surface-keys/packet.spec.md`; return: `SUMMARY`; purpose: Steps 4–5.
- Question: what is the exact `[config.schema]` TOML shape for a string key with an allowed-value list and for a bool key?; scope: `docs/03_wit_and_manifest.md`; return: `SNIPPETS` (≤2, ≤30 lines); purpose: Step 4.
- Question: which test files construct `ExtrusionPath3D` struct literals that would need a `..` FRU under the check-literals gate once new test literals are added?; scope: `crates/slicer-runtime/tests/**`, `crates/slicer-sdk/tests/**`; return: `LOCATIONS`; purpose: Steps 1–3 blast radius.
- Question: what is the next free `DEV-###` in the deviation log?; scope: `docs/DEVIATION_LOG.md`; return: `FACT`; purpose: Step 6 — must be re-derived at that moment, never carried from this document.

## Data and Contract Notes

- IR/manifest contracts: no IR change. Three new manifest keys on `archimedean-chords-infill`, two each on the other two center-based modules. All key strings are snake_case per `CLAUDE.md`.
- WIT boundary: unchanged. `order-lock: option<u64>` already exists on both `extrusion-path3d` (`crates/slicer-schema/wit/deps/types.wit`) and `ordered-entity-view` (`crates/slicer-schema/wit/deps/ir-types.wit`); no `bindgen!` regeneration and no WIT edit is in scope.
- Determinism/scheduler constraints: `OrderLockAllocator` issues tags deterministically from 1, and the host remap walks regions and vectors in a fixed order, so global tag assignment is reproducible across runs. The vector iteration order chosen in Step 2 (`sparse_infill`, `solid_infill`, `ironing`, `internal_bridge_infill`) becomes part of that determinism contract and must match the order `assemble_ordered_entities_with_support_identities` already uses.

## Locked Assumptions and Invariants

- **Lock: the four-vector iteration order.** Once `remap_infill_order_locks_from` assigns global tags in a fixed vector order, changing that order renumbers tags in existing serialized fixtures. Matching `assemble_ordered_entities_with_support_identities`'s existing order makes the choice non-arbitrary and is asserted by AC-5.
- **Lock: one tag per block.** `lock_block`'s contract — one shared tag across a contiguous slice — is depended on by `coalesce_locked_candidates`. Reversible only by changing both.
- Everything else is reversible via config defaults: all three new keys default to canonical's defaults (`"default"`, `"default"`, `false`), and AC-N1 asserts the default path is byte-identical.

## Risks and Tradeoffs

- **Forward dependency on a draft packet.** Steps 4–6 cannot start until 264 is implemented. If 264's module names or emission signatures change during its implementation, this packet's Step 5 must adapt — which is why the module shape is a SUMMARY dispatch at implementation time rather than a frozen fact here.
- **Widening the order-lock remap touches a packet-244 invariant.** The mitigation is the FACT dispatch above plus AC-N1's byte-identity assertion: with no module emitting a lock today, widening the scan is observationally inert until Step 5 lands.
- **The annulus fixture is load-bearing.** Every ordering AC is vacuous on a convex region, because the spiral clips to one fragment and every ordering mode returns the same single-element sequence. A reviewer seeing all ordering tests pass on a square fixture should treat that as a red flag, not evidence.
- **`calib_flowrate_topinfill_special_order` is `comDevelop` in canonical** — no label, no tooltip, set only by the GUI calibration object generator. It is implemented here because it has live slicing-pipeline read sites (map Authoring rule 3 is about read sites, not UI exposure), but it will never appear in a normal user's config.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — the host widening, three coupled functions plus their fixtures)
- Highest-risk dispatch and required return format: the packet-244 exclusion question — `FACT`, because a wrong answer silently contradicts a closed packet's contract.

## Open Questions

- `[FWD]` Should `lock_block` refuse an empty slice (returning `Ok` without consuming a tag) or consume one anyway? Implementer's call; assert whichever is chosen in `surface_fill_order_tdd` so the behaviour is pinned rather than incidental.
- `[FWD]` `concentric-infill` produces closed loops rather than clipped fragments of one source polyline, so `restore_source_path_order`'s premise does not hold for it directly. Canonical still reads `top_surface_fill_order` for `ipConcentric` — confirm against `FillConcentric::_fill_surface_single` how the loop sequence is ordered there, and if the port's concentric module already emits loops center-outwards, `Outward` may be the identity and `Inward` a plain sequence reversal. Resolve by delegated canonical read at Step 5; do not guess.
