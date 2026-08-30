# Support Independent Layer Z — Split Plan

Origin: a `/swarm 239-support-independent-layer-z` run (2026-08-28) measured packet 239's
central premise as false and its true scope as XL. Packet 239 is superseded by the three
packets queued below. This file is the plan-of-record for that split.

## Why the split

Packet 239's `requirements.md` claimed:

> The anchored-event substrate (packets 219–223) already carries everything needed — planar
> and Z-spanning entity contracts, deterministic committed event ordering, and a dedicated
> executor entry point — but two verified blockers keep it out of the production slice path.

This is false. The substrate is **test-only, end to end**. Nine findings, each verified live
during the swarm run by direct read or delegated dispatch:

**F1 — 239's Blocker 1 is REFUTED.** `crates/slicer-runtime/src/layer_executor.rs` contains
exactly three references to the private `is_same_z_entity`: its definition, a positive filter
in `append_same_z_entities`, and a negated filter (`!is_same_z_entity`) in
`execute_anchored_event_collections`. These are **exact complements**, so the executor routing
partition is **already total**. An off-grid `AnchoredGeometryContract::Planar { z }` fails the
tolerance match, is rejected by the ordinary route, and is therefore *caught* by the anchored
route. It does not fall through a gap. 239's `requirements.md` ("matches nothing, so it is
silently excluded") and `design.md` ("matches neither route and vanishes") are both wrong.
**Consequence:** AC-2 and AC-N2 as 239 wrote them cannot be made red at the executor level;
they are only genuinely red at pipeline level.

**F2 — 239's Blocker 2 HOLDS.** No production call site invokes
`execute_per_layer_with_anchored_events` or `execute_per_layer_with_committed_anchored_events`.
`crates/slicer-runtime/src/pipeline.rs` calls only
`execute_per_layer_with_events_and_support_tools` and
`execute_per_layer_with_instrumentation_and_support_tools`. A **third** non-anchored caller
exists that 239 never recorded: `crates/pnp-cli/src/visual_debug.rs`.

**F3 — no injection seam.** `PipelineConfig` (`crates/slicer-runtime/src/pipeline.rs`) has no
anchored-entity field. Its fields are `cancel_flag`, `mesh_ir`, `plan`, `runners`,
`support_tools`, `resolved_configs`, `default_resolved_config`, `bounds`, `wasm_handles`. No
public entry point can carry `&[AnchoredEntity]`.

**F4 — no emission representation.** `GCodeEmitter::emit_gcode(&self, layer_irs:
&[LayerCollectionIR])` (`crates/slicer-gcode/src/emit.rs`). `LayerCollectionIR` carries exactly
one `z: f32` and one `global_layer_index: u32` per row, so a row *is* a whole layer at a single
Z. `CommittedLayerEvent::Anchored(OrderedEventCollection)` has nowhere to go.

**F5 — no producer anywhere.** `AnchoredEntity` appears in exactly four production files:
`crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs`,
`crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-scheduler/src/execution_plan.rs`.
Every other reference is a test, and all five `AnchoredEntity` literal sites are tests.
`ExecutionPlan` stores no anchored entities; `ExecutionPlan::anchored_invocation` is a
`&self`-ignoring pure transform forwarding to `AnchoredInvocation::from_entity`. The
`capability_derived_anchor_closure` test constructs its entities inline; there is **no**
manifest-to-`AnchoredEntity` derivation in any `src/`.

**F6 — module-commit path dead.** `LayerStageCommit::AnchoredEvents(Vec<OrderedEventCollection>)`
exists in `crates/slicer-ir/src/stage_io.rs`, but the writer
`Blackboard::set_anchored_event_collections` and the only reader
`take_anchored_event_collections` both sit inside `execute_anchored_event_collections` in
`layer_executor.rs` — a closed host loop. No guest module commits it. The SDK helpers
`set_anchored_event_collection` / `anchored_proposal`
(`crates/slicer-sdk/src/layer_collection_builder.rs`) have no drain glue and no callers.

