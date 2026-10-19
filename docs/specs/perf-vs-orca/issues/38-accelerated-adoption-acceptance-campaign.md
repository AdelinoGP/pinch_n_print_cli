# Accelerated production adoption acceptance campaign

Type: task
Status: resolved
Assignee: agpen (OpenCode campaign completed; inconclusive adoption gate)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 13, 39, 40, 41

## Question

Does a fresh ordinary-versus-accelerated controlled-build acceptance campaign
satisfy the existing production-adoption gate, or must the conditional
adoption decision return to the human?

Authority: [Accelerated perimeter-spatial adoption decision](13-accelerated-adoption-decision.md).
Follow `docs/23_controlled_perimeter_builds.md`; do not reinterpret historical
scoreboard rows as campaign acceptance.

Remaining checklist (state must be established from disk when claimed):

- Inspect the canonical packet-254 acceptance requirements and current
  controlled toolchain policy; establish required verification results.
- Verify the frozen current-job ordinary references prepared under
  [Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md).
  Run its read-only identity gate before accelerated comparison and after the
  campaign; pass the frozen manifest's new corpus root explicitly as
  `-CorpusRoot`, because the default root deliberately retains old references.
  Missing/changed frozen inputs are a blocker, not licence to regenerate them.
  Re-establish the actual loaded module sets: reference preparation isolated
  its explicit ordinary snapshot from default/integrated module fallbacks.
- Build complete ordinary and accelerated host-plus-guest snapshots; run both
  mode-specific freshness gates and require exit 0. Record provenance.
- Verify the harness copy, CSV status round-trip regression, and dry-run protocol, then run the documented
  campaign serially with exactness checked before retaining timing rows.
- Preserve the campaign's strict favorable CPU and wall range separation in
  every cell, its stop-on-failure/inconclusive policy and no automatic retry.
- Store permitted evidence under `../evidence/`; keep heavy licensed model
  fixtures untracked. Report results and any missing gates to the human.

This ticket authorizes acceptance preparation and measurement, not an
automatic production-default switch or commit. Ordinary remains the actual
default while this gate is pending. The map's eight-cell Orca comparison is
separate from this ordinary-versus-accelerated adoption campaign.

## Progress comment — 2026-09-30

**Blocked before snapshot builds or campaign timing; not resolved.** Exact
compiler identity verified, ordinary and accelerated guest freshness checks
both exited 0, harness copy comparison exited 0, and the dry run produced the
documented synthetic rejection cases. The existing corpus contains all six
input sets and its models match the map fixtures.

The real CSV reader discards `non_fatal_error_count`; an executable round-trip
probe reports `KEEP` instead of `DROP` for baseline count 1 versus candidate
count 2. This graduates
[Acceptance runner CSV status preservation](39-acceptance-runner-status-roundtrip.md).
Historical reference completion events predate the correctness repairs, and
both base references record degraded output. Current-job equivalence is not
yet measured; reference preparation authority is the separate
[Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md).

Evidence: [Accelerated adoption campaign preflight](../evidence/t38-adoption-preflight/PREFLIGHT.md).
Full packet verification tests, all-target build gates, distinct snapshot
provenance, real exactness and the timing campaign remain unrun. No production
code, reference output, gate threshold or default was changed. Claim released
while these prerequisites are open; ordinary remains the actual default.

## Progress comment — 2026-09-30, runner prerequisite repaired

[Acceptance runner CSV status preservation](39-acceptance-runner-status-roundtrip.md)
is resolved: the real producer schema now preserves non-fatal counts and
validated generator markers, with red-before/green-after PowerShell regression
evidence. Campaign preparation must include
`pwsh -NoProfile -File resources/perimeter-acceptance/test-status-roundtrip.ps1`
alongside the harness-copy and dry-run checks.

This campaign is still open and unclaimed, blocked by
[Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md).
No campaign timing or reference refresh occurred; full packet gates and
mode-specific snapshot/freshness provenance remain to be established when taken.

## Progress comment — 2026-09-30, references prepared and frozen

[Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md)
is resolved. The human authorized guarded ordinary-only preparation here,
preserving the historical corpus and all six models/configs byte-for-byte.
The actual job is 20% rectilinear fill, with the distinct explicit perimeter
`sparse_infill_density` percentage retained at 25. All six references passed
clean completion/generator/setting evidence and the limited sparse overshoot
check, with separate narrow repair-regression evidence. See
[Current-job reference preparation findings](../evidence/t40-reference-preparation/FINDINGS.md).

This campaign is now unblocked by reference preparation but remains open and
unclaimed. The frozen manifest names the new corpus and ordinary snapshot;
verify their identity and supply the new corpus root explicitly rather than
silently using historical defaults. Complete the remaining packet verification,
mode-specific campaign snapshots/freshness, loaded-module provenance and real
exactness before retaining any timing. No campaign timing or accelerated
validation ran in the preparation take; no commit or default change occurred.

## Progress comment — 2026-09-30, frozen payloads missing on resumption

**Blocked before accelerated comparison or campaign timing; not resolved.**
The mandatory read-only `main` identity gate (`verify-freeze.py` in
`../evidence/t40-reference-preparation/`) returned exit 1 at its ordinary
snapshot inventory check. A separate read-only audit confirms both the frozen
ordinary snapshot root and current-job corpus root are absent, rather than
merely containing a changed file. The original manifest/preparation script,
historical corpus and build-input inventory still match their recorded hashes.
The cause of the absent target payloads is not established.

Evidence: [Frozen-input difference audit](../evidence/t38-adoption-campaign/freeze-difference.json)
and [failed verifier output](../evidence/t38-adoption-campaign/freeze-before.stderr.log).
No regeneration, reference mutation, dist build or timing retry was attempted.
The runner-check sequence stopped at its first check; harness/status/dry-run
checks and actual module-discovery probes in that sequence did not execute.
The separately delegated packet gate lane stopped after its in-flight
`cargo check --workspace --all-targets` returned exit 0. Compiler identity was
recorded, but its independent exit code was not captured. All other delegated
gates and focused tests remain unrun; no named tests executed. Results:
[gate record](../evidence/t38-adoption-campaign/gates/results.json).
Ordinary Cargo debug/check artifacts were refreshed; no campaign snapshot or
reference payload was regenerated. This is not full gate acceptance.

Claim released; the new prerequisite is
[Missing frozen adoption inputs recovery decision](41-missing-frozen-adoption-inputs-recovery.md).
The human must choose hash-verified restoration or explicitly authorize a new,
separate ordinary-only preparation attempt. Ordinary remains the actual default;
no automatic retry, commit or production-default change is authorized.

## Progress comment — durable replacement prepared; process acceptance pending

[Missing frozen adoption inputs recovery decision](41-missing-frozen-adoption-inputs-recovery.md)
now holds a new, technically validated ordinary-only frozen set outside
`target/`. Use its new manifest and verifier only after recovery acceptance;
do not use the missing historical manifest roots or substitute the default
corpus. Evidence:
[Durable preparation findings](../evidence/t41-reference-repreparation/FINDINGS.md).

The worker fixed and reran a preflight-tool failure without required human
permission. That failure preceded any build/slice, and all subsequent checks
passed, but technical success is not permission to accept the process exception.
This campaign remains blocked by the open recovery decision until the human
accepts or rejects the already prepared set. No accelerated comparison, timing,
commit or default change ran in the recovery take; all remaining campaign gates
still apply.

## Progress comment — recovery accepted; campaign resumed

The human accepted the validated durable set and the recorded preparation-only
preflight restart exception, then instructed continuation. Recovery is resolved.
The campaign uses the new manifest and read-only verifier under
`../evidence/t41-reference-repreparation/attempt-1/`, not the missing `t40`
payload roots. All remaining packet, mode-specific artifact, loaded-module,
status/exactness and timing gates still apply. Stop at first failure or
inconclusive result; no automatic restart, commit or default switch.

## Progress comment — resumed runner checks passed

