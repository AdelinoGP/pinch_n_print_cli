# Handoff: 225a-host-wasi-accomodation — COMPLETED 2026-08-13

This packet closed on 2026-08-13 (second session). The session-pause handoff that
previously lived here is superseded; historical run detail is in the git history
of this file and in the evidence records.

## Outcome

- All steps done. Acceptance ceremony fully green: AC-1..AC-9, AC-N1, AC-N2,
  `cargo check`/`clippy --workspace --all-targets`, `host_services_tdd` (16 passed),
  `wit_drift_detection_tdd` (25 passed).
- Verdict: **Dragon Curve authoring language: MoonBit** (first `LOADABLE_AND_CORRECT`
  in the locked MoonBit, AssemblyScript, C++, Go order), published in
  `docs/14_submodule_programming_languages.md`.
- MoonBit: `LOADABLE_AND_CORRECT`. The earlier trap was a fixture packaging error
  (`build.sh` copied `main.mbt` into `gen/` instead of the interface package where
  `run` is declared; the `sed` patch targeted `moon.pkg` while wit-bindgen 0.60
  emits `moon.pkg.json`), not a toolchain defect.
- AssemblyScript: `LOADABLE_AND_CORRECT` via the clean fork at
  `feat/assemblyscript-no-async` (branch updated from `feat/assemblyscript-backend`
  per user direction; AC-8, requirements, plan, and gate script all amended).
- C++: `LOADABLE_AND_CORRECT` (prior session).
- Go: terminal `NOT_LOADABLE_OR_CORRECT` per explicit user decision — wit-bindgen-go
  v0.7.0 generated imports rejected by `go:wasmimport` on every viable Go toolchain;
  deviation noted in `design.md` Locked Assumptions.
- Final full spec-review: CHANGES REQUESTED on two doc-truthfulness findings
  (docs/14 stale "pending" bullet; requirements.md old branch name); both fixed and
  re-verified before the status flip. No code defects found.
- docs/07: no delta (packet 228 owns the TASK-336 row).

## Note on untracked artifacts

`docs/feasibility-probes/foreign-language-text-postprocess/moonbit/{_build,gen,interface,world}/`
are untracked generated build dirs; the records' reproduction steps regenerate them.
Recommend gitignoring rather than committing.

## Performance addendum (user-requested, 2026-08-13)

Equal-compute timing of the probe guest components, measured with a new
ignored test `foreign_language_text_postprocess_perf` in
`crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs`:
one instantiation per run (load + compile + instantiate timed together), 10
warmup calls, then 1000 timed calls of the exported `run` on the fixed probe
input (`"; probe input\n"`), via `std::time::Instant`. Because the WIT `run`
takes an owned `config-view`, the harness pushes a fresh config resource per
call (uniform across guests).

Host environment: rustc 1.96.0 (ac68faa20 2026-05-25), wasmtime crate 47.0.3,
Windows 11 Home. Runs used the cargo `test` profile (unoptimized host binary —
cargo test's default; guest code is still Cranelift-compiled natively). Two
runs per guest; both reported verbatim from the PERF lines.

| Guest | Component sha256 (verified) | Instantiate ms (r1 / r2) | Mean per-call µs (r1 / r2) |
|---|---|---|---|
| MoonBit | `51b06b5f...41512` | 26.796 / 23.593 | 20.196 / 21.636 |
| AssemblyScript | `1f3dc321...40953` | 29.869 / 24.793 | 20.348 / 19.267 |
| C++ (rebuilt) | `2abdaac1...bf483` | 214.862 / 208.332 | 18.230 / 21.828 |
| Rust baseline (`sdk-postpass-text-guest`) | `000e991c...50c4cf` | 88.542 / 95.216 | 30.475 / 29.078 |

Full hashes: MoonBit
`51b06b5f47445059c4c27ca55cac36de30c3773e4691548c5b48ef5620341512`,
AssemblyScript
`1f3dc321e1b5d6dacd830000f4d11f0489bf9c9a5cac8f36b501c226a2340953`,
C++ `2abdaac1638fa9ec2b740f7d8e1526d60571daf2794c84a51209f414b52bf483`
(rebuilt via the cpp fixture's `build.sh`; bit-identical to the hash in
`docs/feasibility-probes/cpp-text-postprocess.md`), Rust baseline
`000e991cf00fc8ca272bc706322fcfad7cc9dd86472b547376ce8679e550c4cf`.

Notes:
- **Go excluded.** Its only buildable component exports no callable instance
  (terminal `NOT_LOADABLE_OR_CORRECT`), so no number exists to report.
- **Rust baseline caveat.** `sdk-postpass-text-guest.component.wasm` exports
  the same world but its output differs from the probe oracle contract; it is
  timed for comparison only, not correctness.
- **Measurement caveat.** Wall-clock on a non-isolated Windows machine;
  figures are indicative, not rigorous benchmarking.
