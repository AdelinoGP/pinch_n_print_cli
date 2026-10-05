# t17 — clipper2-rust 1.0.3 vs 1.1.0 measured A/B

Ticket: [clipper2 cost/output verdict](../../issues/17-clipper2-cost-output-verdict.md).
Run 2026-09-24 (AFK agent session) on the Windows dev box. Every figure here is
a ledger fact — re-derive at the point of use.

Companion: [STATIC-DELTA.md](STATIC-DELTA.md) (per-file source comparison),
[CODEGEN.md](CODEGEN.md) (compiled-body comparison). Raw artifacts live under
`target/t17-clipper2-ab/` (gitignored, regenerable).

## Verdict

**The 1.0.3 → 1.1.0 bump carries no measurable cost and no output delta.** Stay
on 1.1.0; no pin, no rollback, no vendoring.

- **Output: byte-identical on every fixture measured.** 367 dumps (33 grid
  fixtures across three grid sizes and eleven op variants, hole/annulus/bowtie
  geometry, 50 wedge layers through `offset2_ex`/`offset`/`clip_polylines`, 24
  real `3dbenchy.stl` layers through
  `offset2_ex`/`opening`/`closing_ex`/`clip_polylines`, and two accumulated
  whole-slice `union_ex` inputs) compare **identical** between the two
  versions — `diff -rq` exit 0, whole-tree SHA-256 equal, per-leaf polygon /
  hole / vertex counts equal. Both sides are deterministic across reruns
  (self-diff exit 0), so the equality is signal, not chance.
- **Cost: no systematic difference.** The interleaved paired runs (same binary
  builds, alternating A/B rounds) show per-leaf deltas straddling zero with
  run-to-run spread dominating; the codegen comparison is the load-independent
  backstop — **230 of 240 compiled function bodies are bit-identical**, and the
  three nominal differences are constant-pool label placement and one extra
  generic instantiation, never a geometry instruction sequence.
- **The static prediction is confirmed empirically** ([clipper2 1.1.0 upstream
  evidence](../../issues/16-clipper2-1-1-0-upstream-evidence.md)): 1.1.0's only
  code delta is the additive `PolyFace64` / `poly_tree_to_faces64` read-side
  API, which this repo never calls.

DEV-173's `check_split_owner` recursion is **unaffected by the bump** (1.0.3's
and 1.1.0's `src/engine.rs` are raw byte-identical; the defect ships in both).
The bump was never a fix and is not claimed as one — DEV-173 remains open on
its own terms.

## Method (and the one trap this session hit)

The repo's caret req is `clipper2-rust = "1.1.0"`, and cargo cannot hold two
versions of one crate in one graph (verified: `=1.0.3` + `=1.1.0` is a resolver
conflict). The faithful A/B is therefore **two isolated trees differing only in
the clipper pin**:

- main tree (`Cargo.toml` = `"1.1.0"`, resolves 1.1.0) — the production graph;
- throwaway worktree `.worktrees/t17-clipper103` (pin `"=1.0.3"`), whose
  `crates/` tree is content-identical to main's (verified by normalized diff:
  line-ending-only differences) and whose lock differs from main's in exactly
  the `clipper2-rust` entry.

**The trap, recorded because it silently produced a false "real difference".**
The first harness run resolved `clipper2-rust 1.2.0` — not 1.1.0 — because the
harness lived in its own single-crate workspace (`tmp/t17-outdiff`) whose lock
had never been written, so slicer-core's workspace-inherited caret req floated
up to the newest compatible release. The A/B therefore compared **1.0.3 vs
1.2.0**, and that comparison *does* differ (below). Pinning the harness dep
`clipper2-rust = "=1.1.0"` and re-running produced the true 1.0.3-vs-1.1.0
result: identical. **Any future clipper A/B must assert the resolved version
from the harness's own `Cargo.lock`, not from the tree it was copied out of.**

Measurement discipline otherwise followed the map's recipe traps: guest
freshness checked first (`cargo xtask build-guests --check` exit 0), uninstrumented
runs only, like-for-like statistics (t15 finding #1 — means and slopes read from
`estimates.json`, never the console `time:` line), and repeats on the noisy
leaves.

