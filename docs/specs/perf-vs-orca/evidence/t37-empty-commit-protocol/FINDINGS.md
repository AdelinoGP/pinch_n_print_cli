# Ticket 37 — `Layer::InfillPostProcess` empty-output protocol

Wayfinder ticket: [InfillPostProcess empty-output protocol](../issues/37-infillpostprocess-empty-output-protocol.md).
Finding source: [Benchy classic sparse-fill domain at middle layers](../issues/35-benchy-classic-sparse-fill-domain.md)
and `evidence/t35-fill-domain/FINDINGS.md`.

This document records (1) the protocol decision and its blast-radius
classification, (2) the functional verification that the containment hole is
closed, and (3) the standing paired ordinary + accelerated A/B.

## 1. The protocol decision

**The fix is stage-local to `Layer::InfillPostProcess`.** It is not a change to
the shared empty-output convention, and the infill-linker module is not touched.

The stage's contract is *replace-with-complete-re-emit* (ADR-0028 §Amendment
2026-07-01 item 2). Under that contract a ran invocation's output **is** the
layer's whole infill set, so zero paths is a **verdict** ("nothing survives the
re-clip"), not an absence. The producer nevertheless collapsed both into
`Ok(None)`, and `apply` reads `None` as "committed nothing" and preserves the
prior `InfillIR` — the raw emitter envelope.

The two are now separated by construction, at the producer, in all three
places that produce the commit:

| site | change |
| --- | --- |
| `deconstruct_layer_ctx` (`crates/slicer-wasm-host/src/dispatch.rs`) | `Layer::InfillPostProcess` + all-empty output → `Some(InfillPostProcess(empty replacement))` |
| `commit_native_layer_response` (`crates/slicer-wasm-host/src/marshal/native.rs`) | same rule on the native leg; its emptiness predicate also gained `raft_fill`, which the wasm check already had (a raft-only output was previously dropped on the native leg) |
| `commit_hec_for_test` (`crates/slicer-runtime/tests/common/mod.rs`) | test-leg mirror, so manual-ctx executor tests match production |

Shared constructor: `empty_infill_replacement` (`crates/slicer-wasm-host/src/marshal/out.rs`).

### Blast radius — the other stages sharing the convention

| stage | semantics | empty-output verdict |
| --- | --- | --- |
| `Layer::Infill` | **merge** into the arena slot | unchanged: `Ok(None)` ("no contribution" is genuinely a no-op) |
| `Layer::InfillPostProcess` | **replace** with complete re-emit | **fixed**: `Some(empty)` |
| `Layer::Support` | set/merge | unchanged (not a replace-with-re-emit contract) |
| `Layer::SupportPostProcess` | replace, but its shipped consumer (`support-surface-ironing`) is additive | unchanged: an all-empty invocation must leave the prior `SupportIR` alone |
| `Layer::AnchoredEvents` | emits a collection or nothing | unchanged: there is no prior slot to supersede |

The distinguishing rule is not "which stage is postprocess" but "does the
stage's contract make its output a complete replacement set". Only
`Layer::InfillPostProcess` does.

### ADR-0025 containment contract preserved

The linker's own `Some(empty)` (host partitioned the region and gave the role no
area → clip away) vs `None` (no boundary resolvable → pass through untouched)
distinction lives in `RoleBoundaries::for_role`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) and is **untouched**:
`git diff --stat -- modules/` is empty for this ticket.

## 2. Functional verification (real G-code, matched Benchy/classic/supports-off)

`cargo xtask build-guests --check` exit 0 before both runs; release host, ordinary
mode, `--module-dir modules/core-modules`, config
`evidence/matched-pair/configs/pnp-classic-supports-off.json`.

| metric | baseline (`verify-classic.gcode`, ticket-35 capture) | candidate |
| --- | ---: | ---: |
| printed sparse segments | 106,442 | **34,307** |
| printed sparse mm | 79,939.9 | **25,616.2** |
| worst sparse overshoot past the layer's wall bbox | **21.78 mm** | **0.00 mm** |
| layers whose sparse output changed | — | exactly the 60 empty-linker layers |
| every other layer | — | byte-identical segment/mm counts |

The 60 changed layers are precisely the set ticket 35 identified as
clipped-to-nothing; each now prints **zero** sparse paths (e.g. layer 104:
2,562 segments / 1,741.2 mm → 0 / 0.0; layer 111: 2,791 / 1,906.1 → 0 / 0.0).
Per-layer CSV: `t37-vs-baseline-layers.csv`. Overshoot measured by
`verify_containment.py`.

Arachne (supports-off) behaves the same on its 16 layers: 47,659 → 37,606
segments, 36,219.4 → 28,653.1 mm. Per-layer CSV: `t37-arachne-layers.csv`.

**Negative control — base.stl is unchanged.** Ticket 28 found base has no
all-empty linker layers, so the fix should be a no-op there. Measured against the
ticket-28 base-classic capture: 225,004 segments and 164,905 mm on both sides,
0 differing layers (`t37-base-classic-layers.csv`). This is what makes the
Benchy change attributable to the empty-linker set rather than to a general
output drift.

Both candidate runs completed `status ok`, `degraded=false`, zero fatal, zero
non-fatal.

## 3. Paired A/B