**F7 — WIT types orphaned (CRITICAL).** `crates/slicer-schema/wit/deps/ir-types.wit` declares
`anchored-entity`, `anchored-geometry-contract`, `anchored-entity-provenance`,
`anchored-event-runtime-hooks`, and `ordered-event-collection`. Verified by grep across the
entire `wit/` tree: these are referenced by **zero** interfaces, **zero** worlds, and **zero**
function signatures. `crates/slicer-macros/src/lib.rs` and `crates/slicer-wasm-host/src/`
contain **zero** lift/lower glue for them. A guest module cannot transmit anchored work across
the WIT boundary at all today.

**F8 — support Z is structurally grid-bound.** `modules/core-modules/tree-support` and
`modules/core-modules/traditional-support` both emit via `let z = region.z()`.
`tree-support-planner` reads `layer_plan.layers[layer_rev].z` and takes heights from
`effective_layer_height`. `LayerPlanView` is the single Z authority, and no module has any
concept of a support-specific layer height.

**F9 — `LayerCollectionIR` blast radius.** Five production literal sites across four files:
`crates/slicer-runtime/src/layer_executor.rs` (2), `crates/slicer-sdk/src/traits.rs` (1),
`crates/slicer-macros/src/lib.rs` (1), `crates/slicer-wasm-host/src/dispatch.rs` (1).

### Net effect

Even with F2/F3/F4 fixed, slicing `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`
yields zero anchored entities, so 239's human-gate artifacts would be structurally identical to
today's output. That is vacuous evidence (E1) and unfixable inside 239, whose `design.md` puts
the module surfaces out of bounds. Hence three packets, not one.

## Canonical OrcaSlicer reference

Delegated dispatch, 2026-08-28. Cited by file + function only, never line number.

- **`GCode::collect_layers_to_print` (`GCode.cpp`) — the canonical row-merge rule.** Walks
  object layers and support layers with two independent indices, takes
  `print_z_min = min(object_layer->print_z, support_layer->print_z)`, and un-consumes whichever
  side sits more than `EPSILON` above it. Object and support merge into one row iff
  `|dz| <= EPSILON`; otherwise the **lower** one emits a solo row and the other retries. This
  answers 239's open tie-break question with canonical behaviour rather than a guess.
- **`generate_support_layers` (`Support/SupportCommon.cpp`)** does *not* reference
  `independent_support_layer_height` at all. It groups layers by
  `print_z <= first.print_z + EPSILON`, sets each group's Z to the midpoint
  `zavg = 0.5 * (first.print_z + last.print_z)`, and its height to the group minimum.
- **The flag's real effect is upstream**, in `PrintObjectSupportMaterial::bottom_contact_layer`
  (`Support/SupportMaterial.cpp`): when ENABLED, `print_z` is free-floating from the interface
  flow height; when DISABLED it calls `sync_gap_with_object_layer` and copies the upper layer's
  `print_z`/`height`. `Slicing.cpp` rounds the gap values to multiples of the object
  `layer_height` only when the flag is FALSE.
- **`_extrude` (`GCode.cpp`)** never recomputes geometry — it reads the precomputed
  `path.mm3_per_mm`. `Flow::mm3_per_mm` (`Flow.cpp`) is
  `m_height * (m_width - m_height * (1 - PI/4))`, or `w^2 * PI/4` when bridging. The height term
  is baked per-extrusion-entity at generation time; supports use
  `support_material_flow(object, layer_height)` with **the support layer's own height**. That is
  precisely what makes independent support layer heights produce a different E per mm.
- **`independent_support_layer_height`** in `PrintConfig.cpp`'s `init_fff_params` is `coBool`,
  default **true**.

## Coordinate discipline

1 unit = 100 nm = 10⁻⁴ mm, **not** 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100.
Use `Point2::from_mm` / `mm_to_units()` at every mm↔unit boundary. Existing anchored fixtures
use unit-scale planes (`z: 3000` = 0.3 mm).

