# Current-job adoption reference preparation policy

Type: grilling
Status: resolved
Assignee: agpen (OpenCode session)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

How should the adoption corpus acquire trustworthy precomputed references for
the current supported job after the recent correctness repairs: independently
prepared references supplied by the human, or an explicitly authorized,
provenance-recorded ordinary-only refresh frozen before accelerated validation?

The on-disk corpus exists and its models match the map fixtures. Its reference
completion events are historical and predate the support-correctness and
empty-infill-replacement repairs; both base references record degraded output.
Current-job reference equivalence has not been measured. Existing references
must not be silently replaced or generated from candidate output.

Facts and input hashes:
[Accelerated adoption campaign preflight](../evidence/t38-adoption-preflight/PREFLIGHT.md)
and [corpus inventory](../evidence/t38-adoption-preflight/corpus-inventory.json).

Preserve the distinction between the six-cell ordinary-versus-accelerated
adoption corpus and the map's separate eight-cell matched Orca scoreboard.
Decide who prepares the references, which supported configuration is held
fixed, and what independent correctness/provenance checks are required before
timing. Reference regeneration is not permission to relax exactness thresholds,
bless a known defect, or switch production defaults.

This decision unblocks
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md).

## Progress comment — 2026-09-30, human-confirmed policy

The human authorized **ordinary-only preparation in this session**, after
confirming the following policy. Preparation is not campaign timing or adoption
acceptance; no commit or default change is authorized.

- Preserve all six adoption models/configs byte-for-byte. Keep the historical
  corpus untouched; prepare a separate current-job corpus, explicitly named for
  subsequent use. Do not substitute the eight-cell Orca scoreboard job.
- The actual implicit fill job is **20% rectilinear**, not 25% fill:
  `ResolvedConfig::infill_density` and `ResolvedConfig::sparse_fill_holder`
  (`crates/slicer-ir/src/resolved_config.rs`) default to `0.2` and
  `rectilinear-infill`. The explicit `sparse_infill_density: 25` is a separate
  perimeter-module percentage, as documented by `ORCA_CONFIG_ALIAS_KEYS`
  (`crates/slicer-gcode/src/serialize.rs`). The human confirmed preserving this
  actual job after this distinction was surfaced. Record effective settings;
  stop on unresolved or conflicting settings rather than altering configs.
- Use a fresh ordinary developer host-plus-guest snapshot and 12 threads.
  Verify exact compiler identity and ordinary guest freshness; retain snapshot
  executable/module hashes, policy identity, source revision and dirty-tree
  evidence, exact commands, input/output hashes and completion sidecars.
- Require successful, non-degraded, zero-fatal and zero-non-fatal outputs in
  every cell, with matching config/G-code/scheduler generator evidence. Run the
  narrow support-repair and empty-infill contract regressions. Require a fresh
  sparse-infill overshoot check on each output, explicitly limited to its
  layer's wall bounding box, not full polygon containment or Orca parity.
- Freeze the complete validated reference set and provenance before any
  accelerated comparison. Ordinary-generated references establish ordinary
  output equivalence, not independent geometric correctness. Any prerequisite
  failure stops preparation; no gate weakening or automatic retry.

At this policy checkpoint execution was pending. Full packet/adoption verification, both campaign
snapshots, real exactness comparisons and campaign timings remain pending.

## Resolution comment — 2026-09-30

**Policy confirmed and ordinary references prepared/frozen in this session.**
The human's policy above governs preparation; no accelerated output supplied a
reference. Historical models/configs/references are preserved unchanged, and
the separate current-job corpus contains all six clean ordinary references.
All explicit settings and the actual implicit 20% rectilinear fill job were
verified from the outputs. Every cell passed config/G-code/scheduler generator
evidence, successful non-degraded zero-fatal/zero-non-fatal completion and the
limited deposited-wall-bounding-box sparse overshoot check. Narrow support and
empty-infill producer/runtime regressions passed.

The first preparation attempt stopped before slicing on a Windows manifest-key
bookkeeping bug. Its evidence was preserved; the human explicitly authorized
the separately recorded new attempt after a red/green path regression. No
automatic retry or gate weakening occurred.

Evidence and usage:
[Current-job reference preparation findings](../evidence/t40-reference-preparation/FINDINGS.md),
[frozen reference/snapshot manifest](../evidence/t40-reference-preparation/attempt-2/frozen-manifest.json),
and [read-only freeze verification](../evidence/t40-reference-preparation/attempt-2/freeze-verification.json).
The findings give the verifier command; it must pass before accelerated
comparison and again after a campaign. Missing or changed frozen inputs are a
blocker, not permission to regenerate them. Subsequent acceptance must use the
manifest's new corpus root explicitly; the default corpus intentionally still
holds historical references. Re-establish the actual campaign module set,
mode-specific snapshots/freshness and all remaining packet gates when taking
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md).

This preparation proves ordinary-output reference identity and targeted defect
checks, not full geometric correctness, accelerated exactness or adoption
acceptance. Full adoption gates and timing remain pending. No production code,
config, threshold or default changed; no commit was made. Ordinary remains the
actual default. Generated ordinary/debug build artifacts were refreshed; prior
dist output and both preparation attempts are retained under the target tree.
