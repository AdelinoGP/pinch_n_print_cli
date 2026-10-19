# Adoption campaign resumption: frozen payloads missing

**BLOCKED before accelerated comparison or campaign timing.** Ordinary remains
the actual production default. No acceptance verdict or speed claim is made.

## Frozen-input verification

`main` (`docs/specs/perf-vs-orca/evidence/t40-reference-preparation/verify-freeze.py`)
returned exit 1 at the ordinary snapshot inventory assertion.
[Command/exit record](runner-results.json), [stderr](freeze-before.stderr.log).

The separate read-only `main` audit (`inspect-freeze-difference.py` in this
directory) establishes that both manifest-named roots are absent:

- `target/perimeter-reference-preparation/t40-20260930T065207Z/ordinary/`
- `target/perimeter-reference-preparation/t40-20260930T065207Z/corpus/`

The expected snapshot inventory has 49 files and the current inventory has
none; the expected current-job corpus inventory has 27 files and the current
inventory has none. The historical corpus retains all 21 recorded files with
identical hashes. All 1,158 build-input entries, the manifest digest and the
preparation-script digest match their recorded identities.
Source: [full identity differences](freeze-difference.json).
The missing target payloads' cause is unknown; this audit does not prove how or
when they disappeared, nor whether another copy exists elsewhere.

## Stop and recovery boundary

The runner-check sequence stopped at its first check. Harness-copy comparison,
CSV round-trip, synthetic dry-run, missing-corpus control and module-discovery
probes in that sequence did not execute. The separately delegated packet gate
lane stopped after `cargo check --workspace --all-targets` completed with exit
0. Its [result record](gates/results.json) and
[full check log](gates/cargo-check-workspace-all-targets.log) are preparation
evidence only, not full acceptance. Compiler identity output matches the policy,
but the initial tee pipeline did not preserve `rustc`'s independent exit code.
Clippy, literal/test-quality checks, both guest freshness checks and all focused
packet tests were not run in this take; no named tests executed.

No frozen file was changed, regenerated, re-frozen or replaced. No ordinary or
accelerated campaign snapshot was built, no slicer/timing run was launched,
and no threshold, config, default or commit changed. Tracker/evidence files were
added or updated; ordinary Cargo debug/check artifacts were refreshed by the
delegated gate lane and are not reference payloads. The check log reports
future-incompatibility warnings for `nom` and `quick-xml`, not a failed gate.

[Accelerated production adoption acceptance campaign](../../issues/38-accelerated-adoption-acceptance-campaign.md)
remains open, its claim released, and is blocked on
[Missing frozen adoption inputs recovery decision](../../issues/41-missing-frozen-adoption-inputs-recovery.md).
Recovery requires either restoration matching the original hashes or explicit
human authorization for a distinct new ordinary-only preparation attempt under
the existing policy. Do not weaken the identity gate or edit the old manifest.

## Local validation

Python AST syntax checks, JSON parsing, tracker link checks and
`git diff --check` passed. These checks do not substitute for packet gates,
mode-specific freshness, output exactness or campaign timing.
