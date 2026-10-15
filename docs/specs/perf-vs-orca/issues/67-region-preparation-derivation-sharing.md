# Region-preparation derivation sharing across region projections

Type: task
Label: wayfinder:task
Status: claimed
Assignee: claimed 2026-10-15 — session resumed on the human's "continue"
(no ticket named), so it took the first frontier ticket in order per the map's
standing selection rule. Graduated by the 2026-10-14
[host:native attribution](60-host-native-attribution.md) take
([findings](../evidence/t60-host-native/FINDINGS.md)).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Implement and measure sharing the per-layer region-preparation derivation
between the two `LayerArena` projections, so the matched jobs no longer compute
the same per-region data twice per layer.

The t60 take measured the two derivation slots — `ensure_prepared_regions`
(ordinary) and `ensure_prepared_perimeter_source_regions` (perimeter-source),
both `slicer-wasm-host/src/marshal/prepared.rs::prepare_regions` — at 661 clip
calls per benchy layer and 1,926 per base layer **each**, once per layer, for
**40.53 s + 40.18 s accumulated on base-off** (≈46 % of the then-`host:native`
bucket), 39.42 s + 40.06 s base-on, ≈1.48 s + 1.47 s benchy cells
([findings](../evidence/t60-host-native/FINDINGS.md), `captures/`).

On the matched jobs the two projections operate on **the same region sets with
the same inputs**: `marshal/mod.rs::perimeter_source_regions` returns
`slice.regions` minus modifier-namespace entries (restoring modifier children's
fields when present), and the matched configs author no modifier sub-regions —
so the two derivations are the same computation on identical input. The
duplication exists because the projections are general (they differ when
modifier sub-regions exist) and the two slots are cached separately, though
`take_slice`/`set_slice` invalidate both together.

Candidate shape: key the per-region derived data (`derive_needs_support`,
surface group, quartile-band clips, previous-layer boundary) by region
identity and share it across the two projections — deriving once and adapting
for the other projection when the region sets coincide, keeping the separate
derivation where they differ (modifier sub-regions). Exactness is testable:
the derivation is pure, so shared data must be value-identical to the current
per-projection results; the equality of the two projections' outputs on the
matched jobs is itself checkable before sharing.

Acceptance: independent exactness qualification (prepared data value-identity
across the two projections, including the modifier-sub-region case) plus the
standing paired ordinary + accelerated A/B on the pre-declared cells
(6 retained pairs per mode, zero starvation exclusions), keep/drop to the
human, no auto-commit.

## Notes

- The cache lives in `crates/slicer-runtime/src/blackboard.rs`
  (`LayerArena`; `take_slice` clears both slots, `set_slice` starts them
  unprepared); derivation call sites are `prepare_dispatch_region_views`
  (`crates/slicer-runtime/src/layer_executor.rs`).
- The two slots are consumed by different stage families (`Layer::Perimeters`
  uses the perimeter-source projection; `Layer::Infill`,
  `Layer::SlicePostProcess`, `Layer::Support`, `Layer::AnchoredEvents`,
  `Layer::SupportPostProcess` use ordinary regions), so a shared core must
  keep both projection views intact for the adapters.
- Do not conflate with the sibling named leads from the same take: the
  classic-perimeters host-service round trips (~89 s accumulated base-off) and
  the `region_partition` 3×-per-layer re-run (~10 s) are separate questions,
  recorded in the t60 findings and the map's fog; they are not this ticket.
- Evidence: [findings](../evidence/t60-host-native/FINDINGS.md), owner table
  and per-scope reconciliation in `summary.json` / `captures/`, re-verifiable
  with `verify.py` (exit 0 from the committed set).

## Comments

### Take pre-declaration — 2026-10-15 (recorded before any measurement)

**Selection.** Resumed on the human's "continue" (no ticket named), so the
session took the first frontier ticket in order per the map's standing
selection rule; the human then chose the timed-cell set below when asked
before any machine time was committed (ticket creation is not execution
approval; the ticket was recorded as awaiting the human's route decision).

**Candidate.** Derivation sharing in the arena, not a new cache:
`LayerArena::ensure_prepared_perimeter_source_regions`
(`crates/slicer-runtime/src/blackboard.rs`) now serves the perimeter-source
consumer from the ordinary derivation when the two projections coincide, and
the ordinary slot's allocation is the one shared (no clone, no second
derivation). The coincidence condition is
`slicer_wasm_host::marshal::perimeter_source_projection_is_identity`
(`crates/slicer-wasm-host/src/marshal/mod.rs`): no `MODIFIER_FOOTPRINT_REGION_ID`
carrier and no modifier-namespace child in `slice.regions`, which is exactly
when `perimeter_source_regions` returns the region list unchanged (same
regions, same order, byte-identical inputs). Exactness argument:
`prepare_regions` is pure over `(regions, surface_classification,
global_layer_index)`, so identical inputs imply identical `PreparedRegionData`;
when the condition is false the two projections keep today's separate
derivations verbatim. The sharing decision is cleared with the slots it
describes (`clear_prepared_regions` from `set_slice`, `take_slice`, `reset`).

