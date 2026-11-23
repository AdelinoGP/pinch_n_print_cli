# t31 — clipper2-rust 1.1.0 vs 1.2.0 geometry delta

Ticket: [clipper2 1.2.0 geometry delta](../../issues/31-clipper2-1-2-0-geometry-delta.md).
Started 2026-10-07. Every figure here is a ledger fact — re-derive at the point of use.

## Verdict

**Stay on 1.1.0; park the bump.** The delta is real and is a
**canonical-convergence fix**, not a regression: 1.2.0's `clean_collinear`
restores canonical Clipper2's `startOp = op2` restart that 1.1.0 dropped, and
the repo's own bundled canonical Clipper2 (1.5.2 — what OrcaSlicer compiles
against) matches **1.2.0**, not 1.1.0. So the pre-1.2.0 output is the divergent
side.

It is parked rather than taken because a bump buys **nothing this map's
destination needs**: no speed, no DEV-173 improvement (that code is
byte-identical across the bump), and no blocked consumer. It is a
geometry-touching change, so taking it would invalidate the job's post-t47
timing references and require its own matched-cell disclosure campaign — real
cost for a two-vertex effect on an axis nobody measured as harmful.

The decision is **reversible and now cheap**: the source delta, the fixture
attribution and a re-derivation kit are all recorded here, so taking the bump
when a consumer needs the spur fix (e.g. the parity program filing it as a
correctness issue) is a fresh scope decision on measured ground rather than a
re-investigation.

## Source-level delta (feature-off, i.e. as this repo builds it)

Method: `strip.py` in this directory strips `#[cfg(feature = "using_z")]`
items and `z` field lines, canonicalises `Point::new` spellings, then diffs
each module of the two registry sources. Re-derive with:

```bash
python3 strip.py          # prints every effectively-changed hunk
```

Result: **the feature-off semantic delta is one geometry function plus one
control-flow refactor.** Everything else is either feature-gated `using_z`
plumbing that compiles away, a spelling change, or additive API:

| Module | Item | 1.1.0 | 1.2.0 | Reading |
| --- | --- | --- | --- | --- |
| `engine.rs` | `clean_collinear` | `op2 == start_op \|\| op2 == outrec.pts` termination, no restart | `start_op = op2; continue` | **THE geometry delta** — canonical convergence |
| `engine.rs` | `insert_left_edge` | `if self.actives.is_none() { … } else { let x = self.actives.unwrap(); … }` | `if let Some(actives_idx) = self.actives { … } else { … }` | control-flow refactor; the `None` arm is reached at the same point, no behaviour change |
| `engine.rs` | `do_split_op` | no z hook | `#[cfg(feature = "using_z")]` z-callback block | feature-gated, absent when off |
| `engine.rs` | `intersect_edges` | inline `add_out_pt(...)` etc. | binds `_result_op` then `#[cfg(feature = "using_z")] if self.z_callback.is_some()` | feature-gated; `_result_op` is a sink the feature-off build discards |
| `offset.rs` | several | `point64_from_f(x, y)` helper | `Point64::from(&p)` | `from` rounds identically (`x.round() as i64`); helper deleted |
| `offset.rs` | `do_group_offset` | — | `#[cfg(feature = "using_z")] for p in … { p.z = pt.z }` | feature-gated |
| `core.rs` | `Point` | `#[derive(PartialEq)]` | manual `PartialEq` ignoring `z` | comparison behaviour unchanged without `using_z` (documented: z is only a field under the feature) |
| `core.rs` | `get_segment_intersect_pt{,_d}` | — | `#[cfg(feature = "using_z")] ip.z = 0` | feature-gated |
| `core.rs` | `ellipse_rect64`, `ellipse_rect_d` | — | new `Ellipse(const Rect<T>&)` overload | additive API, unused here |
| `clipper.rs` | `make_path_z`, `make_path_zd` | — | new | additive API, unused here |
| `engine_public.rs` | `set_z_callback` | — | new (x2) | additive API, unused here |

