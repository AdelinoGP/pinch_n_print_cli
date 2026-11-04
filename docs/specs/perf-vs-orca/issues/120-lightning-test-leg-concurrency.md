# Lightning test-leg concurrency

Type: task / measured screen
Label: wayfinder:task
Status: resolved for the test-only experiment; idle replication remains open
Parent: [Slice performance vs OrcaSlicer](../map.md)

The handoff identified independent legs within three long-running lightning
tests. Schedule those legs on the existing Rayon pool without removing any
execution or assertion, then measure the full e2e bucket.

On one shared-machine warm pair the bucket fell from **798.61 s to 700.16 s**
(−98.45 s / −12.33%), with **172 passed and one ignored** and the same calicat
hash in both arms. The machine had active browser load, so this result is a
screen, not an idle-box benchmark or a statistically established effect. The
projected 6–8 minutes was not reached. Complete conditions, gates, test rosters,
and the self-tested evidence verifier are in
[`evidence/t120-lightning-parallel-legs/`](../evidence/t120-lightning-parallel-legs/FINDINGS.md).

The separate [lightning corpus consolidation](119-lightning-corpus-consolidation.md)
is not implemented by this change. Cross-test prepass/preparation reuse remains
a prospective Phase-1 investigation, not an approved production cache.
