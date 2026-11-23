# clipper2 1.2.0 geometry delta (dependency direction, next bump)

Type: task
Status: resolved (2026-10-07; verdict: stay on 1.1.0, park the bump — 1.2.0's
delta is a canonical *fix*, not a regression, and it buys nothing this map
needs; see ## Answer)
Assignee: agpen (current OpenCode wayfinder session), 2026-10-07 — human's small
non-timing ticket family, next in number order after [profile wall-column
semantics](30-profile-wall-column-semantics.md) resolved; claim released on
resolution

## Question

[clipper2 cost/output verdict](17-clipper2-cost-output-verdict.md) established
that 1.0.3 → 1.1.0 is output-neutral, and in passing found that
**1.1.0 → 1.2.0 is not**: 2 of 367 fixtures differ, each losing one
collinear / near-collinear vertex, and 1.2.0's `clean_collinear`
(`clipper2-rust` `src/engine.rs`) replaces 1.1.0's re-read of
`outrec.pts` with the tracked `start_op`. Upstream has since also added a
`using_z` feature and example.

Does this repo want 1.2.0, and if so what output/parity work does the bump
imply? Work:

- Decide whether there is a reason to move at all (the current version is
  cost-neutral and DEV-173-carrying in both — 1.2.0's `check_split_owner` is
  byte-identical to 1.1.0's, so an upgrade does not improve the DEV-173
  posture and must not be motivated by it).
- If moving: re-run the ticket-17 harnesses (preserved in
  [`evidence/t17-clipper2-ab/tools/`](../evidence/t17-clipper2-ab/tools/README.md))
  with the 1.2.0 pin, attribute the two changed fixtures to the
  `clean_collinear` change, and run the fairness contract's output disclosure
  over at least the benchy/wedge matched cells.
- Check whether the `using_z` feature or the `PolyFace64` additions open a
  cheaper path for anything this map cares about (e.g. the contour/hole
  reconstruction in `expolygons_from_tree`, whose hand-rolled tree walk is
  what made the flat `union_64` API unattractive for DEV-173).

Not on the timing chain; parallel-takeable when a dependency decision comes
due. The harness-pin trap from ticket 17 applies: pin the harness dep and
assert the resolved version from the harness's own `Cargo.lock`.

## Answer — resolved 2026-10-07

**Verdict: stay on 1.1.0. Do not bump; leave the dependency as it is
(`clipper2-rust = "1.1.0"`, no manifest change).** The bump is
**geometry-touching** — ticket 17's onward finding is confirmed, not overturned
— but the delta is a **canonical-convergence fix**, so the natural reading of a
"differing fixtures" result inverts: **1.2.0 is the more-canonical side, and
1.1.0 is the one carrying the divergence.** No reason to move was found that
this map's destination actually needs; the work is recorded so the next
dependency decision starts from measured ground.

### (1) Is there a reason to move at all?

**No — and the DEV-173 motivation is explicitly closed off.** Re-derived from
the two registry sources (1.1.0 vs 1.2.0) with `#[cfg(feature = "using_z")]`
items and `z` field lines stripped (script:
[`strip.py`](../evidence/t31-clipper2-120/strip.py), re-derives the table in
[`FINDINGS.md`](../evidence/t31-clipper2-120/FINDINGS.md)):

- `check_split_owner`, `recursive_check_owners` (`engine.rs`) and
  `is_valid_owner` (`engine_fns.rs`) are **byte-identical** across the bump, as
  are `fix_self_intersects`, `dispose_out_pt`, `add_out_pt`, `start_open_path`,
  `add_local_min_poly` and `add_local_max_poly`. The unresolved-`split_idx`
  recursion and the `is_valid_owner`-as-mutation survive untouched, so
  **DEV-173's accepted-risk posture does not improve by upgrading** — exactly
  what ticket 17 predicted, now re-verified against the 1.2.0 source rather
  than a changelog.
- The rest of the feature-off delta is plumbing: `insert_left_edge` becomes an
  `if let Some(…)/else` refactor with the same two arms in the same order (the
  `None` arm is reached at the identical point), `offset.rs` replaces a
  `point64_from_f(x, y)` helper with `Point64::from(&p)` (identical
  `.round() as i64`), and `core.rs` swaps `#[derive(PartialEq)]` on `Point` for
  a manual impl that compares x/y only. Everything else is additive API
  (`make_path_z`, `make_path_zd`, `ellipse_rect64`, `ellipse_rect_d`,
  `set_z_callback`, the `From` conversions) or feature-gated. `minkowski.rs`,
  `rectclip.rs` and `version.rs` are effectively byte-identical.
- **No cost argument either way.** `using_z` is off by default
  (`default = []` in 1.2.0's manifest), and every `z_callback` / `default_z` /
  `set_z` reference carries `#[cfg(feature = "using_z")]`, so the feature-off
  build discards the whole mechanism. This take did **not** time either version
  and makes no speed claim; the point is only that the bump would not be a
  cost decision.

### (2) The two changed fixtures, attributed

Re-ran ticket 17's preserved harness
([`../evidence/t17-clipper2-ab/tools/`](../evidence/t17-clipper2-ab/tools/README.md))
in two throwaway trees differing only in the pin, each asserted from its own
`Cargo.lock` (1.1.0 / 1.2.0 — the t17 harness-pin trap was honoured, not
re-learned). `slicer-core` and `slicer-model-io` compiled **unmodified** against
1.2.0, so the bump is source-compatible for every surface this repo touches.

Result: `diff -rq` reproduces ticket 17 exactly — **2 of 367 fixtures differ**,
both a **single deleted vertex**, same point set and ring order otherwise:

| Fixture | 1.1.0 → 1.2.0 | Dropped vertex | Cross | Inside prev..next bbox |
| --- | --- | --- | ---: | --- |
| `benchy__layer01_opening.json` | 822 → 821 pts | `(174694, -2231)` | **0** | yes |
| `benchy__layer07_closing.json` | 1044 → 1043 pts | `(119056, 69941)` | **−3** | yes |

**Attributed to `clean_collinear`** — and the mechanism is a *fix*:

- Canonical Clipper2 `ClipperBase::CleanCollinear`
  (`OrcaSlicerDocumented/deps_src/clipper2/Clipper2Lib/src/clipper.engine.cpp`,
  `clipper.version.h` = **1.5.2** — what OrcaSlicer compiles against) restarts the
  walk via `startOp = op2; continue;`. That is **1.2.0's shape**. 1.1.0's version
  drops the restart and adds a second `outrec->pts` disjunct to the termination
  test; neither is canonical.
- Upstream's 1.2.0 release note names it: *"Zero-width spurs in union output
  (#9): `clean_collinear` did not restart its walk after removing a vertex, as
  C++ `CleanCollinear` does, so spikes could survive when `preserve_collinear`
  was on (the default). Output now matches C++."*
- **The gate is on in this repo.** `polygon_ops` never calls
  `set_preserve_collinear` on `Clipper64` (crate default `true`), and its
  `ClipperOffset::new(…, false, false)` passes `preserve_collinear = false`
  explicitly, which `ClipperOffset::execute_internal` propagates into the
  finishing union via `c.set_preserve_collinear(self.preserve_collinear)`. Both
  paths reach `clean_collinear` through `check_bounds` on the PolyTree route
  (`boolean_op_tree_64` → `recursive_check_owners` → `check_bounds`), which is
  the route `clip_polygons` uses.
- The dropped layer01 vertex is still present in 1.1.0's output and is
  **removable under `preserve_collinear = false`** — i.e. 1.1.0 left a spur 1.2.0
  removes. 1.2.0's ring is a fixed point under both flag settings.
- `_validations.json` is **byte-identical** across arms (the one
  simplicity-failing fixture, `bowtie`, fails identically on both sides), so
  1.2.0 introduces no invalid geometry, and the other **365 of the 367
  fixtures are byte-identical** by manifest SHA-256.

**Reading: the delta only ever shrinks output, on the axis upstream calls a
bug.** It is a two-vertex effect across a 367-fixture real-model corpus.

Falsifiable re-derivation, no raw tree required:

```bash
python3 docs/specs/perf-vs-orca/evidence/t31-clipper2-120/verify.py   # exits 0
```

### (3) Fairness-contract output disclosure over the matched cells

**Not run — deliberately, and disclosed as such.** The ticket's conditional was
"*if moving*"; this ticket does not move. Re-running the six-cell
benchy/base × classic/arachne × supports off/on disclosure would cost a full
matched-pair campaign to price a bump that (a) buys no speed, (b) does not
improve DEV-173, and (c) has no consumer asking for it. The unit corpus *is* the
disclosure that the move is output-near-identical; the campaign is the cost of
the *next* decision, not this one.

**Consequence if a future take does move:** the matched-cell disclosure is a
prerequisite, and the job's timing references are the post-t47 ones
(`5a61a106…` / `e1088b3d…`+alias per [Frozen-job output drift
boundary](46-frozen-job-output-drift-boundary.md)) — a 1.2.0 bump would emit new
bytes and invalidate whichever references the job currently uses.

### (4) Do `using_z` / `PolyFace64` open a cheaper path?

**No, for anything this map cares about.**

- **`using_z`** threads a per-vertex `z` through clipping/offsetting. This
  repo's clipper surfaces are 2-D by construction — `polygon_ops` converts
  `ExPolygon` (integer x/y, `1 unit = 100 nm`) to `Point64` and back, and Z
  lives one level up in layer IR. Carrying Z *through* clipper removes no work
  on any measured hot path (per-path re-clip, winding predicate, carve gate); it
  is orthogonal capability, and it is off by default anyway.
- **`PolyFace64` / `poly_tree_to_faces64`** already shipped in **1.1.0** (ticket
  16's finding), is **unused here** (zero callers in `crates/`/`modules/`), and
  is not new in 1.2.0. It returns a flat face list of `outer` + immediate
  `holes`; it does **not** expose the nested island-inside-hole hierarchy that
  [`expolygons_from_tree`](../../../crates/slicer-core/src/polygon_ops.rs)
  reconstructs, and it sits *downstream* of the recursion DEV-173 lives in
  (`check_split_owner`). So it does **not** make the flat `union_64` API newly
  attractive: the hand-rolled tree walk would still be required, minus nothing.
  The ticket's premise that `PolyFace64` is a 1.2.0 addition is corrected here —
  it is a 1.1.0 addition, and it is still unused.
- `ellipse_rect64` / `ellipse_rect_d`, `make_path_z` / `make_path_zd`, the
  `From<&Point64>` / `From<&PointD>` conversions: additive, unused, no cost
  effect.

### Boundary

**No manifest bump, no pin, no vendoring, no commit.** `clipper2-rust` stays
`"1.1.0"` exactly as ticket 17 left it. Nothing in this take changed production
code, ran a timing campaign, or touched the win matrix — the take is a
dependency decision plus its re-derivable evidence.

Evidence: [`evidence/t31-clipper2-120/FINDINGS.md`](../evidence/t31-clipper2-120/FINDINGS.md)
plus `strip.py` (source delta), `reduce.py` (raw → in-repo reduction),
`verify.py` (exits 0 against the in-repo set) and `manifest.json`.
