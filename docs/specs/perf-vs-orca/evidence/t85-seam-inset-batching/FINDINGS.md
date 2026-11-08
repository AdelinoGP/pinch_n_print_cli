# Seam planner inset-offset batching — paired gate and attribution

Route: [Seam planner inset-offset batching](../../issues/85-seam-inset-offset-batching.md).
Candidate: a guest-side restructure of
`run_region_planning_entries`
(`modules/core-modules/seam-planner-default/src/lib.rs`) that computes every
region's chosen seam first, issues **one** `offset_polygons_batch` for the whole
print, then projects each seam onto its own returned inset. No IR shape change,
no seam-selection change, no host change, no geometry change.

## Verdict

**Gate met in all four pre-declared cells: wall falls, CPU inside the
work-preserving band, every retained pair byte-identical, one SHA per arm.**

| cell | paired wall (median) | paired CPU (median) | pairs | exclusions |
| --- | ---: | ---: | ---: | ---: |
| ordinary base classic off | **−7.46 %** | +0.45 % | 6/6 | 0 |
| ordinary base classic on | **−2.72 %** | +0.34 % | 6/6 | 0 |
| accelerated base classic off | **−7.03 %** | +0.47 % | 6/6 | 0 |
| accelerated base classic on | **−3.86 %** | +0.35 % | 6/6 | 0 |

Every one of the 24 retained pairs is byte-identical within its pair and to the
post-t81 reference set, and each arm emitted exactly one G-code SHA across all
its repeats in a cell. **No pair moved the wrong way**: the individual wall
ratios run 0.887–0.977, so the median is not carrying an outlier.

The **off cells recover 7 %** of cell wall and the **on cells 3–4 %** — the
difference is the denominator, not the recovery: the same ~6 s comes off a
~85 s cell and a ~190 s cell. Ticket 84 priced the shape at **5.43 %**
(off) using t78's 2.6x effective speedup for the same `map_batch` mechanism;
the measured recovery is **larger**, which the attribution section below
explains (the batch's effective speedup on this request set is far above 2.6x).

CPU rises by **at most +0.47 %** — well inside the +3.0 % work-preserving
ceiling, and consistent with "the same work, moved host-side and fanned out"
rather than with skipped work (output identity rules that out independently).

## Method

- HEAD `de65378a` + the candidate. Snapshots built with
  `cargo xtask dist --edition developer` (ordinary) and
  `cargo xtask dist --accelerated --edition developer`, one arm each, the
  baseline arm being the same tree with the planner source stashed.
- Standing protocol (t57 → t74 → t76 → t78): uninstrumented runs, external
  `GetProcessTimes` wall + CPU, `RAYON_NUM_THREADS=12`, external modules only
  (`--no-default-module-paths --no-integrated-modules`), 24/24 modules staged.
  Six alternating pairs per cell after two warmups, alternating the arm order
  each pair. A pair is excluded whole when either arm's cpu/wall ratio falls
  below 0.5x that arm's own median; **zero exclusions were needed**.
- Machine state: quiet (load 2–10 % before the campaign; the earlier
  pre-campaign sanity pair was taken while a game held ~38 % of the box and is
  **not** used anywhere in this record).
- Qualification before timing: 16/16 cells, two arms, byte identity against the
  derived t81 reference set — see below.

## Qualification (output identity)

`proof-rows.json` → `qualification.json`: **pass, 16/16 cells, 0 problems.**
Both arms in every cell emit identical bytes; the 8 ordinary cells equal the
post-t81 reference set that `derive_references.py` re-derives from
`evidence/t81-resolution-port/campaign/*-rows.csv` (base classic off
`292df501…`, on `22200671…`, matching ticket 84's references exactly).

The accelerated artifacts have **no post-t81 reference set** — t81's campaign
was ordinary-mode only — so those 8 cells are gated on arm-to-arm identity and
their agreement with the ordinary cell is reported instead:
**cross-mode equal 8/8**, which is t72's standing ordinary≡accelerated property
holding at this HEAD.

## Disclosures

- **The two arms' `pnp_cli.exe` are not byte-identical.** `pnp-cli` depends on
  `slicer-integrated-modules`, which links `seam-planner-default` into the host
  binary even for the developer edition, so the candidate's source delta moves
  the host binary by 10,240 bytes. Both arms run with
  `--no-integrated-modules`, so that linked copy is never dispatched; the guest
  module is the only **executing** difference. Provenance is pinned on the
  artifact that actually executes: the snapshot's `seam-planner-default.wasm`
  is byte-identical to a rebuild of the committed source.
- **Accelerated-mode build prerequisite.** A cold accelerated build could not
  complete on this tree: the rustc shim resolved the workspace root from the
  process CWD, which for a dependency's `build.rs` is a cargo-registry package
  directory. Fixed inside this take and recorded as
  [ticket 86](../../issues/86-accelerated-shim-workspace-root.md)
  (`PNP_XTASK_WORKSPACE_ROOT` exported by the three accelerated entry points,
  re-validated in `resolve_shim_root`; `cargo test -p xtask` 187/0). No
  candidate behaviour depends on that fix; without it the accelerated arm of
  this gate could not be measured at all.
- The instrumented attribution runs use `--instrument-stderr --profile
  --profile-verbose`; their walls are attribution only and never quoted as a
  speed claim (`docs/17_agent_debugging.md`).
- One object per fixture; counts below are per-object.

## Attribution

Four instrumented captures (both arms, both base classic cells). Instrumented
walls are attribution only, never wall claims; the guest view projection
(~4–11 ms) and the host harvest (~45–58 ms) are unchanged between arms, so the
**stage delta equals the guest offset delta**.

| cell | seam stage, baseline | seam stage, candidate | drop |
| --- | ---: | ---: | ---: |
| base classic off | 8,356 ms | **1,737 ms** | 6,619 ms (−79.2 %) |
| base classic on | 8,288 ms | **1,703 ms** | 6,585 ms (−79.5 %) |

**Batch occupancy ≈ 7x on the offset request set.** Derivation: ticket 84
measured the non-offset share of the same job's guest call at **7.55 %**
(`1 − 8,195.4 / 8,865` ms), i.e. the offset term is 92.4 % of the
`PrePass::SeamPlanning` guest call. Applying that share to this baseline
(8,355 ms guest call) leaves ≈631 ms of non-offset guest work, so the offset
term ran ≈7,724 ms and now runs ≈1,105 ms. The conservative bound — attribute
**every** remaining millisecond of the candidate stage to the offset term —
still gives **4.4x**. Both are far above the ticket's pre-declared **1.15x**
DROP floor, and the 2.6x t78 pricing that ticket 84 used to predict 5.43 % /
2.41 % was itself too pessimistic, which is why the measured off-cell recovery
(−7.46 %) exceeds that pricing and lands just under the "every offset vanished"
bound (8.83 %).

**Residual serial terms** (unchanged by this candidate, measured in ticket 84 on
the same job): candidate generation 94.6 ms + scoring 16.6 ms + unscoped guest
work 98.4 ms ≈ 210 ms of guest compute, plus the host view projection
(4.2 / 11.4 ms) and harvest (58.1 / 44.7 ms). The stage is now **1.7 s of a
72 s cell (2.3 %)** — it has stopped being a common-floor term.

**Guest fuel rises 3.858 B → 3.978 B (+3.1 %)** because the whole print's
requests are now built as one WIT list. That is metered guest compute, not wall:
the wall fell 79 % while it rose, and output identity rules out skipped work.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md). `verify.py --self-test` exits 0 against the
in-repo evidence alone.
