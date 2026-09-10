# 95 — Author packet P88 — Quality / Precision — new: contour-compensation

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-10)
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P88 — Quality / Precision — new: contour-compensation** — 2 keys, Tier C new module, owner new module contour-compensation. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P88 — Quality / Precision — new: contour-compensation):

`xy_contour_compensation`, `xy_hole_compensation`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier C held as a packet, but the owner is corrected: host prepass built-in, not a
module — and the ordering obligation ticket 93 handed this ticket dissolves rather than
being honoured.**

Both keys are live in canonical `PrintObject::_shrink_contour_holes`, applied from the
compensation block inside `PrintObject::slice_volumes` (`PrintObjectSlice.cpp`), and both
are `PrintObjectConfig` members (`PrintConfig.hpp`) — per **object**, not per region. Both
are `coFloat` default `0` with **neither a `min` nor a `max`**, so no range is borrowed
(ticket 113). Rule 3 passes. Canonical's slicing-time `extra_offset` path is dead —
`slice_volumes` hardcodes it to `0.f` with the `xy_contour_compensation` line commented
out — so the entire behaviour lives in that one post-slice block. In-tree the keys are
zero-occurrence in code apart from the two `ORCA_CONFIG_PADDING` twins (rule 2
non-evidence).

### The tier table's owner cannot work — three independently sufficient reasons

Ticket 04 assigns both keys to a `new contour-compensation module`, and ticket 93 recorded
an ordering obligation on the assumption this would land on `Layer::SlicePostProcess`
beside packet 303's elephant-foot module, running first. Re-derived at claim time per
ticket 27, that owner fails:

1. **Every prepass consumer of the slice footprint would see uncompensated geometry.**
   `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) runs
   `PrePass::OverhangAnnotation`, `PrePass::ShellClassification`, `PrePass::SupportAnalysis`,
   `PrePass::SupportGeometry` and `PrePass::LightningTreeGen` **before**
   `Layer::SlicePostProcess`, and all five read the committed `SliceIR` (verified at each
   producer's `blackboard.slice_ir()` call). Canonical compensates inside `slice_volumes`,
   before `detect_surfaces_type` and before support generation. A module at the
   post-process seam would leave overhang bands, shell classification, support analysis,
   support geometry and lightning trees all computed against the uncompensated outline — on
   **every** layer, at a user-chosen magnitude. Strictly worse than the same concern for
   elephant-foot, which is bounded to the first `elefant_foot_compensation_layers` layers
   and to a width-limited shrink.
2. **A second coarse `SliceIR` mutator on that stage deadlocks the scheduler.** The
   per-stage DAG (`crates/slicer-scheduler/src/dag.rs`) emits an `EdgeReason::IrWriteRead`
   edge whenever `reader.ir_reads()` **exactly** contains the writer's write path
   (`Vec<String>::contains`, not root matching). Packet 303's manifest is
   `reads = ["SliceIR"]`, `writes = ["SliceIR"]`; an honest contour-compensation manifest is
   the same, so the pair produces edges in **both** directions and `validate_cycles` →
   `topological_sort` (`crates/slicer-scheduler/src/validation.rs`) fails with
   `SchedulerError::CyclicDependency`. **No pair of modules in the tree does this today** —
   `seam-placer` escapes it beside `fuzzy-skin` on `Layer::PerimetersPostProcess` only by
   declaring narrow writes (`PerimeterIR.resolved-seam`, `PerimeterIR.regions.walls`)
   against `fuzzy-skin`'s coarse `PerimeterIR` read, and `top-surface-ironing` teaches the
   same lesson from the other side by ordering with `[compatibility].requires` instead of a
   fake IR read. Escaping the cycle would mean rewriting packet 303's manifest, which a
   packet must not do.
3. **Canonical's positive-growth branch is per object, not per region.** On a multi-region
   layer it merges the whole object's layer, compensates the merged expolygons once, then
   re-derives each region as `intersection(offset(region, max_growth), merged)` and trims it
   by the union of the regions already processed, giving priority to the lower region index.
   A host prepass built-in holds the whole `Vec<SliceIR>` and groups by `object_id`; a
   per-layer module stage does not.

Corrected to `host:xy_size_compensation` on a new **host-only** stage
`PrePass::XySizeCompensation` between `PrePass::Slice` and `PrePass::OverhangAnnotation`,
on packet 297's staging shape (kernel in `slicer-core/algos`, producer in
`slicer-runtime/builtins`, one `run_builtin_stage` registration) and packet 304's host-only
stage-registration shape (`STAGE_ORDER` + `HOST_ONLY_STAGES`, never `VALID_STAGES`).

### The ticket-93 ordering obligation dissolves; the packet is order-robust

Canonical's order is conical-overhang → XY compensation → elephant-foot → polyhole. Because
every `PrePass::` stage precedes every `Layer::` stage, a prepass built-in runs before
packet 303's elephant-foot **whichever way the open seam question is settled** — and before
packet 304's `PrePass::PolyholeTransform` when registered ahead of it. So this packet takes
on no dependency on 303 and amends nothing. One correction to the map's fog note while I
was there: canonical calls `apply_conical_overhang()` **before** the compensation block
inside `slice_volumes`, not after it — consistent with packet 297 sitting in the prepass, so
nothing turns on it.

### Sizing and scope

Packet, not direct implementation: a new geometry kernel, a new stage, a new built-in and
two `ResolvedConfig` fields. Rule 4 does not fire (scalar parameters of one pass, not
competing algorithms); `[claims]` empty, no `Producer` minted. Zero declaration-only keys,
no supporting non-queue key, **queue count unchanged at 409**.

Four things are deliberately left out and ride the packet's one deviation row: canonical's
two painted-object suppressions and their `active_step_add_warning` warnings (the
mm-painted arm needs an extruder count ticket 118 showed this tree cannot resolve at
prepass; the fuzzy-skin predicate spans two channels depending on `[[region_split]]`
declarations); canonical's `PrintApply` region-assignment bbox growth by
`xy_contour_compensation`, which has no counterpart because regions are assigned at
`PrePass::RegionMapping` before slicing; `MODIFIER_FOOTPRINT_REGION_ID` regions passed
through uncompensated; and serial-where-canonical-is-parallel.

Packet [`docs/spec_packets/305-xy-size-compensation-slice-prepass/`](../../../spec_packets/305-xy-size-compensation-slice-prepass/)
authored (`draft`), **preflight PASS** (S0-S8, tree-verified). No WIT/IR/schema change, no
guest WASM, ungated kernel beside `bridge_over_infill`.

### The preflight sweep caught two real defects, one of them blocking

- **AC-10's test binary did not exist.** `cargo test -p slicer-scheduler --test contract`
  fails hard with `error: no test target named 'contract'` — `slicer-scheduler` prefixes its
  buckets (`scheduler_contract`, `scheduler_unit`, `scheduler_integration`), and CLAUDE.md's
  `unit|contract|executor|integration|e2e` list describes `slicer-runtime`'s naming, which I
  over-generalised. Fixed in all three places and re-run: `test result: ok. 2 passed`.
- **Twelve AC commands could report a false green on zero tests.** They ended in
  `| tail -5`, whose exit status is always `0` and whose visible output on a zero-match
  filter is `test result: ok. 0 passed`. Every `cargo test` command in the packet now ends
  in `| rg 'test result: ok\. [1-9]'; echo "exit=$?"`. Verified both directions: the guard
  exits `0` on a real run and `1` on a deliberately misspelled filter.
- Three smaller fixes. **`DEV-065` is cited across the tree but was removed from
  `docs/DEVIATION_LOG.md`** (commit `16f10e60`, "remove closed entries, fix dangling
  cross-references"), so the packet cites the manifest comment and doc 04 rather than the
  retired ID — `docs/01_system_architecture.md`, `docs/03_wit_and_manifest.md` and
  `docs/04_host_scheduler.md` carry the same dead pointer, which is a pre-existing tree-wide
  issue this ticket did not create and did not fix. "The only occurrence anywhere" was
  overstated (true of code, false of docs). Two citations lacked crate-qualified paths, one
  pointing at the wrong crate for `builtin_producers_tdd.rs`.

### Follow-on

Reason 2 is a **new, mechanical** consequence for the open elephant-foot seam question, and
a sharper one than the ordering inversion ticket 94 recorded: `Layer::SlicePostProcess` can
host **at most one** coarse `SliceIR` mutator, so it is not extensible for this class of
pass at all. That graduated the map's fog patch into
[148 — Rule the home of the slice-mutating passes](./148-rule-slice-mutating-pass-home.md).

No code change in this ticket. 04 and 05 row annotations are the implementer's Step 7.
