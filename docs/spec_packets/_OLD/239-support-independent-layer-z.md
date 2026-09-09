---
status: superseded
packet: 239-support-independent-layer-z
superseded_by:
  - 239a-anchored-host-seams
  - 239b-anchored-wit-contract
  - 239c-support-layer-height-producer
task_ids:
  - TASK-399
  - TASK-400
  - TASK-401
  - TASK-402
  - TASK-403
  - TASK-404
  - TASK-405
  - TASK-406
  - TASK-407
  - TASK-408
---

# 239-support-independent-layer-z

## Goal

Make support-layer Z independent of object-layer Z: support print rows at planes off the
object-layer grid are routed through the anchored-event substrate, executed by the
production pipeline (which today never invokes `execute_per_layer_with_anchored_events`),
emitted at their declared Z, and flow-scaled correctly — with the `height_delta` risk
measured before any emitter change is made.

## Problem Statement

G-02 (`docs/specs/_OLD/support-parity-gap-register.md`, destination
**239-support-independent-layer-z**): PnP has no support-layer Z independent of
object-layer Z. The anchored-event substrate (packets 219–223) already carries everything
needed — planar and Z-spanning entity contracts, deterministic committed event ordering,
and a dedicated executor entry point — but the feature is still absent from the production
slice path. There is exactly ONE blocker, not two:

1. **CORRECTED (re-verified live 2026-08-28; the 2026-08-22 characterization is REFUTED
   and is preserved here only as history).** The original claim was that
   `is_same_z_entity` (`crates/slicer-runtime/src/layer_executor.rs`) "matches nothing, so
   [an off-grid entity] is silently excluded from ordinary merging" — i.e. that off-grid
   entities fall through a routing gap. **That is wrong.** Direct read of
   `crates/slicer-runtime/src/layer_executor.rs` finds exactly three references to
   `is_same_z_entity`: its definition, a positive filter inside `append_same_z_entities`,
   and a negated filter (`!is_same_z_entity`) inside
   `execute_anchored_event_collections`. Those two filters are EXACT COMPLEMENTS over a
   single predicate, so the routing partition is ALREADY TOTAL: an on-grid same-z-support
   entity (tolerance match against `mm_to_units(anchor.z)` within
   `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`) takes the ordinary model-layer
   route, and an off-grid entity fails that match, is rejected by the ordinary route, and
   is therefore CAUGHT by the negated filter and lands in the anchored collection. It does
   not vanish at this filter. No routing gap exists to close; what remains here is a
   behavior-neutral clarity refactor (one shared named helper so the two filters cannot
   drift apart) — see `design.md` §Approach 1.
2. `crates/slicer-runtime/src/pipeline.rs` calls only the non-anchored per-layer variants
   (`execute_per_layer_with_events_and_support_tools`,
   `execute_per_layer_with_instrumentation_and_support_tools`);
   `execute_per_layer_with_anchored_events` and
   `execute_per_layer_with_committed_anchored_events` are exercised solely by tests
   (verified 2026-08-22; **re-verified live 2026-08-28 — HOLDS**). A third non-anchored
   call site was found in the same live re-verification:
   `crates/pnp-cli/src/visual_debug.rs` also calls
   `execute_per_layer_with_events_and_support_tools`, so visual-debug output shares the
   same blind spot as the slice path.

   **This is the entire mechanism of the observable defect.** The off-grid same-z support
   entity does reach the anchored collection; that collection is simply never executed,
   because no production call site invokes an anchored executor entry point. Blocker 2
   alone explains "off-grid support never prints"; blocker 1 explains nothing.

A stated-but-unmeasured risk sits in emission: `height_delta`
(`crates/slicer-gcode/src/emit.rs`) is computed per emitted row from neighbouring row Zs
and feeds volumetric E (`distance · width · height_delta · flow_factor / filament_area`);
an off-grid support pass may inherit a wrong height term. The gap register records this as
"stated, not measured" — this packet treats measurement as a gate, not a premise.

The current Orca references were regenerated with `independent_support_layer_height`
DISABLED, so G-02 is a missing canonical feature, not a measurable divergence against them;
the "Orca 205 vs PnP 150 print-Z" figure is VOID (trap T11) and never requoted. Fresh
enabled-feature references are human-owned (plan §9); this packet gates on their existence.

