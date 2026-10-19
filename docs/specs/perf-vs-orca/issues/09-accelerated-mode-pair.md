# Accelerated-mode pair

Type: task
Status: resolved

## Question

How do the [emit_walls premise falsified](08-emit-walls-premise-falsified.md)
shares and ranking hold up under the accelerated perimeter-spatial build
(`cargo xtask dist --accelerated`, packet 254)?

## Answer

Measured pair 2026-09-22 (`docs/specs/perf-vs-orca/evidence/perf-emit-walls/FINDINGS.md` addendum; the
controlled driver accepted the pinned toolchain):

- Total guest fuel **−35.8%** (1,133.9 B → 727.6 B); `com.core.classic-perimeters`
  −39.9%. But the hot per-vertex queries shrank only **~1.9×** — accelerated
  mode is **necessary but likely not sufficient**.
- Residual suspects: expensive wasm tree queries (u128 envelope math) and
  fallback paths firing (`bridge_arithmetic_safe`, `IndexState::Linear` — both
  in `crates/slicer-core/src/perimeter_spatial.rs`).
- Ranking **rescaled**: classic self 60.4% of classic fuel, `offset2_ex` 23.7%
  (bit-identical across modes — pure guest clipper), `com.core.infill-linker`
  15.3% (new entry — untouched by acceleration). On calicat `offset2_ex` is
  53.7% of classic fuel. Lead 3 (consume-only-when-read annotations) gains
  weight and must be scoped against accelerated mode; lead 2 rises relatively.
- Coarse output parity held (173 B on 4.29 MB; TYPE counts within known
  instability); one-run wall 27.9 s → 19.0 s under `--profile` (indicative).

Recipe additions (hard-won): accelerated runs must use the complete snapshot at
`target/dist-accelerated/developer/` — the bare artifacts dir has no manifests
and silently loads integrated modules (the §3.4 trap again), and dual
`--module-dir` does not shadow.

Scope of the verdicts (settled in the closing exchange): the clone-premise
falsification is fair in any mode (mode-independent 0.0014%); "where today's
build spends its time" is fair as ordinary-mode because ordinary is the
production default and every handoff baseline is ordinary-mode; the lead shares
were ordinary-only and are superseded by the accelerated figures above.