## Packet Queue

Status vocabulary: `pending` (not generated), `generated` (packet files written and
`PREFLIGHT PASS`), `blocked` (unresolved `[BLOCK]` or gate failure after two fix rounds),
`superseded` (absorbed or dropped), `closed` (packet implemented and its acceptance ceremony
green). A packet's own closure step sets its row to `closed` and fills the `packet dir` column;
until then a row reaching `generated` means authored and preflight-clean, not implemented.

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | `239a-anchored-host-seams` | Give the host an anchored-entity input seam, switch all three non-anchored `execute_per_layer*` call sites to the committed anchored variant, and lower `CommittedLayerEvent::Anchored` into off-grid print rows using the canonical `\|dz\| <= EPSILON` merge rule. | TASK-399..TASK-408 | - | closed | `docs/spec_packets/239a-anchored-host-seams/` |
| 2 | `239b-anchored-wit-contract` | Wire the orphaned `ir-types.wit` anchored records into a real world/interface with host lift/lower glue and SDK drain glue, so a guest module can round-trip an `ordered-event-collection`. | TASK-508..TASK-514 | - | closed | `docs/spec_packets/239b-anchored-wit-contract/` |
| 3 | `239c-support-layer-height-producer` | Declare `independent_support_layer_height`, decouple support Z from `LayerPlanView`, emit off-grid support rows from the support modules, and settle the measure-first `height_delta` verdict. | TASK-515..TASK-522 | #1, #2 | generated | - |

### Seam decision (approved 2026-08-28)

Packets #2 and #3 both stalled on one shared question: a `Layer::Support` guest could not reach
`set-anchored-event-collection`, because a module manifest binds exactly one `stage.id`
(`required_stage`, `crates/slicer-scheduler/src/manifest.rs`), `layer-support.wit`'s `run`
receives only a `support-output-builder`, and exactly one world in the whole WIT tree receives a
`layer-collection-builder` — `layer-path-optimization.wit`.

**Resolution:** `layer-support.wit`'s `run` gains a second parameter,
`collection: layer-collection-builder`, mirroring `layer-path-optimization.wit`. Additive to one
WIT file, follows an existing in-tree precedent, and keeps anchored transport generic per
ADR-0059's "each worker returns ordered event collections". Rejected: moving the drain onto
`support-output-builder` (confines anchored events to support stages, narrowing the generic
substrate packets 219–223 built), and a dedicated anchored-events module (one-stage-per-module
makes this a whole sibling module). Packet #2 owns the widening and its ~15-file break surface;
packet #3 consumes the two-builder signature.

Dependency note: #1 and #2 are mutually independent and both implementable today. #3 requires
both — #1 for host emission of off-grid rows, #2 for guest-side transmission of anchored work.
#1 is the direct heir of 239's acceptance criteria and inherits its reserved `TASK-399..408`
range; #2 and #3 mint fresh above the `docs/07_implementation_status.md` high-water mark
(`TASK-507` at split time — re-derive before registering).

## Disposition of packet 239

`docs/spec_packets/239-support-independent-layer-z/` moves to `status: superseded`, naming all
three successors. Its `design.md` already carries correction entry PC-1 (recording F1) and the
`visual_debug.rs` scope note, both added during the swarm run; that history is preserved, not
deleted. Each successor carries `supersedes: 239-support-independent-layer-z`.

Gap register: `docs/specs/support-parity-gap-register.md` row **G-02** currently names
destination `239-support-independent-layer-z` and must be re-pointed across the three
successors. A new row **G-27** should record the newly discovered defect — the anchored-event
substrate is production-dead (F5, F6, F7) — which was not previously registered anywhere.
Re-derive both the next free `G-` row and the next free `DEV-` id at edit time; they were
`G-27` and `DEV-157` at split time and are mutable shared state.

## Commit note

This plan file and the three generated packet directories should be committed together.
