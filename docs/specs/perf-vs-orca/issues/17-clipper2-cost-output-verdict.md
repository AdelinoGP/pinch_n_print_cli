# clipper2 cost/output verdict

Type: task
Status: resolved
Blocked by: 15, 16
Assignee: wayfinder session (ses_f2b585f72ffeyxRz2rKSTeeEUB), 2026-09-24

## Question

Decide the dependency direction on `clipper2-rust` — stay on 1.1.0, roll back
to 1.0.3, or pin with a documented reason — from measured cost and output
deltas (DEV-173's question: the 1.0.3 → 1.1.0 bump landed without a parity
comparison).

Work:

- **Cost A/B**: run the refreshed `polygon_ops` bench
  ([Criterion bench refresh](15-criterion-bench-refresh.md)) at both crate
  versions, interleaved with the recipe's discipline; `union` / `difference` /
  `intersection` / `offset` are the surfaces every hot call site uses.
- **Output diff**: run identical polygon fixtures through both versions and
  classify any output deltas (vertices, winding, collinear handling) — small
  real-model G-code deltas are within known noise (§3.6), so unit-level polygon
  comparison carries this, not byte diffs.
- Fold in [clipper2 1.1.0 upstream evidence](16-clipper2-1-1-0-upstream-evidence.md)'s
  findings (incl. whether `check_split_owner`'s recursion reach changed between
  the versions) into the recommendation.

The verdict fixes which clipper cost profile the optimization tickets
([offset2_ex call reduction](19-offset2-ex-call-reduction.md) onward) measure
against. Keep/drop to the human; nothing auto-committed.

## Answer

Resolved 2026-09-24 (AFK agent session). **Verdict: stay on 1.1.0; no pin, no
rollback.** The 1.0.3 → 1.1.0 bump is cost-neutral and output-identical,
measured, so there is nothing to pin against and nothing a rollback would fix.
Evidence: [`evidence/t17-clipper2-ab/FINDINGS.md`](../evidence/t17-clipper2-ab/FINDINGS.md)
plus its [STATIC-DELTA.md](../evidence/t17-clipper2-ab/STATIC-DELTA.md) and
[CODEGEN.md](../evidence/t17-clipper2-ab/CODEGEN.md) companions.

**Output: byte-identical.** Two isolated trees differing only in the clipper
pin ran an identical 367-fixture corpus through the module's full entry-point
surface (`union` / `intersection` / `difference` / `xor` / `offset` /
`offset2_ex` / `opening` / `closing_ex` / `union_ex` / `polygon-simplicity` /
`clip_polylines` / whole-slice unions; the corpus includes 24 real
`3dbenchy.stl` layers and 50 wedge layers, 482 ExPolygons, 1,205 holes,
115,204 ring points, 9,629 clipped polylines). `diff -rq` **exit 0**; whole-tree
SHA-256 equal; both sides reproduce themselves exactly on a rerun. This is the
empirical confirmation ticket 16's static reading called for.

**Cost: no systematic delta.** Two independent lines of evidence:

1. The interleaved paired bench runs (alternating 1.0.3/1.1.0, identical
   criterion settings, four named surfaces at 64 and 256 squares) show deltas
   straddling zero with per-run CV 15–22% (forward 10-round pass +5.0% pooled
   median; reversed-order pass +3.4%). A **same-version cross-tree control**
   (two 1.1.0 builds, one per tree — 10 rounds) is the key row: pooled mean
   **+0.3%**, per-leaf medians −10.4% to +6.7%. The machine's resolution floor
   under the run window's external load is therefore ~10%; the A/B deltas sit
   inside it, the two binaries have identical section sizes, and (below) their
   function bodies are identical. No small version effect is resolvable — and
   none exists to resolve.
2. **Load-independent:** the compiled instruction bodies agree. 240 function
   base names in common, **230 bit-identical**, 3 nominal differences — a
   constant-pool label placement shared between `PolyTreeD::add_child` and
   `core::scale_path`, and one extra generic instantiation of
   `RawVec::grow_one`. No geometry function's instruction sequence differs.

**Static re-derivation (ticket 16 folded in):** the geometry modules
(`engine.rs`, `engine_fns.rs`, `core.rs`, `offset.rs`, `rectclip.rs`,
`minkowski.rs`) are byte-identical or line-ending-only; the only content deltas
are *additions* in `clipper.rs` (+58, zero removed) — the `PolyFace64` /
`poly_tree_to_faces64` read-side API this repo never calls. `check_split_owner`
therefore has **identical recursion reach and trigger conditions in both
versions** (DEV-173's risk is unchanged by the bump; the bump was never a fix
and is not claimed as one).

**Cost profile for the downstream tickets.** [offset2_ex call
reduction](19-offset2-ex-call-reduction.md) and everything after it measure
against the same profile whether the lock says 1.0.3 or 1.1.0 — the choice is
free at this seam. Note the bench's own caveats still apply (the two 256-square
leaves are the high-CV ones; t15's slope-vs-mean rule for any comparison).

**Onward context (not this ticket's question, no recommendation attached):**
the harness trap that produced a false "real difference" also produced a real
onward finding — **1.1.0 → 1.2.0 is *not* output-neutral.** 2 of 367 fixtures
differ (a benchy `opening` and a `closing_ex` result each lose one collinear /
near-collinear vertex), and the mechanism is a real code change in 1.2.0's
`clean_collinear` (the tracked `start_op` replaces 1.1.0's re-read of
`outrec.pts`). 1.2.0's `check_split_owner` is still byte-identical to 1.1.0's,
so an upgrade does not improve the DEV-173 posture. A future bump toward 1.2.0
needs its own geometry A/B; the 1.1.0 bump, this ticket now establishes, did
not.

**Process hazard recorded for future dependency A/Bs.** A scratch harness in
its own workspace inherits only the *caret* requirement, so its lock floats to
the newest compatible release — the first run here silently measured 1.2.0
while labelled 1.1.0. Always pin the harness's own dependency and assert the
resolved version from the harness's `Cargo.lock`.

Keep/drop to the human: the recommendation is to keep 1.1.0 as-is (no manifest
change), with the above caveat noted for any 1.2.0 decision. Nothing was
committed to the dependency graph.
