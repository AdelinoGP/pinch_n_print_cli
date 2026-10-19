# Perimeter attribution study

Type: task
Status: resolved

## Question

Where does each external perimeter generator's time go — classic and Arachne —
before any optimization is proposed?

## Answer

Attribution study 2026-09-06/07 (`docs/specs/perf-vs-orca/evidence/perf-perimeters-study/FINDINGS.md`),
supports-off benchy, three probe-free runs per generator after guest freshness
exit 0:

- Classic (temporary ADR-0055 user scopes, fuel shares): **85.8%** of module
  fuel in wall assembly, **8.0%** in gap fill.
- Arachne (host probe): preprocess/graph construction **75.3%** of the
  enclosing pipeline interval; all named stages sum to 92.3%, leaving a
  measured 364.932 ms remainder. Input/output conversion measured separately.

Attribution only: no optimization implemented and no permanent probe left in
the tree.

**Currency caveat:** these shares predate the
[Wall-flags annotation-free fast path](07-wall-flags-annotation-fast-path.md),
which removed the annotation-free scans. Re-attribute against the committed
fast path before trusting the shares at HEAD (also superseded in part by
[emit_walls premise falsified](08-emit-walls-premise-falsified.md) for classic
internals).
