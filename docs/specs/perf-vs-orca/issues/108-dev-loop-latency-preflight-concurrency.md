# Dev-loop latency: the `xtask test` gate, and what actually costs time

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; **KEEP** — the three `xtask test` preflights now run
concurrently; a representative invocation falls from a median 18.77 s to
10.80 s (−7.97 s, −42.4 %, four interleaved pairs, non-overlapping arms). Four
plausible levers were priced and falsified, and two adjacent harness defects are
recorded — see ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human said "continue the
perf vs orca map, create and complete a ticket focused on improving Testing and
Compilation speed", quoting a discussion of Rust iteration-speed practice
(ra-jit/Blaze, mold/lld, sccache/rust-cache, cargo-nextest, selective
`opt-level`, CI caching and agent-loop discipline)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the human's 2026-11-02 instruction. This is a **human-directed
scope extension** of an effort whose destination is slice throughput: the dev
loop's compile/test latency is now an axis of this map by explicit ruling, and
the extension is recorded in the map's Notes. Nothing about the slice
destination, its floor or its KEEP/DROP rules changes.

## Question

**Where does an edit → verify cycle's wall time actually go in this repository,
and which levers from current Rust iteration-speed practice clear a pre-declared
floor without changing what the gates check?**

The take has to separate four surfaces that the discussion conflates:

1. **The gate** — what `cargo xtask test` pays before any test runs.
2. **The compile path** — a warm no-op, a one-line edit to a widely-depended-on
   crate, and the first object-code build of a bucketed test target.
3. **The test run** — per-bucket execution.
4. **The toolchain levers** — artifact sharing between `check` and `clippy`,
   feature-set agreement between documented commands, linker substitution,
   debug-info volume, `cargo-nextest`, `sccache`, CI caching.

Premises to verify, not assume:

- That `cargo check` and `cargo clippy` pay a full re-check when alternated.
  (The repo's commit gate is clippy, while `AGENTS.md` also prescribes a
  check-only tier, so an artifact split would be a real tax.)
- That the documented narrow command and the gated entry point disagree on
  features, forcing a rebuild on every switch.
- That linking is a large share of the local loop, making lld/mold a big lever.
- That debug-info volume is what makes the rebuild slow (the contract artifact
  is an 89.7 MB binary beside a 770 MB PDB).

## Method (pre-declared; deviations in ## Answer)

- **Floor.** A lever is adopted only if it cuts **≥ 5 s** from a measured
  `xtask test` invocation (or ≥ 10 % of a measured compile path), measured as a
  paired median over **≥ 4 interleaved samples**, with the arm ranges
  non-overlapping. The floor is roughly a third of the serial gate's own cost;
  it is chosen before measuring so a 1 s win cannot be dressed up as a route.
- **Frozen harness.** The candidate changes the harness that times itself, so
  each arm is a harness binary built from the same HEAD (`git stash` for
  `before`, applied for `after`) and invoked directly. `cargo xtask` rebuilds
  inside the timed window and would contaminate the sample.
- **Fixed payload.** `cargo xtask test -p slicer-runtime --test contract
  authored_tool_index_tdd --summary` — 4 tests, 0.00 s execution, no rebuild — so
  everything that moves is the gate.
- **Inventory is inventory.** Warm no-ops, per-edit checks, bucket build and
  link numbers, artifact sizes and bucket execution are recorded as measured
  facts, never promoted to gated claims.
- **Falsify before building.** Each toolchain premise is priced with the
  smallest command that can refute it. A refuted premise is recorded as dead
  rather than quietly dropped.

## Stop rule

If the concurrent-preflight candidate misses the floor, revert it and close as
measured-dead with the inventory as the take's output. If it clears the floor,
adopt it, keep every gate's verdict and reporting order, and land it with the
existing `cargo test -p xtask` suite plus one test that fails if the preflights
stop overlapping.

## Answer

**Verdict: KEEP.** The gate was serial work that did not need to be serial.
`test_command` now builds the three preflight closures and hands them to
`run_preflights_with` (`xtask/src/test.rs`), which spawns `check-literals` and
`check-test-quality` on scoped threads and runs the guest-freshness check on the
calling thread, so all three overlap. The freshness check stays on the calling
thread because `Invocation` owns a non-`Send` canonical-WIT loader closure — and
keeping it there preserves the single-toolchain-probe contract for a possible
stale rebuild. Reporting order is unchanged: a literals or test-quality failure
still aborts **before** any guest rebuild.

| round | before (s) | after (s) | delta (s) |
| --- | ---: | ---: | ---: |
| 1 | 18.438 | 12.179 | −6.259 |
| 2 | 19.034 | 13.214 | −5.820 |
| 3 | 23.374 | 9.424 | −13.950 |
| 4 | 18.507 | 8.961 | −9.546 |
| **median** | **18.771** | **10.802** | **−7.969 (−42.4 %)** |

