# The `Layer::Infill` first-dispatch block and base classic-on's 1.096 s residual

Type: task / Phase-1 attribution + probe
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Benchy per-layer program: which carrier holds the wall, and
  what does canonical do differently?](154-benchy-per-layer-carrier-attribution.md)
  — t154's arithmetic leaves **base classic-on at 1.051x** at canonical prepass
  cost, needing **1.096 s** (9.6 % of its 11.46 s per-layer phase), and t154's
  carrier table prices the first `Layer::Infill` dispatch at **30,089 ms**
  thread on that cell (23.5 % of the phase; **2.69 s** wall-equivalent at
  11.18x). t127 proved by declining `raft-default` that the first Infill
  dispatch pays a **stage-shared** block and that the cost transfers to
  whichever Infill module runs first, but named the block "the shared
  prepared-region view materialisation" — a name t154 showed cannot hold at
  HEAD without an arena invalidation, because `Layer::Perimeters` materialises
  that view first through `ensure_prepared_perimeter_source_regions`
  (`crates/slicer-runtime/src/blackboard.rs`) and `Layer::Infill` hits the memo.

## Question

What is the ~63 ms per-layer block that the first `Layer::Infill` dispatch pays
on the base cells, and can removing or shrinking it supply base classic-on's
**1.096 s** — or, failing that, is there a second, larger block in the same
family (`Layer::Support`'s tree-support bracket pays p50 **94 ms** on that cell,
but its own render work is non-zero, so that bracket is contaminated and must be
separated before it is priced)?

The measured shape to explain (t154, base classic-off):

| module | calls | p50 | p90 | thread sum |
| --- | ---: | ---: | ---: | ---: |
| `com.core.classic-perimeters` | 496 | 91 ms | 128 ms | 44,206 ms |
| `com.core.raft-default` (first Infill dispatch) | 496 | **63 ms** | 86 ms | **31,218 ms** |
| `com.core.gyroid-infill` (never first here) | 496 | 1 ms | 2 ms | 593 ms |
| `com.core.infill-linker` | 496 | 9 ms | 17 ms | 4,482 ms |

`raft-default`'s own work is nil (no raft band resolves on the matched job), so
its 63 ms bracket is almost entirely work the host does around the dispatch.
On benchy classic-off the same bracket is p50 5 ms; on base classic-on 61 ms;
on both benchy supports-on cells 4–5 ms.

## Scope

1. **Re-derive the block at HEAD** on base classic-off and base classic-on, and
   confirm it is stage-shared by declining the first Infill dispatch and
   watching the bracket move to the next module (t127's method, 240/495-call
   shape per arm). An `--instrument-stderr` pass is attribution only.
2. **Identify it by probe.** Add temporary host timing around the candidate
   sites and run one base classic-on pass per arm:
   - `prepare_dispatch_region_views`' memo hit
     (`crates/slicer-runtime/src/layer_executor.rs`);
   - the `LayerStageInput` construction, including the `seam_plan()` /
     `support_plan()` / `region_map()` clones;
   - the runner's first-dispatch marshalling and instance-pool acquisition;
   - the arena's prepared-region slots, to prove they are not rebuilt between
     the perimeters and infill stages.
   The probe is temporary and must be reverted; preserve the patch and the
   build identity under `evidence/`, and disclose every probe build.
3. **Separate the `Layer::Support` block** if the Infill block turns out to be
   necessary work: tree-support's p50 is 94 ms on base classic-on, but its
   in-module render is not nil, so split the bracket with the same method before
   pricing it.
4. **Price what is found** against base classic-on's 1.096 s requirement, the
   0.80 % floor, and base classic-off's 24.51 s cell (where the block's ceiling
   is 2.86 s = 14.6x the floor). State the ceiling as thread sum divided by the
   cell's measured effective parallelism, never as a price.
5. **Stop rule.** If the block is the module's necessary input marshalling, or
   removing it is not representation-safe, record measured-dead, name the
   structural blocker, and make no candidate ticket. If it is removable, the
   verdict is a proposed candidate with its ceiling — **not** an adopted change;
   the standing four-cell gate is a separate ticket.

## Acceptance

- The block's identity is either established by direct measurement or
  falsified for every named candidate; "unmeasured" is not an acceptable answer
  for a site that was probed.
- The stage-sharing claim is re-proven on the current tree, not carried from
  t127.
- The probe patch is preserved in `evidence/`, the tree is restored to HEAD, and
  `cargo xtask build-guests --check` exits 0 after restoration.
- `evidence/t155-infill-stage-block/` holds `FINDINGS.md`, `REPRODUCE.md`, the
  probe patch, the reduced tables and `verify.py`; `verify.py` exits 0
  checkout-only and `--self-test` rejects its negative controls.
- The map's destination status and fog are updated and the take is recorded in
  **Decisions so far**.
- No production edit survives and no G-code reference moves.

## Evidence starts

- [`evidence/t154-benchy-per-layer/`](../evidence/t154-benchy-per-layer/FINDINGS.md)
  — the carrier table, the arithmetic and the code-level reason the t127 name
  cannot hold.
- [`evidence/t127-raft-band-dispatch/`](../evidence/t127-raft-band-dispatch/FINDINGS.md)
  — the decline-and-watch transfer test and its candidate patch.
- `crates/slicer-runtime/src/layer_executor.rs` — `prepare_dispatch_region_views`
  and the `LayerStageInput` construction.
- `crates/slicer-runtime/src/blackboard.rs` — `LayerArena::ensure_prepared_regions`
  and `ensure_prepared_perimeter_source_regions`.
- `crates/slicer-wasm-host/src/marshal/prepared.rs` — `prepare_regions` /
  `prepare_slice_regions`.