This is one coherent slice because both blockers plus the emission question live on one
path — plan execution → anchored routing → row synthesis → E computation — and partial
fixes produce off-grid entities that are routed but never printed.

## Architecture Constraints

- Invariants bound this packet (plan §6): 6 (same-Z support in ordinary ordering — AC-N1),
  8 (planar anchored output on declared Z), 9 (Z-spanning atomicity — AC-4), 12
  (serial/parallel determinism — AC-3), 13 (support-disabled emits nothing — AC-N3), 15
  (per-region attribution untouched — planner surface is out of bounds), 16 (non-zero
  matched tests on every command).
- Evidence standards: E1 (no vacuous assertions — every new test judges stream content, not
  artifact existence), E2 (human gate is inspection-only), E4 (freshness gate before
  slice-level evidence — AC-6), E7 (delegated Orca reads, file+function citations), E8
  (coordinate discipline below).
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Rationale for inclusion: the change surface feeds guest execution indirectly —
  `layer_executor.rs` drives `dispatch_layer_call` against guest modules and the pipeline
  switch alters which executor entry point performs those dispatches. Host-side-only edits
  would normally skip the snippet; any evidence run that slices through guest modules does
  not.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Rationale for inclusion: routing compares declared planar Z (canonical i64 units) against
  `mm_to_units(anchor.z)`; row synthesis and the conditional emitter fix handle mm floats
  (`LayerCollectionIR.z`, G-code coordinates). The existing fixture planes use unit-scale
  values (`z: 3000` = 0.3 mm).

## Data and Contract Notes

- IR/manifest contracts: none changed. `AnchoredEntity`, `OrderedEventCollection`,
  `CommittedLayerEvent`, `LayerCollectionIR` consumed as-is; no schema/version constant is
  bumped, so the version-locking constraint is dormant. If a conditional step must add a
  field (e.g. per-row provenance), that step owns the struct-literal blast radius (every
  literal site compiling against the struct) and the matching `docs/02_ir_schemas.md`
  section edit + grep in the same step.
- WIT boundary: untouched; no `.wit` edits, therefore no guest rebuild is forced by contract
  change — freshness gate still guards drift (E4/G-24: staleness presents as count
  divergence).
- Determinism/scheduler constraints: anchored interleaving order comes from the committed
  event stream (physical Z, then stable local id); parallel force must not reorder it
  (invariant 12, AC-3). Row synthesis sorts by `(z, local_id)` — the
  `sort_deterministically` keying — never by HashMap iteration.

## Locked Assumptions and Invariants

- On-tolerance same-z-support merging keeps today's exact behavior (AC-N1 pins it).
- Off-grid entities emit at their DECLARED planes; snapping is prohibited.
- No config keys, manifests, WIT, or IR shapes change in this packet.
- The `height_delta` emitter surface changes ONLY on the measured `MISSCALE_FIXED` verdict.
- Human gate requires the §9 enabled-feature references to exist under `tmp/`; they are
  never generated by this packet.

## Risks and Tradeoffs

- **Off-grid scheduling determinism** (highest): inserting intermediate rows between grid
  rows must be deterministic under rayon-forced parallelism; mitigated by deriving order
  exclusively from the committed event stream and pinning AC-3.
- **Invariant 15/16 interaction**: per-region plan attribution stays planner-owned; if row
  synthesis needed attribution data it doesn't have, that signals scope leakage — reject
  and revisit design rather than touching planner surfaces.
- **Emitter blast radius if fixed**: `height_delta` sits on the hot path of every move;
  a wrong correction mis-scales ALL extrusion, not just support. Mitigated by the
  assert-equal-within-1e-6 verdict test locking the chosen branch and by keeping the fix
  scoped to passes whose row provenance is anchored.
- **Empty-collection equivalence**: enabling the committed variant must not perturb
  support-free slices; covered by AC-N1/N3 plus existing suites (tree/traditional family,
  support-disabled) staying green.
- **Stale-guest masking** (T4): any count-divergence during evidence runs is attributed only
  after `cargo xtask build-guests --check` exits 0.
