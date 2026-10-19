# Arachne critical-tail module attribution

Type: task
Status: resolved
Assignee: current OpenCode session
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 42

## Question

Which module/stage accounts for the long global-layer 1 and 2 execution tail
in the unchanged supports-off Benchy Arachne adoption job, and does accelerated
mode reduce that particular work?

The [saved-evidence investigation](../evidence/t42-arachne-investigation/FINDINGS.md)
found the same two longest layers in every retained sample. Most layer-interval
savings occur elsewhere, and the per-layer mean wall gain is offset by other
phase changes. The saved captures have no stage/module timing; naming graph
construction, infill linking or spatial queries as the cause would be speculation.

## Authorized scope

The human authorized this exact diagnostic pair with "Go ahead" after the
saved-evidence findings. This is not authorization for a campaign retry or fix.

- One ordinary and one accelerated diagnostic slice, run sequentially with
  `--instrument-stderr`, no fuel profiling initially, 12 threads, and exactly
  the original manifest-selected model/config and mode-specific snapshots.
- Verify the complete frozen reference/snapshot identity before and after. If
  missing, changed or unverifiable, stop; do not regenerate or rebuild silently.
- Separate durable diagnostic outputs and tracked commands/provenance/event
  reductions; preserve the original acceptance archive and its verdict.
- Rank module/stage intervals for layers 1 and 2, and prepass stage intervals.
  Report worker elapsed as worker elapsed, never as process CPU or additive
  phase wall. Identify the measured owner before proposing another probe.
- Preserve output/status/generator checks. Instrumented output/timing is
  diagnostic evidence only, not adoption acceptance.

## Not authorized

No campaign retry, four skipped cells, extra sampling loop, profile capture,
production fix, scheduling change, threshold/sample change, reference refresh,
commit or production-default switch. Any follow-up needs an explicit scope.

## Completion signal

Either identify the module/stage that occupies each low-layer tail using actual
events, or report precisely which boundary remains uninstrumented. Return one
measured next lead to the human. Adoption remains inconclusive until a separately
authorized campaign passes the unchanged full gate.

## Answer — diagnostic pair completed

The authorized ordinary and accelerated instrumented slices completed exactly
once each, with original frozen identities passing before and after, clean
status/generator checks and byte-identical reference output. Global layers 1
and 2 remained the longest. `com.core.infill-linker` in
`Layer::InfillPostProcess` occupies 90.01–96.69% of those layer intervals across
the pair; Arachne perimeter dispatch is not the dominant measured owner.

The module timing bracket includes host preparation/marshalling and guest
execution, so it does not yet identify the expensive linker sub-operation.
Promote only [low-layer linker subcost attribution](21-infill-linker-attribution.md)
as the next lead, requiring separate scope/authorization before profiling,
instrumentation or a fix. Current linker WASM hashes differ across modes; its
historical byte-identity claim cannot be assumed today.

Evidence: [diagnostic findings](../evidence/t43-critical-tail/FINDINGS.md) and
[independent raw-event verification](../evidence/t43-critical-tail/attempt-1/verification.json).
No acceptance retry, extra sample, build, production edit, commit or switch
occurred. Ordinary remains production and the adoption gate is still inconclusive.
