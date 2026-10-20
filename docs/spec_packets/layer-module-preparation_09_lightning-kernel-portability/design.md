# Design: lightning kernel portability (Gate B)

## Selected Approach

Gate B is one bounded experiment: the same owned kernel source receives the canonical planning
domains published by packet 07 and runs through the real native entry and the real WASM guest. The
driver decodes both outputs, compares each independently to the canonical graph/grounding/clipped
records, and records a truthful verdict. The kernel never calls the host lightning producer; the
adapter is ordinary-stage only.

The fixture branches are already separated by packet 07. This packet adds no expected data to the
guest or adapter: `KernelInput` is input-only; expected graphs/clips live in the host comparator.
Packet 07's `BInputEnvelope` wraps `KernelInput` plus per-dispatch `FinalFillEvidence`; B parses it
in support and passes only `KernelInput` to the guest.

## Kernel Surface

- `plan_lightning(input: &KernelInput) -> Result<KernelPlan, KernelError>` (`planner.rs`) implements
  canonical top-down planning over object-layer pooled `PlanningLayer` domains:
  - ordered nodes with parent/root IDs, origin region index, and grounding witness;
  - every directed parent→child edge occurrence with stable local IDs;
  - root-point-hash sampling (`convertToPolylines` policy), producing `sampled_segments`;
  - complete `GraphProvenance` (source catalog, boundary provenance, operations) carried from the
    input, with newly selected witnesses resolved against it.
- `clip_lightning(plan, input) -> Result<Vec<ClippedLayer>, KernelError>` clips sampled segments to
  each recorded final fill, keyed by dispatch token, caller identity, and contributor set. It
  implements the canonical `intersection_pl` semantics; it does not clip to planning domains.
- `tree.rs` owns the node/parent/root/grounding machinery. No host-algos, `rayon`,
  `boostvoronoi`, `slicer-runtime`, or production Lightning dependency enters the crate.
- A numerical/topological mismatch is a valid `fail`; no tolerance widening or expected-data
  rebaseline is permitted.

## Transport

Use only `ExtrusionPath3D.points` and each `Point3WithWidth`'s x/y/z/width/flow fields. Every
planned record is one six-point sparse path with finite coordinates, actual layer Z, positive
width, flow 1, speed 1. Metadata integers must be exactly representable in f32 (`< 1 << 24`); zero
encodes absent parent/witness fields. Three tags, ordered `(x,y)` pairs:

- Node tag 1: `[(1,node_id), (x_mm,y_mm), (parent_id_or_0,root_id), (ground_kind,origin_region_index), (surface_id,ring_id), (segment_id,0)]`.
  Every node, including isolated and coincident-but-distinct nodes. Nonroot witness slots are zero.
- Edge tag 2: `[(2,edge_id), (parent_id,child_id), (parent_x,parent_y), (child_x,child_y), (root_id,origin_region_index), (0,0)]`.
  Every occurrence, no geometry dedup.
- Provenance tag 3: the complete `GraphProvenance` DTO as canonical UTF-8 JSON, split into
  eight-byte chunks: `[(3,chunk_index), (byte_len,chunk_count), (b0,b1), (b2,b3), (b4,b5), (b6,b7)]`.
  Zero-pad only the last chunk; reject nonzero padding, invalid UTF-8, unknown/omitted fields, and
  missing/extra/duplicate chunks.

The guest decodes test-only config strings `probe_input_json`, `probe_output` (`planned` or
`clipped`), and `probe_final_dispatch`. For `clipped`, it executes the whole plan and emits only
the selected dispatch's two-point segment multiset with its actual caller origin. `PaintValue::Custom`
inputs are rejected before dispatch; every emitted path must have an explicit `Some` origin because
the current boundary has no clear-origin call.

## Adapters and Driver

- Guest: `crates/slicer-wasm-host/test-guests/lightning-portability-guest/` uses
  `#[slicer_module] impl LayerModule for LightningPortabilityGuest` with the existing `run_infill`
  signature, `cdylib` + `rlib`, `wit-bindgen`, the owned kernel path dependency, and an isolated
  workspace sentinel. Its generated `__slicer_native_entry()` is the native adapter.
- Native: construct `NativeLayerRequest`, call the generated entry, drain
  `NativeLayerResponse.infill` via `sparse_paths()`/`sparse_path_origins()`, assert equal lengths
  before zipping, require every origin `Some`, and reject `None`/Custom.
- WASM: use the current `host::layer_infill::LayerModule::add_to_linker`/`instantiate`,
  `HostExecutionContextBuilder`, config/region/paint/output handles, the generated
  `slicer_layer_infill_infill().call_run(...)`, and `HostExecutionContext::infill_output`; fresh
  stores per call. Check both the trap/result layer and the collected records.
- Both feeds enter the same decoder. Compare each to the canonical reference independently; never
  to each other.

## Comparison

- Node bijection keyed by `(object_id, global_layer_index, node_id)` for construction, with
  coordinates matched within the recorded transport tolerance, preserving origin identity, parent/
  root relations, and ground kind/witness. Local IDs may be bijectively relabeled; distinct nodes
  may not merge.
- Equality of the complete directed edge-occurrence multiset, isolated roots, and all grounding
  labels. Missing/extra/duplicate/reversed/disconnected/corrupt records fail with the first
  difference named.
