# Findings: annotation-free `build_wall_flags` fastpath

## Final decision and evidence correction

The user accepted and committed this fast path for repeatable CPU savings,
explicitly relaxing the wall-time requirement for this candidate. The earlier
provisional recommendations below are historical. No reliable wall win is claimed.

**Quiet follow-up labeling error:** the scripts in `quiet-validation/` used
`$gen` to label runs but reused the Classic config. The runs labeled Arachne
below actually selected Classic: their G-code says `wall_generator = Classic`
and stderr reports Arachne losing the perimeter claim. Therefore the quiet
Arachne rows and Arachne-specific acceptance conclusions below are INVALID;
retain them only as raw, mislabeled evidence. Earlier interleaved Arachne runs
used the correct top-level `"wall_generator": "arachne"` and remain valid.
Module diagnosis lists available modules; it does not prove runtime selection.
Any future run must verify its selected generator in stderr/G-code.

The candidate is promising and should not be auto-dropped. It avoids the
per-wall geometric reprojection work when no effective material or fuzzy-skin
annotation can influence the result. `variant_fuzzy` remains seeded before the
predicate, so painted variants retain fuzzy skin without requiring an
annotation scan. Ineffective `Scalar`, `Custom`, `Flag(false)`, empty, missing,
or wrong-polygon values take the fastpath. Real material paint still enters the
existing path and retains transition behavior.

Focused verification passed:

- `cargo test -p slicer-core --features host-algos --test inner_wall_material_boundary_tdd`: 10 passed.
- `cargo test -p slicer-core --features host-algos --test inner_wall_concave_reprojection_tdd`: 2 passed.
- `cargo test --manifest-path modules/core-modules/classic-perimeters/Cargo.toml --test boundary_paint_tdd`: 7 passed.
- `cargo test --manifest-path modules/core-modules/arachne-perimeters/Cargo.toml --test boundary_paint_tdd`: 4 passed.
- `cargo check --workspace --all-targets`, clippy with `-D warnings`, and `cargo xtask check-literals`: passed.

The added unit coverage checks ineffective paint with reprojection geometry,
exact output length including an extra closing slot, outer fallback, inner
fallback, and `variant_fuzzy` for both wall classes. Existing tests cover real
material paint, transitions, and concave reprojection. The guard now also
documents the precise defensive behavior change: an ineffective-annotation
call with `Some(&[])` geometry and positive `num_points` returns defaults rather
than taking the former panic path; production callers reject empty wall paths.
This does not claim all malformed reprojection inputs are semantically
unchanged.

No WIT or host contract changed. No caller, closure, or reprojection behavior
was modified. No temporary probe code was added.

## Acceptance follow-up measurements

The complete per-sample CPU/wall tables are recorded in `EXPERIMENT.md`; raw
CSV files are under the corresponding `interleaved-*` directories. These
follow-up runs used the preserved baseline snapshot versus the current
candidate and 12 Rayon workers.

### Supports-off Benchy, interleaved baseline/candidate/baseline/candidate

| generator | baseline wall / CPU | candidate wall / CPU |
|---|---:|---:|
| classic | 36.1951 / 165.8438; 36.4534 / 166.9688 | 36.1311 / 141.0938; 33.9544 / 147.4531 |
| Arachne | 26.7840 / 80.4688; 25.9159 / 80.5312 | 25.5349 / 78.4688; 24.7800 / 79.2344 |

### Original tree-support Benchy, interleaved

| generator | baseline wall / CPU | candidate wall / CPU |
|---|---:|---:|
| classic | 44.9295 / 168.8438; 44.2824 / 174.3594 | 44.5201 / 167.8438; 44.0760 / 162.9219 |
| Arachne | 35.2290 / 98.9531; 34.2534 / 98.1562 | 34.4486 / 92.8906; 34.5100 / 96.3906 |

### Original tree-support base, repeated

| generator | baseline wall / CPU | candidate wall / CPU |
|---|---:|---:|
| classic | 566.5472 / 2905.4688; 524.3012 / 2788.1250 | 505.3584 / 2559.8750; 429.6910 / 2561.8594 |
| Arachne | 320.6457 / 1376.3906; 372.9869 / 1376.2344 | 386.2650 / 1351.5000; 332.7867 / 1333.5000 |

