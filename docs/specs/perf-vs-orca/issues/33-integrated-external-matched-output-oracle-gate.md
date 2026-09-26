# Integrated/external matched-output oracle gate

Type: task
Status: open
Blocked by: 23

## Question

After [Integrated-parity oracle experiment](23-integrated-parity-oracle.md)'s
native view repairs, is the integrated path output-equivalent to external/WASM
for the map's matched job, such that integrated timings can inform module
performance work? Do not assume so from narrow projection or single-module
tests: the earlier all-integrated run emitted different G-code TYPE counts
(`docs/specs/perf-vs-orca/evidence/perf-split/FINDINGS.md` §Finding 3).

Run matched integrated and external slices from complete, version-pinned
snapshots on the same benchy/base configuration, beginning with the formerly
divergent infill/bridge/surface classes. Record actual module provenance,
effective config, diagnostics/degraded status, TYPE counts, output bytes, and
geometry-relevant output differences per mode. Verify guest freshness before
attributing any mismatch. If the outputs differ, localize the first differing
stage or view, repair its native/WASM parity, and repeat the output gate;
otherwise explicitly bound the equivalence claim to the cells examined. Only
then may a separate ordinary/accelerated timing comparison use integrated
runs as a module-work oracle. Integrated execution must never be substituted
for the external matched-pair verdict itself.

AFK and parallel-takeable; this is a correctness/measurement qualification,
not authorization to auto-commit a performance candidate.
