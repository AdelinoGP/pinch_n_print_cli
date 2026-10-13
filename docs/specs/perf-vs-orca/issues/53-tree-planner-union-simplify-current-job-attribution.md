# Tree-planner union/simplify current-job attribution

Type: task
Label: wayfinder:task
Status: resolved (2026-10-06; four diagnostic cells qualified, sources/artifacts restored)
Assignee: agpen (current OpenCode diagnostic continuation), 2026-10-06
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 52

## Question

On current matched supports-on Benchy and base jobs, what does the tree-planner
emit pass actually spend on host union versus guest simplification, what real
input/representation shapes reach those operations, and does that evidence
justify scoping one exact call-reduction candidate?

## Authorization and context

[Tree-planner union/simplify investigation scope](52-tree-planner-union-simplify-investigation-scope.md)
owns the human-approved diagnostic contract, exactness boundary and exclusions.
Claim before work. This ticket authorizes diagnosis, not an optimization or
speed gate. Read the map's Notes, the scope answer,
[Tree-planner substage findings](../evidence/t22-planner-substage/FINDINGS.md)
and [Emit-pass carve findings](../evidence/t32-carve-gate/FINDINGS.md).
Historical combined brackets are motivation, not current-tree cost or savings.

## Execution claim

The human corrected the scoping-only handback and explicitly requested
continuation into diagnosis in this session. Claim recorded before diagnostic
work. This supersedes the prior later-session handoff, not the diagnostic-only
boundaries or the prohibition on candidate implementation and speed campaigns.

## Work

1. **Preflight and isolate.** Inspect actual source and available inputs; preserve
   unrelated changes. Consult `debug-pipeline`, the relevant architecture docs
   and the repo's guest/test discipline. Record source/build/config/model
   identities and verify both ordinary and accelerated guest freshness
   independently. Use `tmp/3dbenchy.stl`, `tmp/base.stl` and
   `evidence/matched-pair/configs/pnp-classic-supports-on.json`. Prepare isolated
   external Developer baseline snapshots; verify provenance rather than allowing
   silent integrated fallback. Capture fresh post-repair output references and
   disclose bytes, TYPE counts, generator, degraded/non-fatal status and support
   presence. Stop if inputs are absent or identities/status cannot be established.
2. **Map the current chain.** Inspect `build_roles`, `union_expolys`,
   `role_simplify_tolerance`, `expolygons_simplify` and `simplify_ring`
   (`modules/core-modules/tree-support-planner/src/lib.rs`), SDK clipping
   routing and `HostExecutionContext::clip_polygons`
   (`crates/slicer-wasm-host/src/host.rs`). Separate body/roof/base/floor
   request independence from dependencies inside each chain and the later
   body-minus-interface differences. Do not assume simplification is active or
   a host batch service performs it.
3. **Fresh bounded attribution.** Add temporary, uniquely tagged diagnostic
   probes without algorithm changes. Capture the approved fixture/mode cells.
   Split union host-core work, conversions/round trip, guest simplification and
   surrounding work; state inclusive/exclusive boundaries. Never sum nested
   clocks or call accumulated worker spans CPU. Record actual union invocations
   versus empty/singleton bypasses, simplify activation, input/output polygon,
   hole and vertex counts, role/layer identities and enough input-shape detail
   to assess a concrete lead. Guest fuel excludes native boolean execution;
   instrumented wall is diagnostic only. If subtraction cannot reliably isolate
   a term, report a bounded combined bucket or gap rather than fabricated cost.
4. **Representation evidence.** Capture real pre-union/post-union and
   pre-simplify/post-simplify data plus final role results, retaining integer
   coordinates and order. Keep the unmodified current chain as an independent
   replay basis; do not change tolerances, role separation, carve winding/lone
   repair or subsequent differences. Identify what a later candidate must
   preserve and which existing tests can falsify its defects. Large raw captures
   may remain local; in-repo reductions must carry the headline claim and its
   provenance without depending on those captures. Do not read large JSON
   fixtures directly or commit the heavy models.