**External-load caveat (map §10.3).** The run window had sustained external CPU
load (a separate session's `arachne_structural_invariants` test binary at
~8.5 cores, plus browser processes); observed load fluctuated 33–100%. The
paired/interleaved design and the per-run CVs make the *absence* of a systematic
cost delta evident, but **no small cost delta (< ~10%) is resolvable from these
timings** — the codegen comparison is what closes that question.

## Output evidence

Corpus: 367 JSON dumps per version, canonical serialization of every result.

| Group | Files | What it exercises |
| --- | ---: | --- |
| `grid{16,64,256}__*` | 33 | The bench's own square grids (3 sizes × 11 ops): `union` / `intersection` / `difference` / `offset` (Miter/Round/Square, ±delta) / `xor` / `union_ex` / `offset2_ex` / `opening` / `closing_ex` |
| `annulus__*`, `bowtie__*`, `hole_overlap__*` | 12 | Hole nesting through the PolyTree reconstruction path (`expolygons_from_tree`), contour/hole orientation, simplicity validation of positive and negative fixtures, overlapping holes |
| `wedge__layer*` | 201 | 50 layers of `resources/regression_wedge.stl` through raw slice → `offset2_ex` / `offset` → `clip_polylines` (hatch-and-clip — the lightning-infill / infill-linker surface) |
| `benchy__layer*` | 121 | 24 real `tmp/3dbenchy.stl` layers through `offset2_ex` / `opening` / `closing_ex` / `clip_polylines` |
| (whole-slice unions) | 2 | `wedge__slice_all_union_ex` / `benchy__slice_all_union_ex` — accumulated polygon sets (the DEV-173 `region_needs_support` shape) |

Volume on the 1.1.0 side (measured, `corpus_volume.py`): 367 files — 482
ExPolygons, 1,205 holes, 115,204 ring points, plus 9,629 clipped polylines
(19,258 points) from the open-path surface; 2 fixtures legitimately empty.

Result: `diff -rq out-103 out-110` → **exit 0** (no differing file); whole-tree
sorted-cat SHA-256 equal on both sides, and the per-run `_counts.json` /
`_validations.json` summaries are byte-identical too. Both sides were re-run
(start to finish) and each reproduced itself exactly, so the equality is
deterministic signal, not a chance match.

On the ticket's requested delta classification (vertices, winding, collinear
handling): every class was *exercised*, so their absence is meaningful rather
than untested — the dumps are ordered point sequences, so winding or vertex
loss would change bytes; and the morphological fixtures (`opening`,
`closing_ex`, `offset` with ±delta and all three join types) produce collinear
runs, which is exactly the surface where 1.1.0→1.2.0 *does* differ (see the
onward finding below). The comparison simply found nothing to classify.

## Cost evidence

### Interleaved paired runs

Same two binaries, alternating A(1.0.3)/B(1.1.0) rounds, identical criterion
settings, distinct baseline per run; each filter invoked separately (criterion's
positional filter is a substring match, not a regex). Initial 4-round pass:

| Leaf | 1.0.3 median µs | 1.1.0 median µs | paired Δ | per-run CV range |
| --- | ---: | ---: | ---: | --- |
| `union/256` | 209.98 | 249.55 | +13.6% | 17–18% |
| `intersection/256` | 221.55 | 298.00 | +14.5% | 15–18% |
| `difference/256` | 224.02 | 246.44 | +8.3% | 16–19% |
| `offset/256` | 230.90 | 237.96 | +4.1% | 16–18% |
| `union/64` | 45.11 | 42.59 | −4.5% | 18–19% |
| `intersection/64` | 40.85 | 47.08 | +13.1% | 18–19% |
| `difference/64` | 42.03 | 47.14 | +20.2% | 16–20% |
| `offset/64` | 66.21 | 52.87 | −10.4% | 17–22% |

The deltas straddle zero and are not monotone in work size; the run-to-run CV
exceeds every pooled difference. A two-run same-binary check put the noise
floor at −18% to −52% on the four 256-square leaves (CV up to 97%). Under
§10.3's "external load masquerades as contention" the initial pass alone is
**not** evidence of a version effect in either direction.

