# t90 — single-ownership layer collection through finalization and emission

Route: [ticket 90](../../issues/90-postpass-layer-collection-ownership.md).
Measured at HEAD `eb059f60` on the user's Windows desktop (12 rayon threads).
Candidate: the five working-tree host files listed in `REPRODUCE.md`.

## Verdict in one line

The standing paired gate **PASSES in all four pre-declared base supports-on
cells**: wall **−5.54 % / −7.56 % / −6.54 % / −7.64 %** (ordinary/accelerated ×
classic/arachne) with process CPU **−1.13 … −1.67 %**, **24/24 pairs
byte-identical**, one SHA per arm, zero starvation exclusions — and the
instrumented captures show every second of the win: the whole-print clone, the
commit drop and the postpass teardown drop are gone, leaving one drop inside the
`host:gcode_emit` bracket.

## What was built

One ownership pass, not three micro-fixes:

1. **Finalization takes the collection.** `execute_layer_finalization_owned`
   (`crates/slicer-runtime/src/layer_finalization.rs`) moves the caller's
   `Vec<LayerCollectionIR>` into the module loop and returns the committed
   collection. The old shape deep-cloned the whole print for a rollback
   snapshot no production caller reads and then dropped the replaced vector at
   commit. The borrowing entry points stay as a pointer-swap wrapper, and the
   stage's abort contract changes exactly as the ticket allows: a fatal module
   or validation error consumes the working collection instead of leaving the
   pre-stage snapshot behind. Production aborts the slice there and never reads
   the vector again (pinned by
   `finalization_fatal_error_is_instrumented_and_abort_consumes_the_working_collection`
   in `crates/slicer-runtime/tests/executor/layer_finalization_tdd.rs`).
2. **The emitter takes the collection.** `GCodeEmitter::emit_gcode_owned`
   (`crates/slicer-gcode/src/emit.rs`) has a default implementation that
   forwards to `emit_gcode`, so every other emitter source keeps working;
   `DefaultGCodeEmitter` overrides it to run `apply_cross_layer_tool_rotation`
   on the caller's own vector and emit from it. `DefaultGCodeEmitter::emit_gcode`
   now forwards to the owning method with `to_vec()`, so both entry points share
   one emission path and cannot drift apart.
3. **Postpass moves instead of cloning.**
   `execute_postpass_with_owned_capture` already accepted the collection by
   value; it now hands it to `emit_gcode_owned` on the production path, so the
   whole-print clone and the second whole-print drop at postpass teardown are
   both gone and the collection is dropped once, inside the `host:gcode_emit`
   bracket. The visual-debug capture sinks are the only caller that needs the
   layers afterwards: they keep the borrowed path and the collection is **moved
   into** `PostPassCapture` instead of being cloned a second time.

No geometry, ordering, tool-rotation, travel-reconciliation, `GCodeIR` or
serialized-byte change is intended or observed. Both guest freshness gates exit
0; the candidate is host-only and `provenance.json` records byte-identical
24-module trees per mode with distinct `pnp_cli.exe` SHAs per arm.

## Qualification (16 cells)

`qualification.json`: **16/16 cells pass**. In every cell the two arms' G-code
is byte-identical, the ordinary cell equals the post-t81 reference set
(`derive_references.py`, never hand-pinned), the run is clean
(`status=ok`, `degraded=false`, zero fatal/non-fatal), the output is non-vacuous
in its roles, and ordinary ≡ accelerated in **8/8** cross-mode groups. Every
measure row carries its arm's `exe_sha256`; the gate asserts the arms ran
distinct binaries.

Exactness tests (all green, run before timing):

- `gcode_emit_owned_parity_tdd` — 6 tests: owned-vs-borrowed `GCodeIR` and
  serialized-text equality on a rotation fixture, a single-tool fixture and an
  empty collection; the rotation fixture's emitted entity order (authored
  `[30, 40]` in layer 1 becomes emitted `[41, 31]`, i.e. the tool-1 cluster
  moves to the front) and its "layer 1 prints its inherited-tool cluster before
  any tool change" ordering; its `;TYPE:` sequence; the single-tool negative
  control (no `T<n>`, authored order preserved); the emitter's error path; and
  the metadata layer count.
- `layer_finalization_tdd` — the owning entry point's commit, the borrowed
  wrapper publishing the identical vector, and the new abort contract.
- `cargo test -p slicer-gcode` (all targets) green;
  `visual_debug_postpass_tap_tdd` green; `slicer-runtime --test contract`
  failure set **identical** with and without the candidate (23/23 pre-existing
  `resolution`-fixture reds, 291 passed either way).

## Paired gate (4 cells × 6 pairs)

| cell | baseline wall median | candidate wall median | wall Δ | CPU Δ | pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 187.41 s | 175.44 s | **−5.54 %** | **−1.21 %** | 6/6 |
| ordinary base arachne-on | 190.10 s | 175.29 s | **−7.56 %** | **−1.35 %** | 6/6 |
| accelerated base classic-on | 203.47 s | 187.55 s | **−6.54 %** | **−1.13 %** | 6/6 |
| accelerated base arachne-on | 193.93 s | 178.56 s | **−7.64 %** | **−1.67 %** | 6/6 |

