# Fastpath quiet-validation handoff

**CORRECTION — runtime generator selection was not verified correctly.** Both
quiet harness scripts reused Classic configuration for labels named Arachne.
Those emitted G-code files identify Classic and their stderr drops Arachne's
claim. The Arachne-specific quiet conclusions below are invalid. Diagnosis
proves module availability, not selection. Earlier interleaved Arachne captures
are correctly configured. The user subsequently accepted and committed the
candidate for CPU savings, without claiming a wall win; see
`tmp/PERF-HANDOFF.md` §13 for the final disposition and continuation guidance.

## Controlled identity

- Freshness: `cargo xtask build-guests --check`, exit 0.
- Baseline executable: `tmp/perf-flags/baseline-artifacts/pnp_cli.exe`, SHA-256
  `5e6ec032ffaa576e017070c8acb69b0f1f9be37e2c0e266cd0de6c64837d2e27`.
- Candidate executable: `target/release/pnp_cli.exe`, SHA-256
  `4fbc1280be3699cc979fec6604320b82e6f513643075f7bb67c754ab489647d6`.
- Baseline modules: `tmp/perf-flags/baseline-artifacts/core-modules`.
- Candidate modules: `modules/core-modules`.
- Classic perimeter guest hashes, baseline/candidate:
  `f7b0cd52dfbf8257f5071c7bcd8e98e6d431518328374c15885019abdea9a02a` /
  `7be78bd05f96a0ed71772e7fa8363f9449025906e1d9a764b0fcd3fb356cafed`.
- Arachne perimeter guest hashes, baseline/candidate:
  `ce39fbe63a64b3dc93093929b16a5dd260236e5fb7dd52b37ba2136e185e43ae` /
  `fa099dd3ee8f3363b23a9bd14c8b7e765f487a303a3f0130fadc028bdf2e646a`.
- Both diagnosis manifests passed with 23 external modules and identical
  selected classic/Arachne dispatch.

## Protocol and result

`tmp/alloc-bench/run_bench.ps1`'s `-Warmup` switch labels a row but does not
skip timing, so explicit warmup invocations were used. Measured runs used
ABBA BAAB, no report, no instrumentation, and 12 Rayon workers. Raw files are
`quiet-validation/warmups.csv`, `measured.csv`, `supports-on-warmups.csv`, and
`supports-on-measured.csv`.

- **FACT CPU — Classic: PASS/favorable.** Supports-off ratio 0.9139;
  supports-on ratio 0.9268.
- **FACT WALL — Classic: INCONCLUSIVE.** Supports-off ratio 1.0114;
  supports-on ratio 1.0174; observed ranges overlap.
- **FACT CPU — Arachne: PASS/favorable.** Supports-off ratio 0.9143;
  supports-on ratio 0.9201.
- **FACT WALL — Arachne: INCONCLUSIVE.** Supports-off ratio 0.9760;
  supports-on ratio 0.9745; observed ranges overlap.

All 32 measured runs exited 0; stderr `slice_complete` records showed zero
fatal and zero non-fatal errors. Peak working set had no consistent direction.
External-load snapshots recorded CPU load 12% before and 7% after, with Opera
and Discord among top processes; this does not prove continuous idle time.

Recommendation: **INCONCLUSIVE awaiting USER decision**. Keep the candidate
available for review; do not auto-drop it, and do not claim controlled wall
acceptance. No source change, commit, or discard was performed.
