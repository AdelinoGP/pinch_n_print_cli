# Ticket 74 — LayerFinalization supports-on whole-print view cost

Take: 2026-10-18, ordinary base classic, instrumented observer captures at
`67ef742b` (slicer sources = the t70 keep `7127b4d6`). Attribution first. The
standing paired ordinary + accelerated gate ran on a quiet machine on
2026-10-19 and **passed**; the human approved **KEEP** and the candidate is
committed `091e6bc9`.

## Question

The t73 base refresh measured the four `PostPass::LayerFinalization` module
brackets at 45.0–48.5 s on base supports-on against 0.30–1.39 s supports-off,
while their combined guest fuel was 5.45 B of 12,964 B (0.042 %). This take
decomposes those brackets, names the owner, and tests representation-preserving
reductions.

## Attribution

Four fresh ordinary base classic captures: baseline (HEAD) and candidate, each
supports-on and supports-off, with an env-gated probe
(`probe.patch`; `PNP_T74_PROBE=1`, inert without the variable). All four outputs
are byte-identical to the preserved t58 references: supports-on
`f60e5e9b…` / 43,912,792 B, supports-off `20e7204c…` / 22,428,863 B.

Per-module bracket decomposition (ms; the bracket is
`module_start`→`module_complete`):

| supports-on term | classifier | part-cooling | skirt-brim | wipe-tower | sum | share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| input snapshot full clone | 3,010 | 2,939 | 3,187 | 2,976 | 12,112 | 19.7 % |
| input view entity copy | 146 | 127 | 141 | 134 | 549 | 0.9 % |
| guest call (`run-finalization`) | 449 | 387 | 543 | 437 | 1,816 | 2.9 % |
| store drop (snapshot + view free) | 3,564 | 4,274 | 4,372 | 4,443 | 16,653 | 27.1 % |
| merge rollback full clone | 3,309 | 3,509 | 3,079 | 3,549 | 13,446 | 21.8 % |
| old layer-vector drop | 3,942 | 4,153 | 4,342 | 4,503 | 16,940 | 27.5 % |
| merge apply | 1 | 1 | 0.5 | 0.5 | 3 | 0.0 % |
| **bracket total** | **14,430** | **15,400** | **15,674** | **16,051** | **61,555** | |

Read plainly: **96.1 % of the supports-on bracket is whole-print copy/free
churn** (snapshot clone + its free + merge rollback clone + old-vector free);
the guest call is 2.9 %. The four modules' guest fuel (0.042 % of slice fuel) is
consistent with that: the guest early-returns or walks entities cheaply, while
the host materialises and frees the whole print around it. Term sums differ from
the brackets by 0.1–0.6 % (untimed instrumentation and allocator-adjacent work).

Supports-off control on the same build: module sum 1,290 ms (brackets 334 / 336
/ 300 / 320 ms). On/off bracket ratio 47.7x.

### Why the support-state jump is not proportional to output bytes

| quantity | supports-off | supports-on | ratio |
| --- | ---: | ---: | ---: |
| module bracket sum | 1,290 ms | 61,555 ms | 47.7x |
| output bytes | 22,428,863 | 43,912,792 | 1.96x |
| entities over 495 layers | 6,527 | 30,278 | 4.64x |
| path points over 495 layers | 3,887,933 | 6,066,434 | 1.56x |
| `support_entity_identities` rows | 0 | 23,751 | — |
| full input-view clone, per module | 30–45 ms | 2,939–3,187 ms | 66–106x |
| entity-only view copy, per module | 27–42 ms | 127–150 ms | 3.5–4.6x |

The entity-only copy tracks the entity/point volume (3.5–4.6x). The full
`LayerCollectionIR` copy is 66–106x, and the only field with a material on/off
count difference is the IR-only `support_entity_identities` side table (23,751
rows, each carrying `family_id`, `body_id` and a `demand_ids: Vec<String>`; none
of it is emitted). Annotations are 495 rows in both states, retracts 988 in both.
So the jump is owned by the whole-print clone/free of that support-attribution
side table — plus the second copy of it in the merge rollback clone — not by the
module's declared reads, and not by the printed support path volume.

## Candidate (uncommitted)

Two representation-preserving reductions, `candidate.patch`:

1. **Share the input snapshot.** `finalization_layer_snapshot` now stores
   `(layer_index, Arc<Vec<PrintEntity>>)` shared with
   `LayerCollectionViewData.ordered_entities` instead of cloning the full
   `LayerCollectionIR`. `get_ordered_entities` still serves the same pre-apply
   entity list; `LayerCollectionViewData::from_ir` is the only entity copy.
   This removes the 2.9–3.2 s clone and most of the 3.6–4.4 s store-drop free per
   module.
2. **Skip provably no-op / infallible merges.** `order_lock_remap_is_noop`
   mirrors `apply_to`'s final tag walk. With no pushes and a no-op remap,
   `apply_to` would write nothing, so the whole-print rollback clone is skipped.
   A stream of annotations (all targets validated to exist) plus append-only
   entity pushes has no fallible merge op, so it is applied directly to the
   stage-local working copy. Streams with modify/sort/insert/synthetic/set-order
   ops keep the rollback clone; the contract test
   `malformed_finalization_merge_rolls_back_prior_entity_push_on_both_transports`
   stays green.

Candidate probe, same cells:

| supports-on term | classifier | part-cooling | skirt-brim | wipe-tower | sum |
| --- | ---: | ---: | ---: | ---: | ---: |
| input view entity copy | 150 | 46 | 38 | 30 | 263 ms |
| guest call | 607 | 544 | 623 | 598 | 2,372 ms |
| store drop | 32 | 33 | 20 | 24 | 109 ms |
| merge apply | 0 | 0.5 | 0.2 | 0 | 0.7 ms |
| **bracket total** | **796** | **632** | **689** | **658** | **2,775 ms** |

(Term sums differ from the brackets by 0.1–1.1 %; untimed instrumentation and
allocator-adjacent work.)

- Supports-on module brackets **61,555 → 2,775 ms (−95.5 %)**.
- Supports-off module brackets **1,290 → 1,252 ms (−2.9 %, inside run noise;
  no regression)**.
- Outputs byte-identical to the t58 references in both support states.
- 24 executor finalization tests and 10 wasm-host finalization contract tests
  pass.

## Standing paired gate (2026-10-19)

Four pre-declared cells, 6 alternating baseline/candidate pairs plus two
warmups each, frozen snapshots, external `GetProcessTimes`,
`RAYON_NUM_THREADS=12`, on a quiet machine (09:08–11:50). All 24 measured pairs
were retained (no starved pair), both arms were clean, and every pair's output
was byte-identical.

| cell | wall median baseline → candidate (s) | paired wall Δ | paired CPU Δ | pairs |
| --- | --- | ---: | ---: | ---: |
| ordinary benchy supports-on | 19.11 → 18.55 | −3.13 % | −0.91 % | 6/6 |
| ordinary base supports-on | 361.95 → 315.95 | −12.71 % | −2.30 % | 6/6 |
| accelerated benchy supports-on | 17.32 → 16.64 | −3.89 % | −0.84 % | 6/6 |
| accelerated base supports-on | 326.76 → 279.24 | −14.23 % | −2.97 % | 6/6 |

**Verdict: PASS (wall win and CPU corroboration in every cell).** Source:
`ab-reduction.json`; driver log `target/t74/measure-morning.log`.

Read plainly:

- On base supports-on the candidate's median wall falls **46.00 s** ordinary
  (361.95 → 315.95; paired median ratio −12.71 %) and **47.52 s** accelerated
  (326.76 → 279.24; paired −14.23 %), inside t73's **45.0–48.5 s** bracket
  attribution.
- The per-arm CPU medians fall by roughly the same absolute amount (ordinary
  1,944.10 → 1,895.71 s; accelerated 1,515.39 → 1,469.27 s; paired CPU ratios
  −2.30 % / −2.97 %), consistent with the removed term being serial host
  copy/free rather than guest work. The CPU/wall ratio rises from 5.36 to 6.01
  (ordinary) and 4.63 to 5.26 (accelerated): the same total work with less serial
  host time.
- Benchy saves 0.56 s ordinary and 0.68 s accelerated wall.
- All measured outputs are `76eb4e1d…` (benchy) and `f60e5e9b…` (base), matching
  the preserved t58 references; no baseline/candidate divergence in any pair.

The probe's 61,555 ms baseline bracket was captured in a materially slower
machine state than this gate; it is the internal split, not a wall claim. The
t73 45.0–48.5 s bracket is the wall attribution the standing gate reproduces.

## Evidence and verification

- Raw captures (gitignored): `target/perf-t74-probe2/` and
  `target/perf-t74-candidate2/`, produced with
  `run-refresh-base.ps1 -Generator classic -Supports {on,off} -Kind instrumented`
  plus `PNP_T74_PROBE=1`.
- Reduced, in-repo set: `split.json`; reducer `reduce.py`; checkout-only
  verifier `verify.py` (exit 0, three negative controls rejected).
- Diagnostic patch: `probe.patch` (candidate + probe, reverted before verdict).
  Production candidate: `candidate.patch`.

## Caveats

- Machine state drifted materially during the take: the baseline supports-on
  wall was 571.9 s against t73's 408.7 s, and the candidate run 409.0 s. **No
  wall or CPU claim is made from these runs**; the internal bracket split is the
  measurement. The map's recipe traps apply (instrumented runs never become wall
  evidence).
- The **proof gate is 16/16 PASS** (`qualification.json`: baseline/candidate
  byte-identical, clean, non-vacuous in every cell, including the accelerated
  matrix), and the **standing paired A/B passed on 2026-10-19** with 6/6 pairs
  retained in every cell. The human approved **KEEP**; the candidate is
  committed `091e6bc9`. The interrupted set from the previous evening remains
  preserved as `measure-interrupted-20261019` and is **not** part of this
  result.

## Closeout

The standing ordinary and accelerated developer distributions were rebuilt at
the kept HEAD after the source and docs commits. Both `pnp_cli.exe` hashes and
all 24 modules per mode are byte-identical to the take-era candidate snapshots
(`ac01456d…` ordinary, `b8263a03…` accelerated); both guest freshness gates exit
0. The pre-commit candidate-built standing copies are therefore replaced by
kept-HEAD copies with matching bytes.

## Next route

Proof and standing gate complete and passing; **KEEP approved and candidate
committed `091e6bc9`**. The remaining ~2.4 s of candidate brackets is guest call
time (~85 %), now the largest term; further reduction needs a separate
representation change and a new pre-declaration.