Paired wall deltas span **−1.56 … −13.20 %**; **every one of the 24 pairs moved
the right way**, every pair is byte-identical, each arm produced exactly one
G-code SHA, and no pair was excluded for starvation. Process CPU is **flat or
lower in all four cells**, not merely inside the standing +3.0 % work-preserving
ceiling.

Measured wall deltas are **12.0 / 14.8 / 15.9 / 15.4 s**, larger than the
ticket's pre-declared net ceiling of **10.76 s (classic) / 10.29 s (arachne)**.
The attribution below explains the surplus: the one surviving drop costs less
than the emitter's cloned drop did, so the estimate was conservative.

## Attribution (instrumented, attribution only — never a wall claim)

One `--instrument-stderr --profile --profile-verbose` capture per arm per cell;
all eight outputs byte-identical to their references. Ordinary mode, ms:

| term | classic base | classic cand | arachne base | arachne cand |
| --- | ---: | ---: | ---: | ---: |
| `PostPass::LayerFinalization` before first module (clone) | **1,765** | **0** | **1,684** | **1** |
| `PostPass::LayerFinalization` modules | 766 | 824 | 745 | 829 |
| `PostPass::LayerFinalization` after last module (commit drop) | **3,685** | **0** | **3,758** | **0** |
| `PostPass::LayerFinalization` stage wall | 6,218 | 827 | 6,189 | 832 |
| `host:gcode_emit` bracket (walk + drop) | 5,179 | 3,616 | 5,642 | 2,961 |
| `PostPass::GCodeSerialize` | 703 | 685 | 807 | 704 |
| postpass stage-bracket sum | 12,738 | 5,772 | 13,438 | 5,088 |
| postpass phase wall | 17,228 | 5,774 | 17,161 | 5,091 |
| postpass teardown residual (phase − brackets) | **4,490** | **2** | **3,723** | **3** |

Read plainly: the candidate deletes three whole-print operations the t88
attribution had named — the **1.8 / 1.7 s clone**, the **3.7 / 3.8 s commit
drop** and the **4.5 / 3.7 s teardown drop** — and the remaining single drop
inside the emitter costs **1.56 s (classic) / 2.68 s (arachne) less** than the
baseline's clone-then-drop pair, because no fresh whole-print copy is made
before it. The postpass phase falls **66.5 % / 70.3 %**. The accelerated-mode
captures agree: finalization 6,751 → 820 ms, residual 3,740 → 3 ms, emitter
5,398 → 3,578 ms (classic) and 6,687 → 854 ms, 3,693 → 2 ms, 5,354 → 3,572 ms
(arachne).

The stage brackets are instrumented walls and are **not** comparable to the
uninstrumented campaign; they locate where the seconds went, the campaign
measures them.

## Disclosures

- Instrumented captures are one run per arm per cell, attribution only
  (`docs/17_agent_debugging.md`). Their slice-level deltas are not used for any
  speed claim.
- The emitter bracket's **1.6–2.7 s improvement is a measured observation, not
  a theory**: with ownership there is no freshly allocated copy to free, and
  the bracket shrinks; the take does not further decompose *why* the freed
  allocation is cheaper to drop in that state. Nothing in the candidate depends
  on that explanation — the gate is byte identity plus paired wall/CPU.
- The **finalization abort state changed** from "pre-stage snapshot" to
  "unavailable", as the ticket pre-authorized. Production aborts the slice on
  that error (`fin_result?`) and never reads the vector; the executor test that
  asserted the rollback now asserts the consumption.
- Pre-existing reds at this HEAD, unrelated to the candidate: 23 in
  `slicer-runtime --test contract` (measured identical with and without the
  candidate), 3 in `--test executor` and 50 in `--test integration` (diagnosed
  by error signature: the post-t81 mandatory `resolution` config key and one
  CONFIG_BLOCK expectation that predates it). None touches finalization,
  postpass or emission; `REPRODUCE.md` records the counts and the method.
- Only the four base supports-on cells were timed, as pre-declared. Benchy
  cells and supports-off cells are qualification (identity) only; the map
  already carries supports-off as a small-collection control.
- The machine was otherwise idle at the campaign start (≈2 % total CPU) and
  `RAYON_NUM_THREADS=12` was pinned by the harness; per-run desktop load was not
  logged, and the harness excluded **zero** pairs for starvation.
- Model fixtures `tmp/3dbenchy.stl` and `tmp/base.stl` are user-supplied and
  stay gitignored; every committed number is re-derivable from the reduced rows
  in this directory.

## Reproduction

[`REPRODUCE.md`](REPRODUCE.md). `verify.py --self-test` exits 0 against this
directory alone and rejects **11/11** negative controls, including the two that
would let the evidence drift into a false PASS ("baseline clone gap erased",
"residual drop inverted").
