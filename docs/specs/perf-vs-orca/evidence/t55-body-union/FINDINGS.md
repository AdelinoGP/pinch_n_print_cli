# t55 — exact large body-union investigation

## Outcome

Completed the authorized diagnostic on the actual matched Benchy/base classic
supports-on jobs. Exact full-original-argument native `build_roles` replay passes
for eight selected cases, including largest requests, hole-bearing results and
four body-changing interface subtraction operations. Ordinary and accelerated
captures agree byte-for-byte. No production optimization, KEEP, speed gate,
commit, adoption/default change or tracker edit was made.

**Recommendation:** investigate a scanbeam-local cached `TopX` projection for an
unchanged active edge. The private counter diagnostic identified real repeated
calculations in a dominant native sweep path, with zero projection-value
mismatches across the selected population. This is a concrete avoidable-work
lead, not an implemented candidate or a measured speedup. Reject ordered exact
polygon deduplication as a useful lead for these cases: duplicate count is zero.
Conversion-only and output-reconstruction-only changes miss the dominant work.

## Qualification and coverage

`identity.json` binds the actual model/config/source identities, current-source
baseline snapshots, initial freshness and external module diagnosis.
`ordinary-probe-inventory.json` and `accelerated-probe-inventory.json` bind the
probe snapshots. No archived G-code served as a current output reference.
`archive-audit.json` records the checked t53 archive and tracked build inputs.
Build, diagnosis and freshness logs are preserved under `logs/`; large slice
logs and original fixtures stay in the private raw archive, identified by the
hash/size manifest in `validation.json`.

Eight fresh slices (baseline/probe × fixture × mode) completed with status `ok`,
no degradation, zero fatal/nonfatal errors and nonzero support extrusion.
`finalize.py` additionally performed actual streamed baseline/probe byte
comparisons, not just digest agreement, in all four fixture/mode cells. Each
Benchy output contains 8,658,776 bytes; each base output contains 43,912,905 bytes.
These are qualification outputs, **not candidate acceptance timings**.

Selected body-union shapes below are polygons / holes / vertices. These are
selected requests, not all-layer replay.

| Job/layer | Carved input | Union result | Body cuts that change output |
|---|---:|---:|---|
| Benchy 0 | 182 / 0 / 18,200 | 1 / 0 / 1,816 | none |
| Benchy 152 | 96 / 0 / 9,348 | 5 / 2 / 1,994 | none |
| Benchy 43 | 161 / 0 / 15,758 | 3 / 0 / 2,118 | floor runs; no change |
| Benchy 168 | 40 / 0 / 3,995 | 4 / 0 / 1,236 | roof |
| base 9 | 1,024 / 0 / 102,400 | 4 / 10 / 4,750 | none |
| base 21 | 921 / 0 / 92,112 | 4 / 12 / 5,129 | none |
| base 38 | 654 / 0 / 63,638 | 6 / 2 / 4,976 | roof |
| base 232 | 417 / 0 / 41,120 | 2 / 1 / 3,288 | roof and floor |

The largest-request and maximum-hole selection comes from the audited t53
census; the t55 reducer verifies those selected shapes. Five selected unions
contain holes. Four-role original segments/areas, collision geometry, radius,
width and average nodes are preserved, with f32 values as integer bit patterns.
Role order and ordered integer rings/holes are retained, not canonicalized or
compared by area. Units remain 100 nm. `capture.rs` and `probe.patch` document the
temporary capture, including carve winding/lone-region repair and the current
union/simplify/difference sequence.

`harness.rs::run` calls the restored production
`tree_support_planner::build_roles`
(`modules/core-modules/tree-support-planner/src/lib.rs`) with every recorded
original argument and compares complete ordered role results against independently
recorded WASM output. It also replays each body subtraction and compares the
recorded ordered intermediate. The separately instrumented body union matches
its captured ordered output exactly. The base L21 final body has 5,130 vertices,
not the union's 5,129: full-chain replay preserves this downstream distinction.

Native negative controls reject a changed vertex in eight cases, changed role
in eight, and a removed hole in five. `verifier-controls.py` separately executes
checkout-verifier sabotage controls for vertex/hole/role/native-report drift;
all four are rejected. These demonstrate comparator/verifier sensitivity, not
an algorithm's behavior on deliberately modified input geometries.

## Native decomposition

