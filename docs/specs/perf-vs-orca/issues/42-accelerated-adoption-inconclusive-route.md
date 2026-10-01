# Accelerated adoption route after inconclusive campaign

Type: grilling
Status: resolved
Assignee: current OpenCode session (human route recorded)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 38

## Question

After the single authorized campaign stopped inconclusively on supports-off
Benchy Arachne, should accelerated adoption be parked with ordinary retained as
production, or should a separately scoped investigation seek a route to passing
the unchanged gate?

Authority:
[Accelerated perimeter-spatial adoption decision](13-accelerated-adoption-decision.md)
requires inconclusive results to return to the human, without an automatic retry
or weakened gate. The
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md)
has completed its authorized attempt, not passed adoption.

## Evidence for the decision

The supports-off Classic cell passed output/status checks and separated CPU and
wall ranges. The supports-off Arachne cell passed output/status checks and
separated CPU ranges, but ordinary wall samples span 16.5563–17.4910 s and
accelerated samples span 16.9912–17.1664 s. Their overlap is inconclusive under
the existing rule. Four support-enabled cells remain unrun; a Classic-only win
does not establish production-wide acceptance.

[Main campaign adjudication](../evidence/t38-adoption-resumed/campaign/attempt-1/MAIN-ADJUDICATION.md)
links the actual runner summary, retained samples, provenance and post-run
identity/evidence verification. Generated helper `BLOCKED` text is a documented
reporting-key error, not the actual campaign verdict; its original artifacts
remain preserved.

## Boundaries

- Ordinary remains the actual default pending the human's decision and any
  required acceptance; no commit or switch is authorized here.
- Do not retry the campaign, run the four skipped cells, change thresholds or
  discard samples merely to turn this inconclusive attempt green.
- Any investigation or further campaign needs its own explicit scope and
  authorization, retaining this attempt and the frozen inputs as evidence.
- A separately authorized reporting-helper regression can address the copied-
  status key mismatch without new timing or changes to the acceptance rule.

## Answer — investigation selected

The human chose **Investigate Arachne**, accepting the saved-evidence-first
scope offered in this session. Adoption is not parked, but ordinary remains
production. No retry, skipped cell, rule change, commit or switch is authorized.

The saved-evidence investigation localized the finishing tail to global layers
1 and 2 in every retained Arachne sample. CPU separates favorably, but the
observed wall savings barely reach that tail, and other phase changes offset
the small per-layer mean gain. No module-level cause is yet proved because
the acceptance captures omit stage/module timing. Original evidence hashes
were rechecked without changing the old artifacts.

Evidence: [saved Arachne investigation](../evidence/t42-arachne-investigation/FINDINGS.md).
The bounded next step, [critical-tail module attribution](43-arachne-critical-tail-module-attribution.md),
was subsequently authorized and completed with exactly two diagnostic slices.
It identified infill-linker dispatch as the tail owner; its guest/host subcosts
remain unmeasured. This was not an acceptance restart or implementation change.
