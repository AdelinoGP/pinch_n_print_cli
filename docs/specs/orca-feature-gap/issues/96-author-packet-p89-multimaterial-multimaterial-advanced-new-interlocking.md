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

**Tier C held as a packet; the queue's 3+3 split dissolved into one six-key packet; ticket
04's `new interlocking module` owner HELD, on `Layer::SlicePostProcess`, backed by a host
analysis prepass. A PnP-invented key name is retired, and a finding about the gap inventory
falls out sideways.**

Packet: [`docs/spec_packets/306-interlocking-beams-slice-postprocess/`](../../../spec_packets/306-interlocking-beams-slice-postprocess/)
— authored, `status: draft`, preflight PASS.

### The 3+3 split is not implementable, so ticket 97 is dissolved

Canonical's enabling gate is one condition reading four keys —
`!interlocking_beam || interlocking_beam_layer_count < 1 || interlocking_depth < 1 ||
interlocking_beam_width < EPSILON` (`InterlockingGenerator::generate_interlocking_structure`)
— and `interlocking_depth` is a **P90** key, so a P89-only packet cannot open its own gate
without hardcoding one. `interlocking_orientation` is applied before the voxel walk and
unapplied on every output, threading through every function such a packet would author;
`interlocking_boundary_avoidance` selects the whole `air_filtering` branch. Ticket 97 is
closed as dissolved on the ticket-31 (P24) precedent.

### The seam: a guest module on `Layer::SlicePostProcess`, plus a host analysis prepass

Ticket 04's owner is **held**. The microstructure and the outline rewrite live in a new guest
module `interlocking-beams` on `Layer::SlicePostProcess`:

- **The stage's commit type is a precise fit.** `LayerStageCommit::SlicePostProcess {
  polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>, path_z_updates }`
  (`crates/slicer-ir/src/stage_io.rs`, merged in `crates/slicer-runtime/src/layer_executor.rs`)
  replaces a named region's polygons — precisely canonical's `slices.set(...)` in both
  `applyMicrostructureToOutlines` and `handleThinAreas`. The `RegionKey` carries
  `variant_chain`, which is how the two interlocked material variants are told apart.
- **Rule 4 points here.** The beam pattern is the swappable part of the algorithm; in a module
  a community fork can replace it, in host code it is frozen.
- **The stage is module-targetable and empty**, and ticket 148 asks what it is for. This
  supplies a second occupant and a worked answer.

Only the **analysis** is host-side, on a new host-only stage `PrePass::InterlockingLattice`
committing a new `InterlockingLatticeIR` — the cell set, not the beam polygons, so the pattern
stays in the module. The split runs along canonical's own function boundary:
`getShellVoxels` / `addBoundaryCells` need every layer (`skin = xor_ex(layers[n], layers[n-1])`);
`generateMicrostructure`, `applyMicrostructureToOutlines`, `growBorderAreasPerpendicular` and
`handleThinAreas` need only the layer's own polygons plus the cell set. **This is an
established idiom** — `LightningTreeIR` is a global prepass product consumed by the per-layer
guest `lightning-infill` through `LayerStageInput`; `SeamPlanIR`, `SupportPlanIR` and
`SurfaceClassificationIR` are the same shape.

The module declares **narrow** `writes = ["SliceIR.regions.polygons"]` on the `seam-placer` /
`part-cooling` precedent, so it is orderable against packet 303's coarse `elefant-foot`
without amending 303. AC-N6 asserts the pair validates; that AC is the packet's one genuine
risk, and its dispatch is written to stop and report a block on ticket 148 rather than widen
the writes.

**Cost stated, not hidden:** `InterlockingLatticeIR` is a new IR crossing the WIT boundary —
schema constant, WIT record, `LayerStageInput` field, guest rebuild. A host-only design would
need none of it. The packet pays it to keep the swappable half in a module.

### `interlocking_beam` is already in this tree under a name canonical does not have

`crates/slicer-ir/src/resolved_config.rs` declares **`mmu_segmented_region_interlocking_beam`**.
Canonical has no such key: `PrintConfig.{cpp,hpp}` declare plain `interlocking_beam` as a
`PrintObjectConfig` member, while `mmu_segmented_region_interlocking_depth` and
`mmu_segmented_region_max_width` genuinely do carry that prefix. The PnP spelling is an
invention that landed with the Phase-5 work (`b18c00b3`, 2026-06-13). Under it, the key's
**second** canonical read site is already correct — `multi_material_segmentation_by_painting`
suppresses `cut_segmented_layers` when the flag is set, and `run_phase5_width_limit`
(`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) does exactly that. What is missing
is the **first** read site. The packet renames the key (nine sites, three files, no alias —
ticket 07's standardise-to-Orca ruling) and adds the missing read site. The other five keys
have **zero** occurrences anywhere in `crates/` or `modules/`, including
`ORCA_CONFIG_PADDING`, which carries no twin for any of them.

### Ticket 01's asset cannot see a host-side `ResolvedConfig` key — filed as ticket 149

Ticket 01's own reproduction scrapes live keys from module `[config.schema]` manifests and
`docs/config/host-keys.toml` only. **`ResolvedConfig`'s `cli "..."` declarations are not a
source.** Measured: **30 of the 73 `cli`/`cli_opt` keys in
`crates/slicer-ir/src/resolved_config.rs` are marked `live=no` in the asset.** That 30 is an
upper bound mixing original false negatives with post-2026-08-07 rot, and separating them
needs per-key archaeology — but at least three are provably original, because they landed
2026-06-13, two months before the asset: `mmu_segmented_region_max_width`,
`mmu_segmented_region_interlocking_depth` and the beam bool. **The first two are ticket 98's
entire key list (P91), and both are already live** driving `run_phase5_width_limit` with e2e
coverage in `cube_4color_phase5_tdd.rs`. Ticket 98 must confirm that before authoring
anything.

### Authoring correction — the first revision of this packet was wrong, and how

The first revision sited the **whole** pass as a host prepass built-in, leading with "a guest
module cannot write slices at all". That is a fact about `PrepassStageOutput`
(`crates/slicer-core/src/stage_io.rs`), which has no `SliceIR` variant — true of the
**prepass** seam only, and silent about `Layer::SlicePostProcess`, which has `polygon_updates`
for exactly this. A prepass-specific limitation was generalised to "guest modules" and then
carried the entire owner correction. The second reason, "the algorithm is whole-object, not
per-layer", conflated the global **analysis** with the per-layer **application**; only the
former is global. The user caught both.

**Standing lesson: before correcting a tier-table owner away from a module, name the specific
stage and check that stage's commit type — never reason from another stage's limitation.**
Tickets 94 and 95 rest on different arguments (a layer-major executor; prepass consumers
reading an uncompensated footprint) and are not disturbed. But three consecutive
module-to-host owner corrections is itself worth watching: the modular pipeline is the
project's stated point, and it would be hollowed out one defensible-looking packet at a time.

### Also recorded, not acted on

`STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) lists
`PrePass::PaintSegmentation` **fourth**, ahead of `PrePass::RegionMapping` and
`PrePass::Slice`; `run_prepass` runs it **seventh**, after `PrePass::ShellClassification`.
`STAGE_ORDER` governs guest prepass-module dispatch and visual-debug tap ordering; host
built-ins execute in `run_prepass`'s hardcoded call order. The packet places its prepass stage
where both agree and does not reopen the discrepancy — it is fog, entangled with ticket 148.

Preflight caught four defects in the first revision — three wrong crate-of-origin citations
(`PrepassStageOutput` is in `slicer-core`, `PrepassStageInput` in `slicer-wasm-host`, and
`topological_sort` in `crates/slicer-scheduler/src/topology.rs` **not** `validation.rs`; that
last was inherited by copying packet 305's wording, **so 305 carries the same wrong pin**) and
one `≤3 files per step` violation — and one more in the re-authored revision: the module
manifest originally declared `reads = [..., "InterlockingLatticeIR"]`, but `validate_ir_reads`
(`crates/slicer-scheduler/src/validation.rs`) resolves every declared read against a writer at
an **earlier stage**, and a host built-in mints no module node, so that read would have been
unsatisfiable. `lightning-infill` shows the correct shape: it consumes `LightningTreeIR`
through `LayerStageInput` while declaring only `reads = ["SliceIR"]`. No code change.