The private release harness patches a copy of `clipper2-rust` **1.1.0**, not the
Cargo registry or production algorithms. `harness-provenance.json` and
`harness.Cargo.lock` record the compiler, original library inventory, resolved
version, no new dependency versions and final diagnostic-source identities.
`internals.patch` includes the added `t55_probe.rs` and was applied to pristine
copies with exact resulting source hashes checked. `setup-harness.py setup`,
followed by applying `internals.patch` in the created harness, reconstructs the
diagnostic; setup deliberately refuses to overwrite an existing harness.

`native-internals-first.json` / `native-replay.log` preserve the first successful
timer-only run. `native-internals.json` / `native-counters-replay.log` preserve the
additional avoidable-work counter diagnostic, which reran the requested narrow
test for **new counters**, not to recover truncated output. Both test logs were
read and report `1 passed; 0 failed; 21 filtered out`. The earlier missing inline
test fixture compile failure is preserved in `native-harness-compile-block.log`;
copying the actual production fixture resolved it. The successful runs emit
12 dead-code warnings for unused copied helpers. Only the diagnostic test ran;
this is not a claim that all copied inline tests passed.

Timer-only observations (nanoseconds, one invocation per selected request):

| Phase | base L9 | base L21 |
|---|---:|---:|
| Flatten contours/holes to input paths | 501,400 | 424,300 |
| Boolean wrapper, inclusive | 420,270,500 | 409,699,200 |
| Add subject/clip paths, combined preparation | 2,216,300 | 1,718,700 |
| Execute internal, inclusive | 417,204,300 | 407,150,800 |
| Build output PolyTree | 427,800 | 462,100 |
| Cleanup | 3,100 | 1,100 |
| Reconstruct ordered ExPolygons | 28,600 | 32,500 |
| Intersection list build, nested in execute | 267,169,400 | 254,259,900 |
| Intersection list processing, nested in execute | 2,070,500 | 2,039,100 |
| Top of scanbeam, nested in execute | 128,861,600 | 129,192,900 |

Accounting: flatten/wrapper/reconstruction are disjoint, with outer overhead
left as residual. Add-path/execute/tree/cleanup are disjoint children of the
wrapper; allocation/teardown and other unmarked wrapper work remain residual.
Sweep reset/insertion/horizontals/intersections/top/final-horizontal-joins are
disjoint children of execute, with residual unmarked work. Intersection build
and process are children of sweep intersections; **do not add them again to
execute or wrapper**. In the counter run, `adjust_copy_ns` is a child of
intersection build, combining projection, SEL-link copying and diagnostic
snapshot overhead. The remaining intersection-build time combines merge work,
intersection-node creation and unmarked/observer work; it is not pure sort time.

All observations include instrumentation overhead. The second run also stores
diagnostic edge snapshots and counts merge comparisons, materially perturbing
the sweep. Neither run provides production latency, end-to-end savings,
repeatability, or a speed gate. No guest fuel is interpreted as native cost.

## Concrete candidate assessment

Actual Rust `ClipperBase::adjust_curr_x_and_copy_to_sel`,
`ClipperBase::build_intersect_list`, `ClipperBase::process_intersect_list` and
`ClipperBase::do_top_of_scanbeam` (`clipper2-rust/src/engine.rs` in the pinned
private copy) show this sequence: project each active edge to the next scanbeam
top, merge SEL to enumerate inversions, process intersections, then recompute
`TopX` for surviving nonterminal edges. Intersections can overwrite `curr_x`,
so simply omitting the second assignment is **not safe**.

Local canonical C++ `ClipperBase::AdjustCurrXAndCopyToSEL`,
`ClipperBase::BuildIntersectList`, `ClipperBase::ExecuteInternal` and
`ClipperBase::DoTopOfScanbeam` (`clipper.engine.cpp`) confirm the same broad
projection/intersection/top sequence. The C++ joined-left projection branch is
not present in the inspected Rust adjustment function; this investigation did
not port or repair that difference. A cache must preserve the actual Rust
projection and not silently import a different joined-edge rule.

The added probe records `(bot, top, dx bits, scanbeam y, vertex_top, TopX result)`
after adjustment, then checks that key/value against the normal recomputation
at scanbeam top. It changes no algorithmic result and does not use its snapshot
to replace a calculation.

