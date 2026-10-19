# Unreachable batch-query dead end

Type: task
Status: resolved

## Question

Should the brute-force O(triangles) linear scans in `raycast_z_down_batch` and
`surface_normal_at_batch` be spatially indexed?

## Answer

No — **do not pursue** (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §6 dead-end list). The two batch
queries have **no production callers** — only SDK plumbing and test mocks — so
spatial-indexing them optimizes an unreached path. They look like obvious
targets precisely because their complexity is visible; reachability trumps
complexity. Reconsider only if a production caller appears.
