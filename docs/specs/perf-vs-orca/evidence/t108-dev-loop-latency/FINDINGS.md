# t108 — The `xtask test` gate is serial, and concurrency takes 42 % off the invocation

Route: [ticket 108](../../issues/108-dev-loop-latency-preflight-concurrency.md).
Take at HEAD `b6184e71`; no slice, no guest, no engine source is involved. This
is a **human-directed scope extension**: the map measures slice throughput, and
the human asked on 2026-11-02 for a ticket on the dev loop's own compile/test
latency (the Rust-iteration discussion the map's Notes record). The extension is
recorded in the map's Notes, not assumed.

## Headline

`cargo xtask test` runs three preflights — `check-literals`, the
`check-test-quality` report, and the guest-freshness check — strictly one after
another. They share no state: all three are read-only scans of the tree. Running
them concurrently makes the gate cost `max(latency)` instead of their sum.

**Measured, four interleaved pairs with frozen harness binaries: a
representative `xtask test` invocation falls from a median 18.77 s to 10.80 s
(−7.97 s, −42.4 %).** The arms do not overlap: the slowest `after` sample
(13.214 s) is faster than the fastest `before` sample (18.438 s). The take's
pre-declared floor was **5 s per invocation**; the lever clears it 1.6x.

Candidate: `test_command` builds the three closures and hands them to
`run_preflights_with` (`xtask/src/test.rs`), which spawns the two source scans on
scoped threads and runs the freshness check on the calling thread — the
`Invocation` that owns the canonical-WIT loader is not `Send`, so it stays put
and is still reused for a stale rebuild. Failure reporting keeps the original
order, so a literals or test-quality failure still aborts **before** any guest
rebuild.

## Table 1 — the lever, per round

The harness binaries were built from the same HEAD with the change stashed
(`before`) and applied (`after`), then invoked directly so cargo never rebuilds
the harness inside the timed window. Rounds alternate `before`/`after`.

| round | before (s) | after (s) | delta (s) |
| --- | ---: | ---: | ---: |
| 1 | 18.438 | 12.179 | −6.259 |
| 2 | 19.034 | 13.214 | −5.820 |
| 3 | 23.374 | 9.424 | −13.950 |
| 4 | 18.507 | 8.961 | −9.546 |
| **median** | **18.771** | **10.802** | **−7.969 (−42.4 %)** |

Payload: `cargo xtask test -p slicer-runtime --test contract
authored_tool_index_tdd --summary` — 4 tests, 0.00 s of test execution, no
rebuild. Everything that moves is the gate.

## Table 2 — the gate's components, measured serially

| scanner | round 1 (s) | round 2 (s) |
| --- | ---: | ---: |
| `cargo xtask check-literals` | 8.196 | 9.375 |
| `cargo xtask check-test-quality --report` | 5.621 | 7.686 |
| `cargo xtask build-guests --check` | 2.731 | 4.543 |
| **serial sum** | **16.548** | **21.604** |
| concurrent cost (slowest scanner) | 8.196 | 9.375 |

The measured end-to-end delta (7.969 s) sits inside that window: the gate can
only save `sum − max`, i.e. 8.35 s and 12.23 s in these two rounds, and the
end-to-end sample includes the payload that does not change.

## Table 3 — the dev loop's other measured costs

All wall-clock, one machine, disclosed as inventory rather than as a gated
claim.

| path | measurement |
| --- | ---: |
| warm no-op `cargo check --workspace --all-targets` | 0.81–2.35 s |
| warm no-op `cargo clippy --workspace --all-targets -- -D warnings` | 2.08–2.77 s |
| `cargo check` → `cargo clippy` alternation, both warm | 2.41 s, **0 crates re-checked** |
| `cargo clippy` → `cargo check` alternation, both warm | 1.69–2.35 s |
| broad check after one-line edit to `crates/slicer-ir/src/lib.rs` | 65.3 s |
| broad check after one-line edit to `crates/slicer-core/src/lib.rs` | 72.0 s |
| `cargo check -p slicer-runtime --tests` after one-line edit to the contract bucket root | 314 s |
| first codegen of the contract test binary after metadata-only builds | 994 s |
| incremental rebuild of the same binary | 17.26 s |
| relink-only pass (binary and PDB deleted, objects warm) | 12.76 s |
| `contract` bucket execution | 99.80 s (291 pass / 23 fail without `perimeter-spatial-test-support`) |
| contract test binary / PDB | 89,697,792 B / 770,166,784 B |

The two large codegen numbers explain the suite's documented ≥11-minute cost
better than any gate: the first object-code build of a bucketed test target is
the expensive event, and a comment-level edit afterwards is not.

## Falsified premises

Four plausible levers from the same discussion were priced and rejected before
the preflight work, which is why the take does not spend its budget on them.

1. **`check` and `clippy` do not share artifacts.** They do. With both warm, an
   alternation in either direction completes in 1.7–2.4 s and prints **zero**
   `Checking` lines. The 129 s and 138 s readings that suggested a split were a
   dirty `slicer-runtime` test target left behind by a reverted probe edit — not
   a compiler-wrapper tax.
2. **Feature-set divergence between `xtask test` and the documented narrow
   command causes rebuild thrash.** It does not for `slicer-runtime`: a plain
   `cargo test -p slicer-runtime --test contract … --no-run` immediately after a
   host-algos `xtask test` run finished in 0.72 s with no recompilation. The
   package already enables `slicer-core/host-algos` transitively.
3. **The linker is a large share of the local loop.** A relink-only pass costs
   12.8 s; codegen, not linking, owns the rebuild. (Cold full-build link share is
   not claimed.)
4. **Dropping debug info speeds the rebuild an agent pays.** The same
   incremental rebuild measured 17.26 s at the default `debug = 2` and 28.24 s
   with `profile.test.package.slicer-runtime.debug = 0`. A 0.77 GB PDB is not
   what makes this loop slow.

## Adjacent findings (harness defects, not speed)

**A1 — `cargo xtask test -p <package>` fails for packages that do not depend on
`slicer-core`.** `cargo xtask test -p xtask --summary` exits with
`error: the package 'xtask' does not contain this feature: slicer-core/host-algos`.
`test_command` appends the feature unconditionally when the caller set none, so
the gated entry point cannot run `-p xtask`, `-p slicer-config`, `-p slicer-ir`,
`-p slicer-scheduler`, `-p slicer-gcode`, or `-p slicer-helpers`. The feature is
load-bearing only where `slicer-core` is the tested package; elsewhere it is at
best inert and at worst an error.

**A2 — the documented narrow command's filter matches no test.**
`cargo test -p slicer-runtime --test contract core_module_ir_access_contract_tdd`
reports `0 passed; 314 filtered out`. It builds the bucket and runs nothing,
which reads as green. The filter name in `AGENTS.md` has drifted.

Both are outside this ticket's question and are recorded for their own take.

## Not priced (fog)

- Cold-workspace build effect of `debug = 1` or `debug = 0` (needs two clean
  target dirs to price honestly).
- Linker substitution on a cold full build.
- `cargo-nextest` on the five test buckets. The buckets are few and their tests
  are long; process-per-test isolation may buy little, but it is unmeasured.
- `sccache`, CI cache keys, job parallelism, path filters and
  `cancel-in-progress`. These are the discussion's CI half and cannot be priced
  without a CI runner.

## Method and disclosures

- **Load was not held fixed.** The machine reported 56 % at the end of the take
  (12 logical CPUs). Both arms were measured with frozen binaries in interleaved
  rounds, and the arm ranges do not overlap, so drift is not the explanation for
  the delta.
- **Frozen harness binaries.** `target/t108-xtask-{before,after}.exe` are built
  from the same HEAD with the change stashed and applied. `cargo xtask` itself
  would rebuild the harness inside the timed window; these runs bypass that.
- **The 994 s codegen sample** was taken once, on the first object-code build
  after a series of metadata-only (`cargo check`) builds. It is an inventory
  datum, not a gated claim; the steady-state number is the 17.26 s rebuild.
- **The debug-override comparison** is one sample per arm on the incremental
  path. It is enough to reject "dropping debug info is a free win", not enough to
  price a cold-build effect in either direction.
- **The 23 `contract` failures** on a feature-less run are the deliberate loud
  guard for `perimeter-spatial-test-support`, not a red tree. CI passes the
  feature.

Evidence: `summary.json`, `verify.py` and `REPRODUCE.md` in this directory
(checkout-only verify exits 0 with 26/26 claims, `--self-test` detects 10/10
negative controls).