`minkowski.rs`, `rectclip.rs`, `version.rs`: byte-identical after the strip.
`lib.rs`: only `#[cfg(all(test, feature = "using_z"))] mod using_z_tests;`.

Feature-gating verified directly, not inferred: `z_callback` / `default_z`
(`engine.rs`) and every `self.z_callback.is_some()` call site carry
`#[cfg(feature = "using_z")]`, and the feature is off by default in 1.2.0's
manifest (`default = []`). `Point.z` and `ZType` likewise.

## DEV-173 posture is unchanged by the bump (re-derived)

The ticket asks whether an upgrade improves DEV-173. It does not:

| Symbol (file) | 1.1.0 vs 1.2.0 |
| --- | --- |
| `check_split_owner` (`engine.rs`) | byte-identical (1,861 B both) |
| `recursive_check_owners` (`engine.rs`) | byte-identical (1,904 B both) |
| `is_valid_owner` (`engine_fns.rs`) | byte-identical |
| `fix_self_intersects` (`engine.rs`) | byte-identical |
| `dispose_out_pt`, `add_out_pt`, `start_open_path`, `add_local_min_poly`, `add_local_max_poly` | byte-identical |

The unresolved-`split_idx` recursion, the `pts.is_none()` pre-resolution
descent, and the `is_valid_owner`-as-mutation all survive the bump untouched.
**DEV-173's accepted-risk posture does not improve by upgrading**, exactly as
ticket 17's onward finding said; the bump must not be motivated by it.

## Canonical alignment (the decisive fact)

Canonical Clipper2 `ClipperBase::CleanCollinear`
(`OrcaSlicerDocumented/deps_src/clipper2/Clipper2Lib/src/clipper.engine.cpp`,
`clipper.version.h` = **1.5.2**) reads:

```cpp
OutPt* startOp = outrec->pts, * op2 = startOp;
for (; ; )
{
  if (IsCollinear(...) && (...))
  {
    if (op2 == outrec->pts) outrec->pts = op2->prev;
    op2 = DisposeOutPt(op2);
    if (!IsValidClosedPath(op2)) { DisposeOutPts(outrec); return; }
    startOp = op2;            // <-- restart
    continue;
  }
  op2 = op2->next;
  if (op2 == startOp) break;  // <-- plain termination
}
```

