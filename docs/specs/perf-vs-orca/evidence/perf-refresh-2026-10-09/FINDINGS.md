# Perf refresh 2026-10-09 — classic-cell re-attribution at HEAD

Method: fresh instrumented + profiled passes (`--profile --instrument-stderr`)
over the four classic matched cells at HEAD (t19 KEEP `b56f429a` + t27 KEEP
`7038f1ad`; release `pnp_cli` built from that tree), plus a re-reduction of the
2026-10-09 t27 A/B base captures. Raw captures (gitignored, the fuller optional
source): `target/perf-refresh-2026-10-09/*.stderr.jsonl`;
`target/t27-ab/campaign/measure/*/base/*.stderr.jsonl`. Reducers and reduced
tables are committed beside this note; `verify.py` exits 0 against the reduced
set alone. Guest freshness `cargo xtask build-guests --check` exit `0` before
the batch; `RAYON_NUM_THREADS=12`; external modules only
(`--module-dir modules/core-modules --no-default-module-paths
--no-integrated-modules`). All four fresh outputs clean (`status=ok`,
`degraded=false`, 0 fatal/non-fatal); base-on G-code `f60e5e9b…` and benchy-on
`76eb4e1d…` match the preserved t27 references, benchy-off `34886ccb…` matches
the t19-era reference, base-off `20e7204c…` captured fresh.