- Complete normalized boundary/source/contributor/operation table equality under the same
  correspondence, including boundaries no root uses. Witness addresses resolve in the returned
  tag3 table, not only the input table.
- Clipped: full missing/extra segment equality per dispatch, oriented multiset, plus containment.
  A dispatch whose canonical `contributor_evidence` is `unavailable` is `inconclusive` with the
  named reason; its segments are still compared and its count still contributes to the executed
  population. No dispatch is skipped.

## Outcome

`target/lightning-portability/outcome.json` records `gate: "B"`, status, executed flag, kernel
source hash, guest artifact/build identity, `build-guests --check` exit, native/WASM executed
counts, per-case planned/clipped statuses, commands/exits/log hashes, and named blockers.
`migration_eligible` is the conjunction of A `pass`, B `pass`, and independently recorded generic
framework acceptance. This packet does not verify framework acceptance, so it records that as the
named blocker instead of inferring it from packet status.

## Measured Facts

- The recorder's current capture has five Classic families with canonical planning domains,
  scalars, graph, grounding, callers, and clipped polylines.
- `anchor_bridge` classic planned graph: 27 nodes / 16 edges / 11 roots. `hole_void` classic:
  41/27/14. These are reference populations, not pass targets.
- The owned kernel model already defines `KernelInput`, `KernelPlan`, `GraphLayer`, `GraphNode`,
  `GraphEdge`, `GroundingWitness`, `ClippedLayer`, `ClippedFill`, and `GraphProvenance`; this
  packet implements behavior, not new schema.
- Final-fill contributor evidence is incomplete for surfaces created after the pre-anchor snapshot;
  the per-dispatch rule above is required, not optional.

## Files in Scope

| Area | Files |
| --- | --- |
| Kernel graph/clip | `modules/core-modules/lightning-infill/feasibility-kernel/src/{tree,planner}.rs` (+ Cargo-generated lock) |
| Guest | `crates/slicer-wasm-host/test-guests/lightning-portability-guest/{Cargo.toml,src/lib.rs,Cargo.lock}` |
| Host dev wiring | `crates/slicer-wasm-host/Cargo.toml` dev-dependencies, root `Cargo.lock` (generated) |
| Driver/tests | `crates/slicer-wasm-host/tests/integration/{main,lightning_portability_tdd,lightning_portability_support}.rs` |
| Outcome | `docs/specs/layer-module-preparation-plan.md` (append-only), `docs/07_implementation_status.md` (`TASK-576` B line) |

Packet 07's fixtures, `model.rs`, `lib.rs`, and the recorder are read-only here.

## Read-Only Context

Bounded windows only: kernel model/API, guest discovery/freshness, native request/entry and output
builder, the existing WIT infill world, host boundary example, and packet 07's fixture manifest.
Do not load guest WASM, full JSON graphs, lockfile contents, or generated bindings.

## Out-of-Bounds Files

Production lightning/producer/driver/paint, WIT/SDK/macros/runtime, packet 07 fixture files and
recorder, packet 08, other packets, ADRs/deviations, source-plan approved body, backlog entries
other than `TASK-576`.

## Dispatches

- Canonical tree/grounding/sampling/clipping fact checks: `SUMMARY ≤200 words`, file+function.
- Guest build/freshness and native/WASM execution: `FACT` exits and counts, ≤20 failure lines.
- Comparator/difference audit: `SUMMARY ≤200 words`.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Exact freshness outcomes:0 fresh;1 stale or lock divergence (rebuild or `--sync-locks` respectively);3 infrastructure unable to form an opinion, stop. Ordinary/accelerated artifacts are distinct; this gate uses ordinary artifacts. No guest failure attribution before this check. Guest execution uses `cargo xtask test`.

## Invariants

- No host lightning producer, host fallback, or callable host-algos path in the native adapter; the
  owned kernel and wasm32 guest closures exclude `host-algos`. Legitimate native SDK dependencies
  may contain it.
- No new WIT/SDK/public schema. The guest uses the existing `slicer:layer-infill` world.
- No `None`/`Custom` origin; equal path/origin lengths before zip.
- Scientific mismatch is recorded as a named `fail`/`inconclusive`; never repaired from expectations.
- Each decision test re-runs the driver in its own body, so AC-2/AC-3/AC-5 do not depend on test
  execution order.

## Risks and Tradeoffs

- The kernel port may differ numerically or topologically. That is a valid B `fail` and the
  packet's product.
- Clipped contributor evidence is partial by recorder limitation; the per-dispatch `inconclusive`
  rule keeps the experiment honest without blocking it.
- Guest instantiation/freshness can fail for infrastructure reasons; exit 3 is a stop, not a
  scientific result.

## Context Cost Estimate

Aggregate M across six bounded steps; largest step M. The tree port and the driver each need
narrow delegated fact checks, not full canonical source reads.

## Open Questions

- [FWD] Does the owned kernel match the canonical planned graph/grounding on any family, and where
  is the first difference? Recorded at execution.
- [FWD] Does clipped output match segment multisets for dispatches with complete evidence?
  Recorded per dispatch.
- Limitation: final-fill contributor provenance is incomplete; affected dispatches are `inconclusive`
  and remain a migration requirement.
- Limitation: Arachne/translated/density companions are not gate coverage.
