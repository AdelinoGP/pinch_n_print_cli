# Final single-HEAD `n >= 4` Orca closing scoreboard (`8/8` destination closure check)

Type: task / scoreboard (single-HEAD `n >= 4` matched-pair closing board against OrcaSlicer `2.5.1` across all 8 cells × `{pnp-ordinary, pnp-accelerated, orca}`; zero code edits)
Label: wayfinder:task
Status: open
Assignee: unassigned
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