The arms do not overlap: the slowest `after` sample (13.214 s) is faster than
the fastest `before` sample (18.438 s). The pre-declared 5 s floor is cleared
1.6x. The gate's own components measure 8.196 / 5.621 / 2.731 s (round 1) and
9.375 / 7.686 / 4.543 s (round 2), serial sums 16.548 / 21.604 s — so the
achievable saving is `sum − max` = 8.35 / 12.23 s, and the measured end-to-end
delta lands inside it.

**The dev loop's real weight is codegen, not the gate.** A warm no-op
`cargo check --workspace --all-targets` is 0.81–2.35 s; a one-line edit to
`crates/slicer-ir/src/lib.rs` costs 65.3 s and to `crates/slicer-core/src/lib.rs`
72.0 s; and the first object-code build of the `contract` bucket after
metadata-only builds costs **994 s**, against **17.26 s** for an incremental
rebuild of the same binary. That is the shape the suite's documented ≥11-minute
cost comes from, and no gate-level change touches it.

**Four premises falsified, so the take does not spend its budget on them:**

1. **`check`/`clippy` artifact split — dead.** With both warm, an alternation in
   either direction takes 1.7–2.4 s and prints **zero** `Checking` lines. The
   129 s / 138 s readings that suggested a split were a dirty `slicer-runtime`
   test target left by a reverted probe edit.
2. **Feature-set thrash between `xtask test` and the documented narrow command —
   dead for `slicer-runtime`.** A plain feature-less `--no-run` immediately after
   a host-algos `xtask test` run finished in 0.72 s with no rebuild; the package
   already enables `slicer-core/host-algos` transitively.
3. **Linker — dead for incremental relinks.** A relink-only pass costs 12.8 s;
   codegen owns the rebuild. Cold-build link share is not claimed.
4. **Debug-info volume — not a win on the path an agent pays.** The same
   incremental rebuild measured 17.26 s at the default `debug = 2` and 28.24 s
   with `profile.test.package.slicer-runtime.debug = 0`.

**Adjacent findings (recorded, not fixed here).**

- **A1.** `test_command` appends `--features slicer-core/host-algos`
  unconditionally when the caller set none, so `cargo xtask test -p xtask`
  (and `-p slicer-config`, `-p slicer-ir`, `-p slicer-scheduler`,
  `-p slicer-gcode`, `-p slicer-helpers`) fails with
  `the package '…' does not contain this feature`. The injection is load-bearing
  only where `slicer-core` is the tested package.
- **A2.** `AGENTS.md`'s narrow example filter
  `core_module_ir_access_contract_tdd` matches **no test** in the contract
  bucket (`0 passed; 314 filtered out`) — the documented command builds the
  bucket and runs nothing, which reads as green.

**Ruled out on platform: blaze (ra-jit).** The discussion's headline experimental
option — rust-analyzer's query database plus a Cranelift code generator, holding
the project compiled in memory — **cannot run on this host**. Upstream's README
("Good to know") supports macOS on Apple silicon and Linux only: "Windows has no
installer, and v2 has not been run there." The take's host is
`x86_64-pc-windows-msvc`, so no build or test wall reported here can move because
of it. This is an eligibility finding, not a price: no blaze speedup is measured
or claimed. Its CI half also does not land here — the repo's CI is Ubuntu, where
the tool exists, but blaze ships its own pinned Rust nightly against this repo's
pinned 1.96.0 and controlled-perimeter accelerated build, and upstream itself says
"your toolchain and CI stay the final word". That is a reading of the vendor
docs, not a measurement.

**Not priced, named as fog:** the cold-workspace effect of `debug = 1/0`, linker
substitution on a cold full build, `cargo-nextest` on the five buckets,
`sccache`, and the CI half of the discussion (cache keys, job parallelism, path
filters, `cancel-in-progress`). The last group cannot be priced without a CI
runner, so it is not claimed.

**Verification.** `cargo test -p xtask` 188/0, including
`preflights_overlap_and_preserve_every_outcome`, which records each closure's
run interval and fails if the three stop overlapping;
`cargo clippy -p xtask --all-targets -- -D warnings` clean;
`check-literals` 0 violations; `check-test-quality --report` 0 findings.

**Disclosures.** Load was not held fixed (56 % reported at the end of the take
on 12 logical CPUs); the paired interleaving and non-overlapping ranges are why
that does not explain the delta. The 994 s codegen sample was taken once after a
series of metadata-only builds and is inventory, not a gated claim. The
debug-override comparison is one sample per arm.

Evidence: [`evidence/t108-dev-loop-latency/`](../evidence/t108-dev-loop-latency/FINDINGS.md)
(`summary.json`, `verify.py`, `REPRODUCE.md`; checkout-only verify exits 0 with
28/28 claims, `--self-test` detects 11/11 negative controls).