The tracked benchmark copy remains byte-identical to the evidence authority.
The real CSV status round-trip regression passed; the dry run produced the
required synthetic rejection cases with no automatic commit; the missing-corpus
control rejected before slicing. The frozen ordinary snapshot's runner-default
module discovery matches isolated discovery: the same 24 external modules, with
one explicit search root and no added platform/environment module. Default
integrated registrations are shadowed by that external set; their warnings are
visible in the retained diagnostic output.

Evidence: [Runner preflight results](../evidence/t38-adoption-resumed/runner/results.json).
These are runner/provenance controls, not timing or accelerated exactness.
The separately delegated packet verification lane is running; new mode-specific
campaign snapshots, candidate discovery and real acceptance remain pending.

## Progress comment — actual focused assertions passed; presenter conflict preserved

All-target check and Clippy, literals/report gates, mode-specific freshness and
the 28 controlled-driver tests passed. Focused core assertions passed (3 tests),
runtime capture passed (1), complete per-mode self-baselines passed (3), and the
overlap rejection passed (1). No baseline fixture was regenerated.

Two interruptions remain visible in the evidence. The main dispatch's malformed
core command was rejected before running any test; the main authorized its
syntax correction, and the corrected tests ran once. Runtime capture and
self-baseline summaries display `FAIL` despite successful Cargo exits and
passing named assertions. `collect_bare_panics` and `print_summary`
(`xtask/src/test.rs`) classify caught panic-hook lines as failed-test evidence;
`test_command` propagates the actual Cargo exit. The saved-log diagnosis also
notes that `--summary-from` renders with `succeeded=false`, so its replay verdict
is not independent proof. The capture test was not rerun, and no assertion,
fixture, production fallback, or summary renderer was modified.

This explains the presenter conflict, not the correctness of the caught
Boost fallback geometry. Actual assertions are the focused gate authority;
real campaign output/status checks remain mandatory. Evidence:
[actual focused results](../evidence/t38-adoption-resumed/gates/remaining-status.json)
and [verdict diagnosis](../evidence/t38-adoption-resumed/verdict-diagnosis/diagnosis.md).
Complete mode-specific production snapshot preparation is delegated and running;
no campaign sample has run yet.

## Resolution comment — single campaign completed; adoption inconclusive

Complete ordinary and accelerated production snapshots were built, freshness-
checked and frozen outside `target/`, with actual compiler/mode/module identity
recorded. The ordinary snapshot matches the frozen reference producer. The
documented runner controls passed again, and the real campaign ran once using
the explicitly selected durable corpus and snapshots.

Supports-off Benchy Classic passed exactness, clean status and strict favorable
CPU/wall separation (`KEEP`). Supports-off Benchy Arachne passed exactness and
clean status, with favorable CPU separation, but its wall ranges overlap
(`inconclusive`). The runner stopped there; all four support-enabled cells are
`not-run`. No retry, selective sample removal or standalone continuation ran.
This task is complete, **not an adoption acceptance**. Ordinary stays the default.

Before/after reference verification and post-run recursive snapshot/input checks
passed. A reporting helper's inconsistent copied-status key falsely labels its
generated top-level result `BLOCKED`; the main verified the preserved raw and
tracked evidence and adjudicated the actual runner result as `INCONCLUSIVE`.
Original helper/results remain unchanged. Evidence authority:
[main adjudication](../evidence/t38-adoption-resumed/campaign/attempt-1/MAIN-ADJUDICATION.md),
[real runner summary](../evidence/t38-adoption-resumed/campaign/attempt-1/campaign-summary.json),
[main saved-evidence verification](../evidence/t38-adoption-resumed/campaign/attempt-1/main-verification.json),
and [snapshot provenance](../evidence/t38-adoption-resumed/snapshots/snapshot-manifest.json).

Build/test and acceptance outputs were refreshed under `target/`; prior
acceptance evidence and complete campaign raw files are preserved in the
durable namespace. No production source, default, threshold, reference or lock
changed; no commit occurred. The incomplete adoption gate returns to the human
via [Accelerated adoption route after inconclusive campaign](42-accelerated-adoption-inconclusive-route.md).