**Tests (candidate tree).** Five new contract tests
(`crates/slicer-runtime/tests/contract/region_preparation_sharing_tdd.rs`):
sharing both call orders pinned on allocation identity
(`std::ptr::eq` — a second derivation cannot fake it), the restored-geometry
case where a wrongly shared derivation would report the ordinary answer, the
invalidation of the decision with its slice, and the no-slice `None` contract;
plus one `slicer-wasm-host` unit test pinning that the condition holds exactly
when the projection is the identity. Mutation check: forcing the condition
false fails 3 of the 5 contract tests. `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo xtask check-literals` and
`cargo xtask check-test-quality --report` are clean.

**Qualification (before timing).** The 16-cell output-identity matrix —
{benchy, base} × {classic, arachne} × {supports off, on} × {ordinary,
accelerated} — baseline and candidate once per cell; outputs must be
byte-identical to each other and to the preserved t58 references; clean
completion (`ok`, not degraded, zero fatal/non-fatal), non-vacuous
wall/infill/support role counts.

**Timed cells (the standing four-cell set, human-selected 2026-10-15).**
`ordinary-benchy-classic-on`, `accelerated-benchy-classic-on`,
`ordinary-base-classic-on`, `accelerated-base-classic-on`. Rationale: the
shared derivation is host-side and measured generator-invariant (t60: the two
slots are the arena's region projections), so the arachne-on cells carry the
same effect class; their outputs remain covered by the 16-cell qualification.
Any extension to the arachne or supports-off cells is a new pre-declaration.

**Measured expectation (stated so the result can contradict it).** t60's
attribution puts the duplicate derivation at 40.53 s + 40.18 s accumulated on
base-classic-off, 39.42 s + 40.06 s base-on and ≈1.5 s + 1.5 s on the benchy
cells, so the candidate removes ≈40 s of host thread-time on base and ≈1.5 s
on benchy, once per layer. At the t63-measured 2.4–7.0 % per-layer-phase
scheduling deficit the phase is largely thread-packed, which predicts
thread-time/12 ≈ 3.3 s (≈1.5 %) on `ordinary-base-classic-off` (218 s wall,
t66 qualification) and ≈0.7–1.5 % on the two base supports-on cells, with the
benchy cells at the ~0.1–0.5 % floor of the paired design or below it; a
latency-bound phase would show more, never less. Wall claims are read only
from the uninstrumented campaign.

**Protocol.** Per timed cell: 1 proof pair (baseline + candidate once, output
hashes must match), then 1 warm-up per arm, then 6 baseline/candidate pairs
with odd pairs baseline-first and even pairs candidate-first; 12 rayon
threads; process wall and CPU via `GetProcessTimes`; a whole pair is excluded
when either arm's cpu/wall ratio falls below half that arm's median (never
resampled; fewer than 6 retained pairs = inconclusive, not a pass); paired
medians of retained per-pair ratios; gate = wall delta < 0 % AND CPU delta
≤ 0 % in every timed cell.

**Builds.** Baseline = HEAD snapshot, candidate = HEAD + this take's diff,
each built as complete ordinary (`cargo xtask dist --edition developer`) and
accelerated (`--accelerated --edition developer`) snapshots; runs use
`--module-dir <snapshot>/modules --no-default-module-paths
--no-integrated-modules` with `SLICER_MODULE_PATH` cleared; `cargo xtask
build-guests --check` must exit 0 in the arm's tree before its build.

**Artifacts.** Scripts and reducers:
`docs/specs/perf-vs-orca/evidence/t67-region-prep-sharing/`; raw runs and
snapshots under the ignored
`.local-artifacts/perimeter-reference-preparation/t67-region-prep-sharing/`.
The candidate stays uncommitted until the human's KEEP/DROP; both guest
freshness gates must be green at the candidate tree.

### Take paused — 2026-10-15 (human parked the take before the campaign ran)

**State at the pause.** Candidate tree = HEAD `d5ff9d26` + the uncommitted
diff (`crates/slicer-runtime/src/blackboard.rs`,
`crates/slicer-runtime/tests/contract/main.rs`,
`crates/slicer-wasm-host/src/marshal/mod.rs`, plus the untracked
`crates/slicer-runtime/tests/contract/region_preparation_sharing_tdd.rs`),
also held in `stash@{0}` ("t67 candidate (region-preparation derivation
sharing)"). Contract tests, the marshal unit test and the mutation check pass;
clippy/literals/test-quality clean; `cargo xtask build-guests --check` exit 0
in both modes at the candidate tree. All four snapshots are built and hashed
under `.local-artifacts/perimeter-reference-preparation/t67-region-prep-sharing/snapshots/`
(baseline = HEAD tree, candidate = HEAD + diff; ordinary and accelerated;
`hashes-baseline.txt` / `hashes-candidate.txt`).

**Nothing was measured**: no proof run, no qualification, no timing. Raw tree
contains only `snapshots/`.

**Resume.** From the repo root, with the candidate restored in the working
tree: (1) `pwsh -File docs/specs/perf-vs-orca/evidence/t67-region-prep-sharing/launch-detached.ps1 -Phase proof`
(16-cell proof driver, resumable via run-cell's one-shot guard); (2)
`python docs/specs/perf-vs-orca/evidence/t67-region-prep-sharing/build-qualification.py`
then `python .../verify-references.py` (both must pass before timing); (3)
`launch-detached.ps1 -Phase measure`; (4)
`python .../reduce_ab.py`. Nothing needs rebuilding unless sources change;
re-run `build-snapshots.sh <arm>` for the arm whose tree changed.


