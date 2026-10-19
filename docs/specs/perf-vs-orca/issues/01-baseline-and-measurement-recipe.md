# Baseline and measurement recipe

Type: task
Status: resolved

## Question

Establish the first PNP-vs-OrcaSlicer slice-throughput baseline and codify the
measurement protocol that future evidence must follow to be trustworthy.

## Answer

Done 2026-09-04 (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §1–§4).

- **Baseline captured** on three fixtures (calicat 876 tris, 3dbenchy 225,786,
  base 2,461,234 — the last two user-supplied under `tmp/`, never committed):
  OrcaSlicer 0.84 s / 6.34 s / 44.17 s vs PNP 4.81 s / 42.5 s / 11 m 19.6 s,
  with G-code sizes diverging in both directions. The configurations are
  **not matched** (Orca: BBL X1C 0.4 mm, `0.20mm Standard`; PNP: 0.5 mm,
  `support_type: tree(auto)`) — no speed multiple is defensible from this
  table, which is why the matched-pair rig
  ([Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md))
  exists as route work.
- **Recipe**: uninstrumented runs for wall-clock claims, instrumented for
  attribution only (42.5 s → 73.5 s on benchy — instrumentation is not free);
  process CPU measured externally after exit.
- **Six traps codified**, each having produced or nearly produced a wrong
  conclusion: unmatched configurations; instrumentation overhead; per-layer
  `elapsed_ms` is accumulated worker wall-clock (~8× phase wall on benchy), not
  CPU; `--module-dir modules/core-modules` is mandatory or every WASM core
  module silently falls back to integrated (no supports at all); `cargo xtask
  build-guests --check` must exit 0 before timing (exit 3 is not clean); G-code
  bytes are stable to ~500 B / 6.7 MB but base.stl wall varies ±13%, so one-run
  before/after comparisons on base are noise.
- Reproduction commands (PNP wall, attribution profile, ADR-0055 fuel profile,
  OrcaSlicer CLI with its vendor profile triple) verified in the capture
  session.

Figures above are as measured at capture; re-derive before quoting.