That is **1.2.0's** shape, line for line. 1.1.0's version both drops the
restart and adds the `outrec->pts` second disjunct to the termination test —
neither is canonical. Upstream's own 1.2.0 release note names the fix
explicitly ("Zero-width spurs in union output (#9): `clean_collinear` did not
restart its walk after removing a vertex, as C++ `CleanCollinear` does, so
spikes could survive when `preserve_collinear` was on (the default). Output now
matches C++.").

**Therefore the t17 "2 of 367 fixtures differ" result is 1.2.0 being *more*
canonical, not less.** The pre-1.2.0 output is the divergent one. That inverts
the natural reading of a "differs" finding: on this repo's parity-first
standards the 1.2.0 side is the *correct* side.

## Reachability on this repo's paths

`clean_collinear` is reached from `check_bounds` (`engine.rs`) on the PolyTree
path — `boolean_op_tree_64` → `recursive_check_owners` → `check_bounds`. This
repo uses exactly that path: `clip_polygons`
(`crates/slicer-core/src/polygon_ops.rs`) calls `boolean_op_tree_64`, and
`union_ex` / `intersection_ex` / `difference_ex` and the whole
`expolygons_from_tree` contour/hole reconstruction route through it.

The `preserve_collinear` gate (the condition under which 1.1.0 leaks spurs) is
**on**: the repo never calls `Clipper64::set_preserve_collinear`, so the
crate default `true` applies. Both call sites (`Clipper64`, `ClipperD`) were
grepped; zero callers in `crates/` or `modules/`.

## Work item 3 — do `using_z` / `PolyFace64` open a cheaper path?

**No, for anything this map cares about.**

- **`using_z`** adds a `z` field threaded through clipping/offsetting so
  callers can carry a per-vertex Z through a boolean op. This repo's clipper
  surfaces are 2-D only: `polygon_ops` converts `ExPolygon` (`i64` x/y, `1 unit
  = 100 nm`) to `Point64` and back, and Z exists one level up in layer IR.
  Carrying Z *through* clipper would not remove any work on the measured hot
  paths (the per-path re-clip, the winding predicate, the carve gate); it is an
  orthogonal capability. It is also off by default, so it is not a cost
  argument either way.
- **`PolyFace64` / `poly_tree_to_faces64`** is the additive read-side API that
  already shipped in 1.1.0 (ticket 16's finding) and is **unused here** (grepped:
  zero callers in `crates/`/`modules/`). It returns a flat face list, each face
  an `outer` + immediate `holes` — it does **not** hand back the nested
  island-inside-hole hierarchy that `expolygons_from_tree`
  (`crates/slicer-core/src/polygon_ops.rs`) reconstructs by hand, and it does
  not address the recursion DEV-173 lives in (which is inside
  `check_split_owner`, upstream of any tree-walk API). So it does not make the
  flat `union_64` API newly attractive for this repo: the hand-rolled tree walk
  would still be required, minus nothing.
- `ellipse_rect64` / `ellipse_rect_d`, `make_path_z` / `make_path_zd`,
  `From<&Point64>/<&PointD>` conversions: additive, unused, no cost effect.

## Measurement (re-derivation of the t17 onward finding)

**Reproduced exactly.** The 367-fixture corpus harness is preserved at
`../t17-clipper2-ab/tools/outdiff-main.rs` + `outdiff-Cargo.toml`. Two
throwaway trees were built here, differing only in the pinned dependency, and
each asserted from its **own** `Cargo.lock` before its run (the t17 harness-pin
trap):

| Tree | Pin in harness `Cargo.toml` | Resolved in own `Cargo.lock` |
| --- | --- | --- |
| `tmp/t31-outdiff-110` | `clipper2-rust = "=1.1.0"` | 1.1.0 |
| `tmp/t31-outdiff-120` | `clipper2-rust = "=1.2.0"` | 1.2.0 |

Both arms ran the same command on the same inputs:

```bash
./tmp/t31-outdiff-{110,120}/target/release/t17-outdiff.exe \
  --out tmp/t31-out-{110,120} \
  --stl resources/regression_wedge.stl --model tmp/3dbenchy.stl --tag benchy
```

`slicer-core` and `slicer-model-io` compiled **unmodified** against 1.2.0 —
the bump is source-compatible for every surface this repo touches.

> **Note on the fixture name.** The harness dumps the `closing_ex` result under
> the stem `{tag}__layer{i:02}_closing` (see `outdiff-main.rs`), so the corpus
> file is `benchy__layer07_closing.json` even though the entry point called is
> `closing_ex`.

`diff -rq` reproduces ticket 17's onward finding: **2 of 367 fixtures differ**,
and both are a single dropped vertex. Corpus composition re-derived here:
`benchy` 121 (24 sample layers × 5 dumps + 1 whole-slice union), `wedge` 201
(50 layers × 4 + 1 union), `grid{16,64,256}` 33 (3 sizes × 11 ops), `annulus` 8,
`hole_overlap` 3, `bowtie` 1 = 367.

| Fixture | 1.1.0 | 1.2.0 | Dropped vertex | Cross | In prev..next bbox |
| --- | ---: | ---: | --- | ---: | --- |
| `benchy__layer01_opening.json` | 822 pts | 821 pts | `(174694, -2231)` | **0** (exact) | yes |
| `benchy__layer07_closing.json` | 1044 pts | 1043 pts | `(119056, 69941)` | **−3** (near) | yes |

Attribution, re-derived rather than quoted:

- Each 1.2.0 ring is 1.1.0's ring with **exactly one vertex deleted in place** —
  same point *set* otherwise, same start point, same ring order. (Not a
  rotation, not a re-trace.)
- The dropped vertex is exactly, or within 3 units² of, collinear with its
  neighbours, and lies inside their bounding box: it is a **removable spur
  vertex**, not a real corner.
- The layer01 `(174694, -2231)` vertex is still present in the 1.1.0 output and
  is **removable under `preserve_collinear = false`** — which is the flag
  `polygon_ops` actually passes (`ClipperOffset::new(…, false, false)`; third
  positional arg is `preserve_collinear`, and the offset's finishing union
  propagates it via `c.set_preserve_collinear(self.preserve_collinear)` in
  `ClipperOffset::execute_internal`). The 1.2.0 ring has no removable vertex
  left under either flag setting: it is a fixed point of the canonical walk.
- `_validations.json` is **byte-identical** across arms (the one
  simplicity-failing fixture, `bowtie`, fails identically on both sides), so
  1.2.0 removes no valid geometry and introduces no invalid output.
- The other 365 fixtures are byte-identical by manifest SHA-256 (366 of the
  367 are unchanged *or* accounted for by the analysis above; `_counts.json`
  differs only because it is the summary that records this delta).

So the delta is 1.2.0 finishing the cleanup 1.1.0 left half-done — the spurs the
upstream release note names. It is a **two-vertex** effect across a 367-fixture
real-model corpus, on an axis (spur removal) that only shrinks output.

### Reproduce

```bash
# reduced evidence set (in-repo, no raw tree required)
python3 docs/specs/perf-vs-orca/evidence/t31-clipper2-120/verify.py   # exits 0
```

`manifest.json` carries SHA-256 for all 369 files per arm;
`out-110/` and `out-120/` carry the two differing fixtures plus the summary
files. The rest is reachable by re-running the two harness trees above.

> **Verifier scope.** `verify.py` checks the *reduced* set shipped here — the
> manifest hashes for all 369 files, and full ring-level analysis of the two
> differing fixtures. It does not re-run the corpus (that needs
> `tmp/3dbenchy.stl`, gitignored and licence-encumbered). Its check 3 reports
> "1.1.0 had 0 removable" for `benchy__layer07_closing` because the dropped
> vertex there is *near*-collinear (cross −3), not exactly so, and the crate's
> `is_collinear` is an exact `ProductsAreEqual` test; the layer01 fixture is the
> one that demonstrates the mechanism exactly.

## Why 1.1.0's spurs are not obviously free to keep

They are not merely cosmetic on the *unit* surface: the spur vertex survives
into the emitted `ExPolygon` contour, so every downstream consumer of
`opening` / `closing_ex` / `offset2_ex` / `union_ex` output — 8, 13, 16 and 48
call sites respectively outside `polygon_ops.rs` — inherits a contour with a
zero-length (or near-zero) edge. Whether that is *harmful* depends on the
consumer, and this take did **not** measure downstream impact. It is stated
here as the reason the question is not settled by "output is nearly identical":
it is settled by "the fix is canonical, the cost is unmeasured, and nothing in
this map's budget needs it."


## Recipe notes for a future bump

- A 1.2.0 bump is a **geometry-touching** change: the fairness contract's
  output disclosure applies to the matched cells, not just the unit corpus.
- The `using_z` feature is **off by default** and must stay off for a
  cost-neutral bump; enabling it changes `Point` layout on every hot path.
- `Point`'s `PartialEq` is a manual impl in 1.2.0. It compares x/y only, so
  behaviour is unchanged feature-off — but any code that hashes or derives on
  `Point` should be re-checked if the feature is ever enabled.
- The harness-pin trap from ticket 17 still applies verbatim.
