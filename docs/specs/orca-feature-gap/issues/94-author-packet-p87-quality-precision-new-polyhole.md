# 94 — Author packet P87 — Quality / Precision — new: polyhole

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P87 — Quality / Precision — new: polyhole** — 3 keys, Tier C new module, owner new module polyhole. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P87 — Quality / Precision — new: polyhole):

`hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier C held as a packet, but the owner is WRONG and is corrected: this is a host prepass
built-in, not a module.**

**Rule 3 (dead-in-canonical) — all three queue keys pass, and so does a fourth.** Every one is
read by canonical `PrintObject::_transform_hole_to_polyholes` (`PrintObject.cpp`), reached from
`PrintObject::slice` (`PrintObjectSlice.cpp`); the pass came into OrcaSlicer from SuperSlicer.
Config is read **per region** (`region().config()`). Zero occurrences in this tree — `polyhole`
matches nothing under `crates/`, `modules/` or `xtask/`, not even an `ORCA_CONFIG_PADDING` twin,
which is unusual for this queue.

**Owner corrected under ticket 27's re-derive rule: `new polyhole module` (Tier C) →
host prepass built-in `host:polyhole` on a new host-only stage `PrePass::PolyholeTransform`,
between `PrePass::Slice` and `PrePass::OverhangAnnotation`.** The tier table's owner cannot work:
the pass is **cross-layer**. Canonical accepts a hole only if a matching hole exists on a
contiguous neighbouring layer (its grouping step), and indexes the twist rotation by the
**absolute** layer index. This tree's layer executor is **layer-major** — `execute_per_layer*`
iterates layers in the outer loop and `plan.per_layer_stages` in the inner
(`crates/slicer-runtime/src/layer_executor.rs`) — so a module on `Layer::SlicePostProcess` can
never see the neighbour it needs, and there is no barrier between per-layer stages where a host
pass could stand in. This is the same reasoning packet 297 used to reject the module seam for its
own cross-layer pass. Rule 4 does **not** fire: the keys are scalar parameters of one geometric
pass, not an enum selecting between competing algorithms, so no claim is minted.

**The seam has a working precedent.** `commit_shell_classification_builtin`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs` — the file name is misleading, it is the
`PrePass::ShellClassification` built-in) already clones the committed `Vec<SliceIR>`, walks
per-`(object_id, region_id)` timelines via `build_region_timelines`, mutates across layers, and
writes back through `replace_slice_ir`. And **canonical's per-region gate is reproducible here, not
flattened**: `RegionMapIR::config_for(&RegionKey { .. })` resolves each region's own interned
`ResolvedConfig`, so two regions on one layer can disagree exactly as they do in canonical.

**A fourth key rides along and the queue count does not move.** `hole_to_polyhole_max_edges`
(coInt, min 3, default 50) is read by the same canonical function but is **absent from
`docs/ORCA_CONFIG_REFERENCE.md`** — already flagged as an inventory gap by tickets 04 and 05, so
it is in no packet. It is declared here as a supporting non-queue key because the alternative is a
hardcoded `50` in the kernel, which rule 4 forbids. **Queue count unchanged: 409.** Same
disposition packet 303 used for its five re-declared non-queue keys. This is a second confirmed
instance for ticket 123 alongside 93's `elefant_foot_layers_density`.

**Packet `docs/spec_packets/304-polyhole-slice-prepass/` authored (`draft`), preflight PASS**
(S0–S8, tree-verified: 16/16 pre-existing symbols resolved, all net-new paths clear). Seven steps,
aggregate `M`. No WIT change, no IR field, no schema bump, no guest WASM in the change surface,
no claim. Kernel lands **ungated** in `crates/slicer-core/src/algos/polyhole.rs` beside
`bridge_over_infill` — deliberately, since an ungated module and test target remove the
silent-zero-tests trap entirely rather than merely documenting it.

**Blast radius of the new stage, fully enumerated** — and bounded only because the stage is
host-only: `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`), `HOST_ONLY_STAGES`
(`crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — omitting this fails the
partition test), one `required_slots` arm, one `run_builtin_stage` call, one
`PrepassExecutionError` variant, two doc stage lists. **Not** touched: `VALID_STAGES`, `STAGES`,
the `slicer-macros` glue match, `stage_io.rs`'s commit match, `module_new.rs`'s scaffold arm.
Two neighbouring assertions were checked and need no edit: `dag_cli_integration.rs` asserts
*containment* of six built-in stage ids rather than equality, and `builtin_producers_tdd.rs`
asserts a producer count of 7 that holds only because this built-in registers no `Producer` — the
`PrePass::PaintSegmentation` shape. Recorded in the packet so the implementer proves it rather
than discovering it.

**The significant finding is a cross-packet ordering conflict with packet 303, and it is real.**
Canonical's order inside `PrintObject::slice()` is `slice_volumes()` (XY compensation →
elephant-foot → `apply_conical_overhang()`) → `fix_slicing_errors` → `_transform_hole_to_polyholes()`.
Polyhole is **last** of the slice-mutating passes. Packet 303 (ticket 93) places elephant-foot on
`Layer::SlicePostProcess`, which runs after **every** prepass stage — so in this port polyhole
would run **before** elephant-foot, inverting canonical. This is not cosmetic: canonical EFC
iterates `idx_contour <= simplified.holes.size()`, so it resamples and offsets **holes**, and
canonical's own grouping rule carries a lone-first-layer rescue whose stated reason is "cause of
first layer compensation" — the rescue exists *because* EFC already perturbed layer 0's hole.
Inverted, layer 0 groups normally and then gets EFC-resampled as an already-faceted polygon.
Observable only when both features are enabled (both default off) and only on the first
`elefant_foot_compensation_layers` layers. **Neither packet may amend the other**, so packet 304
records it as a deviation clause and the question is filed as fog. The cost of being wrong is
small and stated: if the fog resolves toward a prepass elephant-foot, packet 304 changes the
position of one `run_builtin_stage` call, not its design.

**Six authoring defects were caught by the preflight before this ticket closed**, four of them the
false-green class the gate exists for: AC-11's test filter matched no `module::fn` path and would
have reported **0 tests as green**; AC-10 had the same shape; AC-9's filter did not match the
registered `mod` name; AC-13's two greps were case-sensitive against text that begins with a
capital. Every test-count AC now asserts `test result: ok. [1-9]` rather than an exit status.
Step 6 also gained a generator blast radius: `cargo xtask check-deviations` regenerates doc 07's
Open Deviation Map and doc 15's tables from the new row, so those two files are modified by the
step even though they are never hand-edited.

No code change in this ticket. The 04 and 05 row annotations — including the owner correction —
are the implementer's Step 7.