Driver: [`run_ab.ps1`](run_ab.ps1) — ticket-32 lineage (uninstrumented,
process creation-to-exit wall, `GetProcessTimes` CPU, per-sample cpu/wall,
interleaved arms, arm order alternating by repeat). The two arms differ in
exactly one file per mode: the `pnp_cli.exe` host binary. Guests, manifests,
config, fixtures, and thread count are identical; the 24 module directories in
both accelerated arms are byte-identical (`diff -rq` clean).

| arm | SHA-256 (`pnp_cli.exe`) |
| --- | --- |
| ordinary baseline | `f9650548005b8970faca3c826e21c275af61b027eaea9fb9e665dd657faf46c9` |
| ordinary candidate | `d5d3275d1f37898908bcc75ef1046bc9792840d72f9c178e3a31ac82b4231b81` |
| accelerated baseline | `c33b631d2e8d155d4f6fced52009f2c647ff215af9f647b96ac4b6519d6aac98` |
| accelerated candidate | `490f5ff5b25c6ff0f8ec4ea823fb5cc9d0a7cd7fcb1b0fa40407db3de226edfa` |

The baseline accelerated snapshot was built from the clean tree (HEAD
`3f59b2a9`, no working-tree changes present) and archived to
`target/t37-ab/acc-baseline/`; the candidate was built from the fixed tree and
archived to `target/t37-ab/acc-candidate/`.

Six measured repeats per arm per cell, one warmup per arm, 12 threads, matched
benchy supports-off. Paired deltas are candidate − baseline on the same repeat
index.

| batch | arm | wall median | CPU median | paired wall Δ (faster) | paired CPU Δ (faster) |
| --- | --- | ---: | ---: | ---: | ---: |
| classic-off ordinary | baseline | 20.40 s | 131.23 s | **−0.17 s** (6/6) | +0.04 s (3/6) |
| classic-off ordinary | candidate | 19.89 s | 131.35 s | | |
| classic-off accelerated | baseline | 18.71 s | 112.48 s | −0.13 s (5/6) | −0.66 s (4/6) |
| classic-off accelerated | candidate | 18.62 s | 111.74 s | | |
| arachne-off ordinary | baseline | 17.44 s | 73.93 s | −0.03 s (3/6) | −0.18 s (4/6) |
| arachne-off ordinary | candidate | 17.43 s | 73.86 s | | |
| arachne-off accelerated | baseline | 17.33 s | 72.32 s | −0.01 s (3/6) | −0.55 s (5/6) |
| arachne-off accelerated | candidate | 17.35 s | 71.48 s | | |

**No wall win with corroborating CPU is measured in any cell.** The one
directional wall signal — classic-off ordinary at −0.17 s median, 6/6 pairs —
is ~0.8 % of the cell's wall and its **CPU is flat (+0.04 s, 3/6)**, so under
the map's verdict metric (median uninstrumented wall, process CPU corroborating
and never contradicting) it is not a win. The arachne and accelerated cells are
flat-to-noise in both metrics. The batch ran quiet (cpu/wall 4.06–6.70; no
sample below the 0.75× best-ratio starvation gate, so none was excluded); raw
rows are in `ab/t37c-*.csv`.

An earlier full repeat of the same four batches, on a first-cut candidate
(`3ebb7939…`) that predates the native `raft_fill` predicate alignment but was
otherwise functionally identical (same 34,307-segment / 25,616.2 mm / 0.00 mm
containment result), measured the same flat picture (−0.13/−0.01/+0.08/+0.25 s
paired wall medians). The four `t37b-*` row sets are kept in this directory as
that independent repeat.

**This is expected and does not diminish the change.** The defect's cost was
containment, not throughput: the resurrected envelope is emitted *instead of*
the (empty) linked output, so removing it removes ~3.05 MB of G-code from the
classic file (7.44 MB → 4.39 MB, matching ticket 35's 3,054,585 attributed
sparse bytes) but no measurable slice wall — sparse printing is a small share of
this pipeline's CPU.

### Output disclosure (fairness contract)

Every accepted run: `status ok`, `degraded=false`, zero fatal, zero non-fatal.

| cell / mode | baseline G-code bytes | candidate G-code bytes |
| --- | ---: | ---: |
| classic-off ordinary | 7,443,834–7,445,020 | 4,393,593–4,394,586 |
| classic-off accelerated | 7,443,959–7,444,713 | 4,393,915–4,394,559 |
| arachne-off ordinary | 4,705,584 (all 6) | 4,279,497 (all 6) |
| arachne-off accelerated | 4,705,584 (all 6) | 4,279,497 (all 6) |

The classic candidate's small run-to-run spread is the known same-binary jitter
class (DEV-093); arachne is byte-stable in both arms.

Note the arachne cell also loses output (4,705,584 → 4,279,497): the mechanism
is stage-generic, and on Arachne the same 16 layers have their sparse envelope
clipped away. The output disclosure therefore covers both generators.

## 4. Recommendation

**KEEP.** This is a containment fix, not a performance candidate: it stops PNP
printing paths its own linker rejected, outside the part cross-section (worst
overshoot 21.78 mm → 0.00 mm), and restores the linker's verdict as the final
authority. The paired A/B measures no wall/CPU win in any cell, so the case for
keeping it is correctness, not speed. Keep/drop returns to the human; nothing is
auto-committed.