| Case | Adjustment TopX calls | Unchanged-edge repeated TopX calls | Merge comparisons |
|---|---:|---:|---:|
| Benchy 168 | 98,808 | 94,813 | 285,896 |
| Benchy 152 | 268,008 | 258,660 | 765,546 |
| Benchy 43 | 706,258 | 690,500 | 2,363,107 |
| Benchy 0 | 592,180 | 573,980 | 2,080,725 |
| base 232 | 7,839,912 | 7,798,796 | 34,498,554 |
| base 38 | 7,785,078 | 7,721,441 | 31,334,174 |
| base 21 | 14,739,114 | 14,647,003 | 64,913,122 |
| base 9 | 14,244,614 | 14,142,214 | 63,508,135 |

Every recorded repeated call has an identical projection value; mismatch count
is zero in every case. **A future candidate could retain the already computed
projection in scanbeam-local edge state and restore it for unchanged edges,
falling back to the existing `TopX` on missing/invalid cache.** It must distinguish
the cached projection from mutable intersection `curr_x`, retain edge identity
and y, invalidate on geometry/dx/vertex replacement and edge recycling, and
leave all ordering, tie handling, winding, holes and role subtraction untouched.
This would remove concrete duplicate projection calculations, not the required
topology computation or AEL traversal. Added state/access/invalidation costs
may erase any benefit; **impact unmeasured**. No time is assigned exclusively to
`TopX`, and the inclusive top bucket is not a savings estimate.

Other assessed leads:

- Ordered exact carved-polygon deduplication: **falsified for this corpus**;
  zero exact duplicates in every selected request, so there is no removable
  duplicate polygon here. This does not prove absence across all layers.
- Flattening/prepared paths or reconstruction-only work: measurable but small
  relative to native execution here; does not explain the dominant sweep.
- Union skipping, hole flattening, simplified geometry or role coalescing:
  rejected as violating the authorized ordered-representation/role contract.
- Replacing SEL merge with a different sorter or partitioning unions: merge
  comparisons are substantial, but emitting all inversion pairs and preserving
  equal-x ordering/output provenance remain unproved. No replacement, component
  concatenation or sort stability claim was tested; not recommended for adoption.

## Validation, restoration and limits

Executed narrow diagnostic command, with `set -o pipefail` and combined output
tee'd to `target/test-output.log`:

```bash
T55_EVIDENCE='D:/slicerProject/pinch_n_print_cli_2/docs/specs/perf-vs-orca/evidence/t55-body-union' cargo test --manifest-path .local-artifacts/perimeter-reference-preparation/t55-body-union/harness/Cargo.toml --offline --locked --release --bin t55-native-diagnostic -- t55_full_original_role_chain_and_union_internals --nocapture 2>&1 | tee target/test-output.log
```

The harness explicitly enables `slicer-core/host-algos`; it does not depend on
a misleading feature-gated zero-test pass. No full workspace suite was run.
No production changes were committed, so production commit gates were not run.

Checkout-only verification, requiring Python but no models, raw outputs, Cargo
registry, guest builds or harness executable:

```bash
python docs/specs/perf-vs-orca/evidence/t55-body-union/verify.py
python docs/specs/perf-vs-orca/evidence/t55-body-union/verifier-controls.py
```

Both commands passed. They verify streamed specimen framing/hashes/coverage,
mode equality, saved output qualification, exact numeric replay regeneration,
native report/log agreement, phase nesting, negative-control counts, pinned
provenance and sealed evidence. They do not rerun native algorithms; fresh
native reproduction needs this repository's build inputs and toolchain.

`restoration.json` records original source/artifact restoration and both
freshness exits 0. After private native diagnosis, `finalize.py` rechecked original
source and artifact bytes, full Developer distribution file sets, tracked build
inputs, baseline/probe snapshot inventories, unchanged registry and unchanged
t53 evidence. Independent final ordinary and accelerated freshness commands
both exited **0**, as recorded in `validation.json`; clean checker logs are
empty, so the recorded exit status—not text grep—is the evidence. No production
Rust diff remains. Main-session tracker/map and unrelated working-tree changes
were preserved untouched. Only t55 evidence and private diagnostic artifacts
were added; the diagnostic leaves cache/build outputs and archived logs on disk.

Limits: eight selected full-role cases, not all-layer/full-job native replay;
BaseInterface is empty in these cases; simplification is inactive (average nodes
5/9), so positive simplification is not covered. Hole-bearing union outputs are
covered, but selected carved inputs have no holes. No optimized cache execution,
global parity claim, robust timing distribution, memory/cost measurement or
candidate speed comparison is claimed. Further implementation/adoption needs
separate authorization and exactness/fallback tests for the cache invalidation
hazards above.
