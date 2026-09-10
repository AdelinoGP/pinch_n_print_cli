# 96 — Author packet P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-10)
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking** — 3 keys, Tier C new module, owner new module interlocking. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking):

`interlocking_beam`, `interlocking_beam_layer_count`, `interlocking_beam_width`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Authors the interlocking module's ADR** (algorithm port: port-strategy + seam + data-flow decisions; number re-derived from disk at authoring time).

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier C held as a packet, but the queue's 3+3 split is dissolved into one six-key packet,
and the owner is corrected from a guest module to a host prepass built-in. A fourth
finding falls out of the evidence: ticket 01's asset has a class of false negative nobody
has measured.**

Packet: [`docs/spec_packets/306-interlocking-beams-slice-prepass/`](../../../spec_packets/306-interlocking-beams-slice-prepass/)
— authored, `status: draft`, `/spec-review --preflight` **PASS** (0 blockers, 0 high).

### The 3+3 split is not implementable, so ticket 97 is dissolved into this packet

Canonical's enabling gate is one condition reading four keys —
`!interlocking_beam || interlocking_beam_layer_count < 1 || interlocking_depth < 1 ||
interlocking_beam_width < EPSILON` (`InterlockingGenerator::generate_interlocking_structure`)
— and `interlocking_depth` is a **P90** key. A P89-only packet cannot open its own gate
without hardcoding it. The other two P90 keys are no more separable:
`interlocking_orientation` is applied before the voxel walk and unapplied on every output,
so it threads through `getShellVoxels`, `computeUnionedVolumeRegions`, `handleThinAreas`
and `applyMicrostructureToOutlines`; `interlocking_boundary_avoidance` selects the
`air_filtering` branch that owns the air dilation **and** the whole of `handleThinAreas` /
`growBorderAreasPerpendicular`. Splitting buys nothing because the work is the voxel
generator (946 lines of C++ across `InterlockingGenerator.{cpp,hpp}` and
`VoxelUtils.{cpp,hpp}`) and the generator needs all six. Ticket 97 is closed as dissolved
on the ticket-31 (P24) precedent; the packet's `task-map.md` records the absorption.

### The tier table's owner cannot work — three independently sufficient reasons

1. **A guest module cannot write slices at all.** `PrepassStageOutput`
   (`crates/slicer-core/src/stage_io.rs`, re-exported by `crates/slicer-runtime/src/prepass.rs`)
   has exactly the variants `None`, `SurfaceClassification`, `LayerPlan`, `SeamPlan`,
   `SupportPlan`, `RegionMap`, `SupportGeometry` — **no `SliceIR` variant**. A prepass
   module reads `slice_ir` off `PrepassStageInput` (`crates/slicer-wasm-host/src/binding.rs`)
   but has no channel to return a mutated one, and interlocking's entire output is rewritten
   `SlicedRegion.polygons`. This is also why `PrePass::PaintSegmentation` — the one
   module-targetable prepass stage in this neighbourhood — is itself a host built-in.
2. **The algorithm is whole-object, not per-layer.** The voxel cell is
   `(2·beam_width, 2·beam_width, 2·beam_layer_count)`, so one cell spans
   `2·beam_layer_count` layers in Z; `computeUnionedVolumeRegions` allocates a ghost layer
   above the top for the topmost skin, and `handleThinAreas` builds one
   `near_interlock_per_layer` vector across the whole object before touching any layer.
3. **A second coarse `SliceIR` mutator on a shared stage cycles the scheduler** —
   `EdgeReason::IrWriteRead` (`crates/slicer-scheduler/src/dag.rs`) in both directions,
   `validate_cycles` (`crates/slicer-scheduler/src/validation.rs`) →
   `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) fails with
   `SchedulerError::CyclicDependency`. Same finding ticket 95 recorded.

Corrected to `host:interlocking_beams` on a new host-only stage `PrePass::InterlockingBeams`,
registered immediately after the `PrePass::PaintSegmentation` built-in — the earliest point
at which the material-split regions the generator pairs over exist. **This does not pre-empt
ticket 148**: interlocking's seam is not a free choice, so it adds a fifth data point on the
same side as tickets 94/95 rather than a competing ruling.

### `interlocking_beam` is already in this tree under a name canonical does not have

`crates/slicer-ir/src/resolved_config.rs` declares **`mmu_segmented_region_interlocking_beam`**.
Canonical has no such key: `PrintConfig.cpp` / `PrintConfig.hpp` declare plain
`interlocking_beam` as a `PrintObjectConfig` member, while
`mmu_segmented_region_interlocking_depth` and `mmu_segmented_region_max_width` genuinely do
carry that prefix. The PnP spelling is an invention that landed with the Phase-5 work
(`b18c00b3`, 2026-06-13). Under it, the key's **second** canonical read site is already
correct — `multi_material_segmentation_by_painting` suppresses `cut_segmented_layers` when
the flag is set, and `run_phase5_width_limit`
(`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) does exactly that. What is
missing is the **first** read site, the generator gate. The packet renames the key (nine
sites, three files, no alias — ticket 07's standardise-to-Orca ruling) and adds the missing
read site. The other five keys have **zero** occurrences anywhere in `crates/` or
`modules/`, including `ORCA_CONFIG_PADDING`, which carries no twin for any of them.

### Ticket 01's asset cannot see a host-side `ResolvedConfig` key — filed as ticket 149

Ticket 01's own reproduction scrapes live keys from module `[config.schema]` manifests and
`docs/config/host-keys.toml` only. **`ResolvedConfig`'s `cli "..."` declarations are not a
source.** So a key implemented purely as a host-side `ResolvedConfig` field reads `live=no`
even when it drives a real decision point. Measured: **30 of the 73 `cli`/`cli_opt` keys in
`crates/slicer-ir/src/resolved_config.rs` are marked `live=no` in the asset.** That 30 is an
upper bound mixing original false negatives with post-2026-08-07 rot, and separating them
needs per-key archaeology — but at least three are provably original, because they landed
2026-06-13, two months before the asset was generated:
`mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth` and the beam
bool. **The first two are ticket 98's entire key list (P91), and both are already live**
driving `run_phase5_width_limit` with e2e coverage in `cube_4color_phase5_tdd.rs`. Ticket 98
must confirm that before authoring anything.

### Also recorded, not acted on

`STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) lists
`PrePass::PaintSegmentation` **fourth**, ahead of `PrePass::RegionMapping` and
`PrePass::Slice`; `run_prepass` (`crates/slicer-runtime/src/prepass.rs`) actually runs it
**seventh**, after `PrePass::ShellClassification`. `STAGE_ORDER` governs guest prepass-module
dispatch and visual-debug tap ordering; host built-ins execute in `run_prepass`'s hardcoded
order. The packet places its stage where both agree and does not reopen the discrepancy.

Preflight caught four authoring defects, all fixed before PASS: three wrong crate-of-origin
citations (`PrepassStageOutput`, `PrepassStageInput`, `topological_sort` — the last inherited
by copying packet 305's wording) and one edit-cap violation that split Steps 5–6 into 5–10.
