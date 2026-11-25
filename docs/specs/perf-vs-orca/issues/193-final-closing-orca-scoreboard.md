# Final single-HEAD `n >= 4` Orca closing scoreboard (`8/8` destination closure check)

Type: task / scoreboard (single-HEAD `n >= 4` matched-pair closing board against OrcaSlicer `2.5.1` across all 8 cells × `{pnp-ordinary, pnp-accelerated, orca}`; zero code edits)
Label: wayfinder:task
Status: resolved (2026-11-25; **8/8 wall wins, official running tally 7/8 —
  the map stays open.** Binding `pnp-ordinary` ratios **0.6803x–0.9646x**, accelerated
  **0.6840x–0.9705x**; seven cells pass `< 1.00x` + `>= 4` retained unstarved +
  strict separation + identity + CPU integrity guard, while `benchy-classic-on`
  is a **0.9646x / −2.9 ms strict-separation tie** (2.4473 s vs 2.5370 s;
  binding max 2.4913 s vs Orca min 2.4884 s). `16/16` byte-identical to
  `t182/reference.json`, 40/40 Orca matched-key assertions, zero starvation,
  `code-1200 = 0`; `verify.py` PASS **853/853**, `--self-test` **34/34**.
  Graduated [194](194-benchy-classic-on-postpass-wit-close.md).)
Assignee: 2026-11-25 OpenCode wayfinder session — the human said "continue the perf vs orca map" (no ticket named); 193 was the sole open frontier ticket.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Exact Zero-WIT `PostPass` acceleration: lazy `LayerFinalization`, parallel zero-alloc `GCodeSerialize`, and `GCodeEmit`/tail cleanup](192-postpass-exact-acceleration.md) ([`evidence/t192-postpass-exact-accel/FINDINGS.md`](../evidence/t192-postpass-exact-accel/FINDINGS.md)), whose cross-window route composition (`ESTIMATE ONLY`) projects **all 8/8 cells `< 1.00x`** on both arms (`0.5761x–0.9151x` ordinary; `0.5504x–0.8620x` accelerated).

## Question

**On one quiet single-HEAD `n >= 4` matched-pair closing board against OrcaSlicer `2.5.1`, does `pnp-ordinary` (binding arm; `pnp-accelerated` measured and disclosed beside it) beat OrcaSlicer (`< 1.00x` median uninstrumented wall-clock, strict separation on close cells `0.85x < ratio < 1.00x`, CPU integrity guard satisfied, `16/16` byte-identical to `t182/reference.json`, matched keys asserted per Orca run) in all 8 of 8 matrix cells — closing the `perf-vs-orca` map?**

## Scope

1. Build/stage fresh `pnp-ordinary` and `pnp-accelerated` dists from the post-`t192` HEAD (`cargo xtask build-guests --check` and `--accelerated --check` exit `0`).
2. Run a quiet single-HEAD `n >= 4` interleaved matched-pair scoreboard across all 8 cells (`{benchy, base} × {classic, arachne} × {supports off, on}`) × `{pnp-ordinary, pnp-accelerated, orca-2.5.1}` (`RAYON_NUM_THREADS=12`, unsandboxed, matched settings asserted per Orca run, OrcaSlicer `2.5.1` executable identity stamped).
3. Verify `t176` Option C CPU integrity guard (tight process CPU, unstarved `cpu/wall`, matched non-degraded output) and disclose `PnP/Orca` CPU efficiency ratio beside every cell.
4. Record `docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/` with reduced JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit `0`).
5. **Decision branch:**
   - If **8/8 cells pass** on the single-HEAD `n >= 4` closing board → **CLOSE the `perf-vs-orca` map** (and hand off to a fresh `<= 0.50x` / 2× faster map if desired).
   - If any cell remains `>= 1.00x` or fails strict separation → attribute the remaining cell(s) and graduate the targeted follow-on ticket(s) (e.g. `PP-4` `GCodePostProcess` stream-view or `PP-1b` `part-cooling` `entity-roles` query).

## Answer

**No — 8/8 wall wins, official running tally 7/8 (7 cells close), so the map stays open.**
One quiet single-HEAD board (`t193-closing-board`, HEAD `97b2474d`, last code
commit `e7b3c68e` / `t192`, OrcaSlicer **2.5.1** `5013b1bd…`), 8 cells ×
3 tools × 1 warmup + 4 measured runs, `RAYON_NUM_THREADS=12`, 96 measured
rows, zero starvation, load median **21 %** / free RAM median **15.2 GB**.

| Cell | Orca | PnP ordinary | Ratio | Strict sep. | Close |
| --- | ---: | ---: | ---: | ---: | --- |
| `benchy-classic-off` | 1.842 s | 1.601 s | **0.8691x** | +0.163 s | PASS |
| `benchy-classic-on` | 2.537 s | 2.447 s | **0.9646x** | **−0.003 s** | **FAIL** |
| `benchy-arachne-off` | 2.058 s | 1.783 s | **0.8662x** | +0.170 s | PASS |
| `benchy-arachne-on` | 2.568 s | 2.428 s | **0.9454x** | +0.091 s | PASS |
| `base-classic-off` | 12.738 s | 9.440 s | **0.7411x** | +2.518 s | PASS |
| `base-classic-on` | 22.522 s | 15.500 s | **0.6882x** | +6.352 s | PASS |
| `base-arachne-off` | 13.596 s | 10.312 s | **0.7584x** | +3.017 s | PASS |
| `base-arachne-on` | 23.213 s | 15.792 s | **0.6803x** | +7.165 s | PASS |

- **Accelerated arm:** 8/8 wall wins (`0.6840x–0.9705x`); it would close
  `benchy-classic-on` at `0.9182x` / +0.117 s, but `pnp-ordinary` binds per
  `t176`.
- **CPU efficiency disclosure:** PnP/Orca process CPU **0.9447x–2.0980x**
  ordinary (`0.9365x–2.0527x` accelerated); the integrity guard passes 8/8.
- **Attribution (`benchy-classic-on`):** 2,547 ms instrumented = prepass
  **1,629 ms (63.96 %)**, per-layer **563 ms (22.10 %)**, postpass **272 ms
  (10.68 %)**, outside 80 ms. Postpass = `GCodePostProcess` **127 ms**,
  `LayerFinalization` **82 ms**, `GCodeEmit` **45 ms**, `GCodeSerialize`
  **14 ms**. The cell is not prepass-bound; it is a **3 ms postpass tie**.
- **Decision:** graduate [Close the `benchy-classic-on` tie: land `PP-4`
  `GCodePostProcess` stream-view and `PP-1b` `part-cooling` `entity-roles`](194-benchy-classic-on-postpass-wit-close.md).
  `t189` prices the pair at **51–121 ms** on `benchy` (2.08–4.95 % of the
  binding median), **17x–42x** the separation deficit, projecting
  **0.917x–0.945x** (`ESTIMATE ONLY`).
- **Evidence:** [`evidence/t193-closing-scoreboard/`](../evidence/t193-closing-scoreboard/FINDINGS.md)
  (`verify.py` PASS 853/853, `--self-test` 34/34; one aborted-launch warmup
  row deduped and disclosed).
