# Reproduce t129

## Checkout-only

From the workspace root:

```bash
python docs/specs/perf-vs-orca/evidence/t129-prepass-work-set-parity/verify.py
python docs/specs/perf-vs-orca/evidence/t129-prepass-work-set-parity/verify.py --self-test
```

- `verify.py` re-derives every number in `FINDINGS.md` from `census.json`:
  stage sums, the clean work-set tranche and its shares, the derived Orca wall,
  the prepass-only and full-parity route multiples, and every shape's floor
  multiple. It also checks that the report's headline numbers are present.
- `--self-test` mutates the census and the report in eight ways and requires
  every mutation to be detected. It does not need the canonical checkout.

Expected output:

```text
VERDICT: PASS (4 cells, 6 shapes, 20 prose claims)
SELF-TEST: 8/8 negative controls rejected
```

## Canonical citation check (optional)

The canonical tree is not part of this repository. If the local checkout is
present at the pin, run:

```bash
python docs/specs/perf-vs-orca/evidence/t129-prepass-work-set-parity/verify.py --orca-check
```

It checks:

- `D:\slicerProject\Orca(pnp_gui)` exists;
- `git -C "<checkout>" rev-parse HEAD` equals
  `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`;
- every file in `canonical_citations.json` exists and its first function symbol
  appears in that file.

Expected output at the pin:

```text
ORCA-CHECK PASS: 14 citations resolve at 40ecc66f78ed
VERDICT: PASS (4 cells, 6 shapes, 20 prose claims)
```

If the checkout is absent the command prints `ORCA-CHECK SKIP` and does not
fail; the in-repo census is still reproducible without it.

## Inputs

| input | use |
| --- | --- |
| `census.json` | the stage walls, cell arithmetic, shapes and route table |
| `canonical_citations.json` | the pinned canonical file + function citations |
| `FINDINGS.md` | the report the verifier checks |
| `../t126-post-t123-phase-split/summary.json` | stage walls (instrumented attribution) |
| `../t124-post-t123-scoreboard/` | ratios of record (uninstrumented paired medians) |
| `../t75-shellclass-phase-b/split.json` | ShellClassification phase-B split, two whole-layer offsets |
| `../t84-common-floor/summary.json` | OverhangAnnotation and SeamPlanning splits |
| `../t76-footprint-union/FINDINGS.md` | MeshAnalysis union census and partitioned cost |
| `../t113-strong-planner-phase1/reduced/summary.json` | planner sub-term split |
| `../t70-clipper-candidate/FINDINGS.md` | Rust/C++ Clipper2 kernel parity |

## What the verifier does not do

- It does not re-measure the slicers. The stage walls are t126's instrumented
  brackets; the ratios are t124's paired medians. The verifier checks the
  arithmetic that combines them, not the measurements.
- It does not resolve the canonical citations unless `--orca-check` is passed.
- It does not price shape S4 or S5, which the report explicitly leaves
  unpriced.
- It does not run the Orca or PnP slicer, rebuild guests, or touch the
  reference G-code set.

## Negative controls in `--self-test`

1. Move a stage wall by 500 ms (stage sum / clean tranche).
2. Overstate the clean tranche by 500 ms.
3. Move the ratio of record (derived Orca wall).
4. Move the prepass-only multiple.
5. Move the full-parity multiple.
6. Move a shape's floor multiple.
7. Move a clean-tranche required-cut share.
8. Remove a headline number from the report.

Each mutation must produce at least one failure; the self-test prints
`8/8 negative controls rejected`.
