# t57 isolated shared-engine cache experiment

**Status: completed; DROP accepted by the human 2026-10-07.** Run
`python docs/specs/perf-vs-orca/evidence/t57-topx-cache/verify.py` to check the
entire evidence package from a checkout: exactness provenance, the 16-cell
byte-identical qualification matrix, the paired campaign (KEEP/DROP/INCONCLUSIVE
stop discipline, output-sabotage rejection) and restoration seals. It rebuilds
and slices nothing. The campaign stopped at its first timed cell by rule — the
candidate was slower in 6 of 6 retained pairs on both wall and process CPU — so
the remaining three cells are recorded `not_run`; do not pool or rerun them
without a new, separately authorized take. The decision record is
`drop-acceptance.json` and the independent review disposition is
`review-record.json`.

The earlier `--interrupted` mode of that verifier describes the first stop only
and is superseded by the completed run; `seal-interrupted.py` preserves that
intermediate state.

The human-approved contract is
[`56-scanbeam-topx-cache-experiment-scope.md`](../../issues/56-scanbeam-topx-cache-experiment-scope.md).
This is one bounded take, not a permanent dependency change, acceleration adoption,
OrcaSlicer scoreboard, retry authorization or automatic commit.

## Artifacts and independence

Heavy models, executables, WASM components and raw G-code live in the gitignored
`.local-artifacts/perimeter-reference-preparation/t57-topx-cache/` archive.
Do not overwrite its snapshots or run directories. The `source/target` directory
is a junction to the repository's build cache; active artifacts must be restored
after experiment builds. Production tracked source and Cargo registry are never
patched.

`identity.json` binds the original current-source build inputs, compiler,
model/config inputs, pristine registry inventory, original distributions and
saved active artifacts. `candidate.patch` is applied in a *separate* source copy:
copy pristine `clipper2-rust` **1.1.0** into `.local-libs/clipper2-rust/`, preserving
its licence, then apply the patch. Its root/core manifests select the exact private
engine via an explicit path; only candidate lockfiles change. `patch-provenance.json`
records every changed file's original/result digest and the unchanged
`top_x` implementation (`.local-libs/clipper2-rust/src/engine_fns.rs`, relative to
the archived candidate source root).

The patch also contains a test-only dependency-closure witness and an isolated
compiler-output-path accommodation. `run_shim_mode`
(`xtask/src/rustc_driver.rs` in the candidate) strips a Windows verbatim prefix
only from the policy-validated `--out-dir` argument, retaining the identical
physical directory. The saved long-path probe fails with MSVC LNK1181; the
same-directory normal-path and short-verbatim controls pass. No acceleration
cfg, compiler allowlist, freshness algorithm or production geometry is changed.

## Build/graph authority

`build-candidates.py` builds complete ordinary/accelerated external Developer
snapshots and checks both mode-specific guest freshness gates. It preserves the
original failed accelerated build log. Its explicit diagnostic resume option
keeps the completed ordinary snapshot intact; it is not a timing retry.

`audit-resolutions.py` compares **each graph with its own original graph**, using
locked/offline actual Cargo metadata for native and WASM root resolutions and
each independent guest's WASM resolution. It checks unchanged nonengine
package/version/source/checksum lock inventories and normalized resolved packages,
dependency edges and features. The cache feature `topx-test-support` must be absent
from production graphs. `candidate-builds.json` records all audited graphs,
actual consumers, fresh snapshots and external module provenance.

The rejected root-lock-union check and its diagnosis remain saved. Independent
guests already have versions absent from the root lock; comparing against the
root alone is not a no-drift test. `per-graph-lock-differences.json` is the actual
per-graph comparison, and `resolution-audit-progress.json` records resolved-edge
and feature equality rather than merely an absence of changed version strings.

## Correctness support

`setup-checks.py` builds an independent pristine engine oracle, changing only its
package name and workspace sentinel. `checks.rs` executes the same cache-key,
fallback, lifetime and ordered boolean/offset workload natively and in actual
WASM via `run-wasm.cjs`. `setup-consumers.py` similarly copies unchanged
production `slicer-core` algorithms into an independently named oracle using the
pristine engine. `consumers.rs` compares exact production mesh/hole/closing-offset
and boolean wrapper results on both targets (`run-consumers.cjs`).

`setup-replay.py` / `replay.rs` replay every original role argument and exact
recorded ordered result in the selected independent t55 corpus. No area-only or
canonicalized contour equality substitutes for ordering, holes and roles.
`negative-controls.py` sabotages separate copies, requiring behavioral native
failure and an actual WASM assertion trap for stale-y reuse and a wrong cached
projection. Its workspace-repair resume preserves the initial compile blocker;
no live candidate is altered. `accounting.py` measures target-specific layout
and executed contract work in another diagnostic-only copy.

For Cargo tests, always combine stdout/stderr and tee to `target/test-output.log`,
then read/archive that file before another test. The independent harnesses have
no feature-gated empty test target; their manifests explicitly enable the engine's
diagnostic feature. Use the saved commands/logs for exact native/WASM reproductions.
`finish-gates.py` runs workspace build, workspace all-targets Clippy, literals,
test-quality report, both final candidate freshness checks, runner syntax and
synthetic reducer regressions. It writes `exactness.json` only after all required
signals have been checked.

## Qualification, timing and restoration

`qualify.py` freezes current inputs and has `run-cell.ps1` produce fresh
baseline/candidate outputs for the complete approved matrix in both modes
(`resume-qualify.py` / `resume-qualify-2.py` are the two authorized minimal
resumes; each re-proves completed cells from raw bytes and reruns only untouched
cells). `run-cell.ps1` uses only immutable, probe-free external snapshots. The
existing measurement validator proves the actual generator/claim holder and
completion status; `outputs.py` independently streams bytes, role extrusion and
warnings. Classic-supports-on outputs also check the archived current post-repair
t55 reference; the rest use freshly generated original-engine current-source
references. Both modes' baseline bytes must agree. Any unqualified output stops
the take before timing.

Only after `qualification.json` passes can `campaign.py run` execute the frozen
`protocol.json`: classic supports-on Benchy/base in both modes, whole-process
GetProcessTimes wall and kernel+user CPU, excluded warmups and alternating pairs.
Starvation is reduced separately per fixture/mode/arm, excluding whole pairs; the
KEEP/DROP/INCONCLUSIVE taxonomy never reports a starvation-inconclusive cell as
a measured DROP. The runner stops at the first non-passing cell, records unrun
cells, and never resamples; rerunning an externally interrupted in-flight cell
requires the explicit `--resume-authorized` flag and is never an automatic
retry. `campaign.py verify` re-derives the saved paired result and executes an
output-sabotage control without requiring heavy raw artifacts. `test-reducers.py`
uses explicitly **synthetic** analytic regression inputs, never performance or
qualification evidence.

Finally run `manage.py restore`: it verifies unchanged production inputs and
registry, byte-restores archived active artifacts, checks complete distribution
file sets, and requires both ordinary and accelerated freshness exits **0**.
`finalize.py` then re-verifies, re-seals `validation.json` and records the raw
measured-row manifest. No script commits, adopts a permanent dependency, switches
a default or authorizes a second take. The measured result is preserved for the
human DROP decision.