**Wall columns in this note are instrumented/accumulated attribution signals,
never wall claims** (`--profile` inflates; host columns accumulate thread-time —
see `docs/17_agent_debugging.md`'s observer-effect contract).

## Cell anatomy at HEAD (profiled capture, attribution only)

| cell | elapsed_ms | validation | prepass | per_layer | postpass | head | tail |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy-classic-off | 19,996 | 3 | 8,446 | 9,658 | 541 | 1,297 | 52 |
| benchy-classic-on | 31,734 | 3 | 18,366 | 9,647 | 2,366 | 1,205 | 160 |
| base-classic-off | 268,640 | 4 | 147,253 | 116,824 | 3,090 | 1,217 | 300 |
| base-classic-on | 623,795 | 4 | 360,986 | 122,695 | **129,319** | 1,116 | 9,804 |

`head` = slice-id timestamp → first event (module discovery / 24-guest compile /
config prep; ticket 27 measured 1.1–1.5 s); `tail` = last `phase_complete` →
`slice_complete` (ticket 27's post-postpass segment).

## Headline: `PostPass::LayerFinalization` pays ~100–120 s on base classic supports-on

The 2026-10-09 t27 A/B base captures (probe-free, arm-isolated, both modes;
reduced in `reduced/t27-ab-postpass.json`) show the postpass phase in **every**
run — including every baseline arm, so it is independent of the t27 candidate:

| A/B arm | n | prepass median | per_layer median | **postpass median** | elapsed median | postpass min–max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary / baseline | 6 | 324,804 | 99,768 | **106,282** | 539,231 | 100,201–110,389 |
| ordinary / candidate | 6 | 306,809 | 98,392 | **107,373** | 519,956 | 100,613–109,088 |
| accelerated / baseline | 6 | 337,752 | 62,500 | **114,072** | 526,328 | 103,210–121,259 |
| accelerated / candidate | 6 | 314,211 | 63,961 | **115,848** | 501,246 | 110,270–118,519 |

The fresh base-on profiled capture decomposes the 129.3 s postpass
(`reduced/finalization-gaps.json`):

| item | ms |
| --- | ---: |
| `PostPass::LayerFinalization` stage (`stage_complete`) | 110,109 |
| — four module brackets (`module_complete` sum) | 62,749 |
| — four inter-bracket gaps (8,258 / 12,595 / 13,340 / 13,167) | 47,360 |
| `PostPass::GCodeEmit` | 6,394 |
| `PostPass::GCodeSerialize` | 993 |
| `PostPass::GCodePostProcess` | 712 |
| between-bracket remainder (incl. the pre-emit `to_vec` + reconcile; suspect, uninstrumented) | ≈11,111 |

The four module brackets are `overhang-classifier-default` 15,062 ms,
`part-cooling` 15,055 ms, `skirt-brim` 15,982 ms, `wipe-tower` 16,650 ms — while
their guest fuel is ≤ 1.4 B of the cell's 13,090 B total (0.01%). The cost is
host-side whole-print copying, not module work. On base-classic-off the same
stage is 2,040 ms (modules 1,978 + gaps 60) — the supports-on cell pays ~54x
for ~2x output.

Mechanism (code-derived):

- `execute_layer_finalization_with_instrumentation`
  (`crates/slicer-runtime/src/layer_finalization.rs`) clones the entire
  `Vec<LayerCollectionIR>` **before every module**
  (`let mut candidate = layers.clone()`), runs the module on the clone, then
  assigns it back (`*layers = candidate`), dropping the replaced vector. The
  clone sits inside the module bracket (`on_module_start` → `on_module_end`);
  the drop lands in the measured inter-bracket gaps. Four modules × (clone +
  drop) ≈ the 110 s stage. Modules are forced serial (`pool size 1`).
- `execute_postpass_with_capture` (`crates/slicer-runtime/src/postpass.rs`)
  makes another whole-print copy (`layer_irs.to_vec()`) and a per-layer
  `slicer_gcode::reconcile_finalization_travel` pass immediately before
  `PostPass::GCodeEmit`, between the instrumented stage brackets.
- The per-module clone+assign pattern reads as failure atomicity, but
  `fin_result?` aborts the slice on error, so a shared working copy is the
  obvious candidate shape (`module.ir_write_mask()` is already threaded into
  `CompiledModuleLive`).

History: the pre-repair ticket-27 runs showed postpass 24–32 s because DEV-174
was dropping the support geometry those modules process; the post-repair
`dev174-after` ordinary run already recorded **113.7 s** postpass in
`t27-serial-floor/results/run-phase-splits.csv` but was never chased (ticket 27
attributed the post-postpass tail, not this phase). The cost is therefore
latent, not new; the A/B table above is the first repeated measurement.

## Host work-share refresh

`host:slice` leads every cell, driven by `polygon_ops::closing_ex` — the
flat-bridge enclosure closing in `assemble_flat_bridge_areas`
(`crates/slicer-core/src/algos/prepass_slice.rs` builds
`closed = closing_ex(&support, FLAT_BRIDGE_ENCLOSURE_RADIUS_MM, closing_join)`):

| cell | host:slice | closing_ex | offset | clip_polygons |
| --- | ---: | ---: | ---: | ---: |
| base-classic-on | 565.4 | **389.5** (470) | 127.6 (3,113) | 48.3 (19,597) |
| base-classic-off | 561.3 | **384.2** (470) | 128.4 (3,113) | 48.8 (19,597) |
| benchy-classic-on | 28.9 | 24.9 (240) | 1.8 | 2.2 |
| benchy-classic-off | 27.3 | 23.5 (240) | 1.7 | 2.1 |

(accumulated thread-time ms; call counts in parentheses)

`host:native` — native `polygon_ops` work no host-built-in bracket claimed —
is the second bucket and has no named owner:

| cell | host:native | offset | clip_polygons |
| --- | ---: | ---: | ---: |
| base-classic-on | 398.2 | 192.9 (58,391) | 205.4 (115,874) |
| base-classic-off | 197.2 | 102.9 (5,196) | 94.3 (18,678) |
| benchy-classic-on | 22.9 | 13.6 (15,733) | 9.3 (29,366) |
| benchy-classic-off | 12.4 | 7.4 (2,454) | 5.0 (8,193) |

Supports-off cells show a stable per-layer shape (≈10.5 offset + ≈37.7 clip
calls per layer) independent of object size; suspects include
`sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`,
run on each Perimeters/PerimetersPostProcess commit) and region preparation
(`prepare_regions`, `crates/slicer-wasm-host/src/marshal/prepared.rs`, covered
by ticket 05's arena cache). Its wall share is not bounded by this pass.

Remaining host buckets: `host:support_analysis` 207.5 (offset 165.5 — the
post-t27 parallel contact pass), `host:overhang_annotation` 146.4 (offset
124.8), `host:shell_classification` 145.7 (offset 128.9 — phase B), all on
base-on; `host:mesh_analysis` 21.9 (the footprint union).

## Guest fuel refresh (deterministic; shares)

| cell | total fuel | classic-perimeters | `<module self>` | offset2_ex | infill-linker | tree-support |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy-classic-off | 817.2 B | 89.6% | 78.3% | 11.3% (720) | 9.8% | — |
| benchy-classic-on | 866.6 B | 84.5% | 73.9% | 10.7% (720) | 9.2% | 5.0% |
| base-classic-off | 12,592.5 B | 98.1% | 84.0% | 14.2% (1,485) | 1.7% | — |
| base-classic-on | 13,090.0 B | 94.4% | 80.8% | 13.6% (1,485) | 1.6% | 3.4% |

`<module self>` is the unmarked interior — the per-vertex query/stamp path
(tickets 08/18/25) that ticket 20's conditional-stamp candidate targets; the
classic self-fuel is essentially identical supports-on/off (the supports add
`tree-support` fuel, 3.4–5.0%, not perimeter work).

## Coverage gaps

- Arachne cells were not re-run today. The finalization stage is
  generator-independent (the same four modules in every cell), so base arachne
  supports-on — ~455–474 s post-repair (t27 `CELL-MEDIANS.md`) — is the likely
  second beneficiary of the copying finding; a take must qualify its own cells.
- `host:native`'s wall (vs work) share and its owners are unresolved.
- The between-bracket ≈11.1 s is attributed to "the `to_vec`+reconcile suspect"
  by code position only, not by probe.

## Tickets fed

- **58** — LayerFinalization whole-print copying (headline).
- **59** — PrePass::Slice flat-bridge closing reduction.
- **60** — `host:native` attribution.