A **10-round extension plus a reversed-order pass and a same-version
cross-tree control** was then run to isolate the residual (the two runs above
were not enough to say what the floor really was):

| Pass | Pooled mean | Pooled median | Per-leaf medians |
| --- | ---: | ---: | --- |
| A/B forward (1.0.3 first) | +8.1% | +5.0% | +2.8% … +9.1% |
| A/B reversed (1.1.0 first) | +19.6% | +3.4% | −2.2% … +12.7% |
| **Control: both 1.1.0**, cross-tree | **+0.3%** | **+0.2%** | **−10.4% … +6.7%** |

The control is the key row: two builds of the *same* version from the two trees
produce no systematic pooled bias (+0.3%), only per-leaf scatter of ±10% with
SEM 2.5–5.3% at n=10. That is the machine's resolution floor, and the A/B
order-cancelled deltas (+0.7% to +10.6%, no consistent sign within a leaf
across orders) sit inside it. Combined with identical `.text`/`.rdata` section
sizes and the bit-identical bodies below, the honest reading is **no
measurable version effect**; the residual is link-layout sensitivity under
external load, recorded rather than chased.

### Codegen comparison (load-independent)

`--emit=asm` for the clipper2-rust crate at each version, with the probe
forcing the exact call surfaces this repo uses (`boolean_op_tree_64`,
`union_64`, `inflate_paths_64`, `ClipperOffset::execute_tree`, `Clipper64::execute`).
After normalizing symbol hashes, anonymous-constant names, and local label
numbering: **240 common function base names, 230 bodies bit-identical, 3
nominal differences — all non-semantic.** See [CODEGEN.md](CODEGEN.md) for the
per-difference analysis.

## Onward finding — 1.2.0 does change output (outside this ticket's question)

Recorded because the harness trap above surfaced it, and because
[clipper2 1.1.0 upstream evidence](../../issues/16-clipper2-1-1-0-upstream-evidence.md)
flagged v1.2.0 (tagged 2026-09-18) as unexamined. **This is not part of ticket
17's 1.0.3-vs-1.1.0 verdict and no recommendation is attached** — it is a
heads-up for the next dependency decision.

- **1.1.0 vs 1.2.0 output differs** (2 of 367 fixtures): one benchy layer's
  `opening` result loses one vertex, another layer's `closing_ex` result loses
  one vertex. In both cases 1.2.0 emits the shorter ring and 1.0.3/1.1.0 the
  longer one; the dropped vertex is collinear (cross product 0) on the first
  and near-collinear (|cross| = 3 units² ) on the second. Counts and all other
  fixtures agree.
- **The mechanism is a real code change**: 1.2.0's `engine.rs` differs by 191
  lines from 1.1.0 and, in `clean_collinear`, drops 1.1.0's
  `outrec.pts.is_some() && op2 == outrec.pts.unwrap()` loop-termination
  condition in favour of the tracked `start_op`.
- **DEV-173 is unaffected**: 1.2.0's `check_split_owner` is byte-identical to
  1.1.0's (same raw, unresolved-`split_idx` recursion and same
  `is_valid_owner`-as-mutation), so the accepted risk posture does not improve
  by upgrading.
- Practical reading: a 1.2.0 bump is a **geometry-touching** change requiring a
  parity/A-B pass of its own — unlike the 1.1.0 bump, which this ticket now
  establishes was output-neutral.

## Limits

- The load window prevents resolving small (< ~10%) cost deltas from wall
  timings; the codegen comparison covers that gap for *code* differences but
  cannot detect a difference that is purely in code layout (e.g. alignment).
  None was observed (`--emit=asm` bodies compared instruction-for-instruction).
- The corpus covers the clipper surfaces this repo calls; it is not a
  conformance suite for clipper2 as a whole.
- `tmp/3dbenchy.stl` and `tmp/base.stl` are gitignored model fixtures (map
  evidence policy); this evidence does not commit them. `base.stl` was not
  used — the corpus's largest boolean inputs are the wedge/benchy whole-slice
  unions.