5. **Qualify probes and stop.** Compare diagnostic outputs to each mode's fresh
   baseline byte-for-byte and require clean, nonvacuous support output. Capture
   current-chain intermediate identity as feasible; disclose any unqualified
   surface. Stop on output drift or identity/freshness failure before interpreting
   attribution. Missing coverage, unstable results or an absent simplify path
   are findings, not reasons for automatic retries, substitutions or relaxed
   checks. Do not silently repair an exposed geometry/dispatch defect.
6. **Evidence and restoration.** Preserve headline findings, reduced captures
   and a falsifiable reducer/verifier under this effort's `evidence/`, usable
   from a checkout without gitignored raw trees. Run only relevant narrow tests
   if diagnostic/replay code needs validation; check target features first and
   tee test output per repo rules. Remove probes, restore sources and active
   artifacts without discarding unrelated work, and verify restoration and both
   mode-specific freshness gates. Report current attribution, representation
   hazards, coverage and gaps, with a recommendation to scope a candidate or
   park the lead. No speed win, candidate implementation, KEEP decision,
   automatic retry, commit, adoption retry or default switch.

## Answer — measured diagnostic resolution, 2026-10-06

**The combined union/simplify bucket is native body-union work, not active
simplification. No expensive removable call was established.** All four
fixture/mode diagnostic cells ran and qualified against fresh baseline outputs.
The actual split, counters, shape evidence and limitations live in
[Current-job union/simplify findings](../evidence/t53-union-simplify/FINDINGS.md).

- Simplification activations are **zero** in both modes on both jobs. Benchy's
  chains execute 181 real unions; base's execute 472. Empty/singleton bypasses
  already avoid host calls. Real body inputs are large merging requests, not
  redundant empty calls; roof/base/floor batching does not remove that work.
- Native union-core brackets measure 2.4277426 s / 1.8161515 s on ordinary /
  accelerated Benchy, and 74.6488767 s / 77.7053785 s on ordinary / accelerated
  base. These are single **instrumented serial diagnostic observations**, not
  acceptance timings, CPU, mode speedups or measured optimization savings.
  Host conversions and the combined transport/diagnostic residual are separate
  exclusive buckets in the findings; the inclusive guest union bracket must not
  be added again.
- All eight baseline/probe slices finish clean with external provenance and
  actual support extrusion; each baseline/probe output is byte-identical, also
  agreeing across modes per job. Real integer ordered intermediates are recorded
  and ten selected specimens pass native current-chain replay.
- Exact specimen selection does not cover largest unions or hole-bearing
  outputs, and replay stops before full four-role body subtraction. Shape census
  does observe holes and large inputs. This is a qualified diagnostic, not proof
  that any future union replacement preserves all representations.

**Recommendation:** park simplification/four-role batching call reduction.
If another structural take is selected, first scope large body-union
input/algorithm representation analysis with largest-request and hole-bearing
exact replay coverage. No candidate, speed win or permanent human park decision
is implied. The new question is
[Large body-union investigation scope](54-large-body-union-investigation-scope.md),
unclaimed and not authorized for implementation.

Validation actually executed: baseline/probe Developer builds and eight slices,
both mode-specific freshness gates, one native replay test (passed), checkout-only
reducer verification with negative controls, patch applicability and diff checks.
The main session independently re-summed streamed records, checked duplicate and
correlated request IDs, directly byte-compared all four actual output pairs,
verified current model/config/source/artifact hashes and complete active
distribution inventories against the saved pre-probe state, and reran both
freshness commands (each exit 0) plus the falsifying evidence verifier.

Temporary production edits are removed; production-source diff is empty and
inventoried artifacts/distribution file sets are restored exactly. Retained
probe/replay patches and isolated diagnostic snapshots are explicitly evidence,
not active production builds. Prior tracker/evidence work was preserved. No
known production side effects remain; no commit, automatic retry, adoption or
default change occurred.