The base-support command hit a 40-minute harness limit after the classic rows
and first Arachne baseline; the remaining three Arachne runs were completed in
a follow-up command. This was a measurement orchestration limit, not a slice
failure. All completed runs exited 0. Base stderr showed `degraded: true`, zero
fatal errors, and exactly 29,108 non-fatal errors for every baseline and
candidate run, preserving DEV167 visibility. Benchy stderr showed zero fatal
and zero non-fatal errors.

The controlled Benchy wall samples moved lower for both generators, but remain
load-qualified; with only two repeats per side, the controlled-wall acceptance
gate is **not demonstrated**. CPU was lower for the candidate in every
supports-off, supports-on Benchy, and base comparison group. Base Arachne wall
is also inconclusive.

## Recommendation

**PROVISIONAL candidate; ACCEPTANCE INCONCLUSIVE.** CPU evidence is favorable
and the exact-output guard tests plus parity/error checks support correctness.
The candidate is suitable for user review, but this evidence does not justify
labeling the controlled wall gate as passed or declaring a final KEEP decision.
No further performance runs are requested in this bounded follow-up.

## Quiet-machine controlled wall follow-up

Date: 2026-09-07. Raw CSV and logs are under
`tmp/perf-flags/quiet-validation/`. `cargo xtask build-guests --check` returned
exit 0 before timing. The preserved baseline and candidate identity, dispatch,
and guest hashes are recorded in `HANDOFF.md`.

Each side/generator received one explicit warmup, then ABBA BAAB (four measured
samples per side), with 12 Rayon workers, no report, and no instrumentation.
All measured runs exited 0. The harness recorded wall, process CPU, peak
working set, output bytes, and output hash.

### Benchy, supports off

| generator | side | wall median / range (s) | CPU median / range (s) |
|---|---|---:|---:|
| Classic | baseline | 20.2949 / 19.8971–23.7612 | 156.1562 / 155.3125–156.7812 |
| Classic | candidate | 20.5261 / 19.7627–24.9625 | 142.7188 / 141.5469–143.3438 |
| Arachne | baseline | 20.7738 / 20.1375–22.2503 | 155.6250 / 153.8594–155.9219 |
| Arachne | candidate | 20.2759 / 19.6160–22.4710 | 142.2891 / 140.8125–144.0312 |

Candidate/base median ratios: Classic wall 1.0114 and CPU 0.9139; Arachne
wall 0.9760 and CPU 0.9143. **FACT CPU acceptance: PASS/favorable for both
generators. FACT WALL acceptance: INCONCLUSIVE for both generators**; ranges
overlap and Classic is not faster by median.

### Benchy, original tree supports on

| generator | side | wall median / range (s) | CPU median / range (s) |
|---|---|---:|---:|
| Classic | baseline | 26.9623 / 26.7044–27.7479 | 168.9531 / 168.6562–169.4531 |
| Classic | candidate | 27.4307 / 25.6223–28.0987 | 156.5859 / 155.6250–157.4219 |
| Arachne | baseline | 27.0163 / 26.1387–28.3125 | 169.6797 / 168.7812–170.5000 |
| Arachne | candidate | 26.3285 / 25.4236–27.0960 | 156.1250 / 153.8906–158.0938 |

Candidate/base median ratios: Classic wall 1.0174 and CPU 0.9268; Arachne
wall 0.9745 and CPU 0.9201. **FACT CPU acceptance: PASS/favorable for both
generators. FACT WALL acceptance: INCONCLUSIVE for both generators**; Classic
is slower by median and Arachne's apparent improvement is small relative to its
observed range.

The one-time external-load snapshots recorded aggregate CPU load 12 before and
7 after; Opera and Discord were among the top background processes. These are
indicators, not proof of continuous idle conditions. Recommendation:
**INCONCLUSIVE awaiting USER decision**; do not auto-drop, but do not label the
controlled wall gate passed.
