# The `Layer::Infill` first-dispatch block and base classic-on's 1.096 s residual

Type: task / Phase-1 attribution + probe
Label: wayfinder:task
Status: resolved (2026-11-14)
Assignee: current OpenCode session (wayfinder), 2026-11-14
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

## Answer

**The block is the prepared-region derivation, paid twice per layer, and the
second payment is redundant.** `prepare_dispatch_region_views` →
`ensure_prepared_regions` / `ensure_prepared_perimeter_source_regions` →
`prepare_slice_regions` → `prepare_regions` accounts for **97.5 %** of the
first-`Layer::Infill` bracket on base classic-off (p50 **52.4 ms** of **53.6
ms**) and **97.3 %** on base classic-on (p50 **53.3 ms** of **54.8 ms**). Inside
it, **94.3–94.4 %** is one `derive_needs_support` call per derivation, and that
call is a single `intersection_ex(region.polygons, footprint)` — the object has
**exactly one** non-empty overhang footprint on every layer (max 1 across
1,488 calls) and the answer was **true in every measured call**.

The named candidates are all falsified, by direct measurement:

- **Marshalling / instance pool / first-dispatch plumbing:** the entire dispatch
  path is ~0.95 ms p50 (1.8 % of the bracket) — pool lease 5 µs, linker 196 µs,
  component instantiation 83 µs, region push 58 µs, typed export 593 µs,
  teardown 25 µs. `LayerStageInput` construction is 5 µs.
- **The arena memo:** it works. Every non-first module in the stage reads the
  prepared slots in **5 µs** (3.0–4.0 ms total over 496 calls).
- **The t127 name ("shared prepared-region view materialisation"):** half right.
  The materialisation is the derivation, not the marshal; and it is paid
  **twice** because `sync_perimeter_infill_areas_into_slice` takes and
  re-stages the slice (three times per layer: once after `Layer::Perimeters`,
  once per `PerimetersPostProcess` commit), and each `set_slice` clears the
  arena's prepared slots.
- **`Layer::Support` is not a host block:** `tree-support`'s `prep_views` is
  **5 µs**; its 85.9 ms p50 bracket on base classic-on is the in-module render
  (linker→call→teardown 85.4 ms), a work-set/phase difference, not plumbing.

**Stage-sharing is re-proven at HEAD**, not carried from t127: a probe-only
decline of `raft-default` moves the whole block to `gyroid-infill`
(**24,533.1 ms**, p50 50.0 ms) while every other Infill module stays under
3.5 ms and the output hash does not move (`9bb18f0e…`).

**The second derivation is provably redundant.** The probe hashes the fields
`prepare_regions` reads (region order and count, `region_id`, `object_id`
length, `nonplanar_surface` presence, every polygon coordinate) before and
after the fill sync: **0 of 1,488 syncs changed any of them** on base-off,
base-on and the decline arm. The sync writes only the four canonical fill
polygons, which the derivation does not read.

**Pricing** (ceilings = thread / t154's effective parallelism; walls of record
carried from t154; 0.80 % floor = 0.008 × PnP wall):

| | base classic-off | base classic-on |
| --- | ---: | ---: |
| redundant Infill-side derivation (thread) | 25,986.3 ms | 26,101.2 ms |
| ceiling (÷10.91x / ÷11.18x) | **2.382 s** | **2.335 s** |
| floor multiple | **12.15x** | **4.20x** |
| share of the cell's required wall cut | 21.5 % of 11.10 s | 4.9 % of 47.94 s |
| the destination's per-layer requirement | — | **1.096 s** |
| ceiling over that requirement | — | **2.13x** |

Indicatively, subtracting the ceiling from t154's canonical-prepass projection
takes base classic-on from **1.051x to ~0.943x** — the only losing cell in that
arithmetic would clear the line. That composition is an estimate, not a claim.

**Verdict (stop rule): removable — a proposed candidate with its ceiling, not
an adopted change.** Preserve the prepared-region memo across the fill-polygon
sync; clear it explicitly on the modifier-footprint path (which can mint or
remove regions and change `polygons`; zero footprints on the matched job, so the
clear never fires there). Exact by construction under the audit above. The
standing eight-cell gate (0.80 % paired wall/CPU, byte-identical output) is the
candidate ticket's own, graduated as
[156](156-prepared-memo-across-fill-sync.md).

**Not claimed:** no wall claim from any instrumented or probe pass; no ratio of
record; the box load was not sampled; no accelerated arm; no exactness argument
for a cheaper `needs_support` predicate (its 94 % share is measured, its
replacement is not designed — that is fog, not a ticket); no cause claim for the
pre-probe pass's slower p50 (63 ms vs 52.4 ms), whose box state was slower in
every phase.

Evidence: [`evidence/t155-infill-stage-block/`](../evidence/t155-infill-stage-block/FINDINGS.md)
— eight runs, `probe.json`, `summary.json`, `probe.patch` (reverted;
`git apply --check` clean), `verify.py` PASS 9/9 with `--self-test` 9/9
mutations rejected; the restored tree's `cargo xtask build-guests --check`
exits **0** and the restored binary reproduces `9bb18f0e…`.
