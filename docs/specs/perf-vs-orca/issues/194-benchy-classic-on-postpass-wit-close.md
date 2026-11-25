# Close the `benchy-classic-on` tie: land `PP-4` `GCodePostProcess` stream-view and `PP-1b` `part-cooling` `entity-roles`

Type: task / measured candidate (additive-WIT `PP-4` + `PP-1b`, exact output, final single-HEAD `n >= 4` 8/8 closure)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Final single-HEAD `n >= 4` Orca closing scoreboard](193-final-closing-orca-scoreboard.md) ([`evidence/t193-closing-scoreboard/FINDINGS.md`](../evidence/t193-closing-scoreboard/FINDINGS.md)), whose closing board won **8/8 on wall clock** but closed **7/8**: binding `benchy-classic-on` is **0.9646x** (2.4473 s vs 2.5370 s) yet misses strict separation by **2.9 ms** (binding max 2.4913 s vs Orca min 2.4884 s); the other seven cells pass `< 1.00x` + `>= 4` retained unstarved + strict separation + identity + CPU integrity guard.

## Question

**Does landing the exact additive-WIT pair `PP-4` (resource-backed `gcode-stream-view` + `push-input-range` for `com.core.machine-gcode-emit`) and `PP-1b` (`entity-roles` query for `com.core.part-cooling`) move binding `benchy-classic-on` from the measured **0.9646x / −2.9 ms** tie to `< 1.00x` with `>= 4` retained unstarved **strict separation**, keep the other seven cells closed, keep **16/16** output identity to `t182/reference.json`, and close the `perf-vs-orca` map on a fresh single-HEAD `n >= 4` board?**

## What is already known (t189 + t193)

- **`t193` board (2026-11-25, HEAD `97b2474d`, Orca **2.5.1** `5013b1bd…`):** 8/8 wall wins, 7/8 closed. `benchy-classic-on` ordinary median **2.4473 s** vs Orca **2.5370 s** (**0.9646x**, 3.67 % cut), but strict separation is **−2.9 ms**; the accelerated arm already closes it at **0.9182x / +117 ms**, but `pnp-ordinary` binds per `t176`. All 96 measured samples unstarved, 16/16 identity, 40/40 Orca matched-key assertions, `code-1200 = 0`.
- **`t193` attribution (fresh `--instrument-stderr` pass, output hash `a8c93535…`):** total 2,547 ms — prepass **1,629 ms (63.96 %)**, per-layer **563 ms (22.10 %)**, postpass **272 ms (10.68 %)**, outside 80 ms (3.14 %), validation 3 ms. Postpass: **`GCodePostProcess` 127 ms** (46.7 % of postpass), `LayerFinalization` **82 ms**, `GCodeEmit` **45 ms**, `GCodeSerialize` **14 ms**. `com.core.part-cooling` still burns **143.2 M fuel**.
- **`t189` prices (post-`t192` wall):** `PP-4` **43–86 ms** `benchy` / **200–528 ms** `base`; `PP-1b` **8–35 ms** `benchy` / **35–145 ms** `base`; combined **3.9–4.8 %** `benchy` / **2.4–3.5 %** `base`. On `benchy-classic-on` the pair is **51–121 ms = 2.08–4.95 %** of the binding median, **17x–42x** the 2.9 ms separation deficit, projecting **0.917x–0.945x** with **48–118 ms** separation (`ESTIMATE ONLY`).
- **Why it is still open:** `PP-4` requires an **additive WIT change** (a resource-backed stream view + a `push-input-range` import), which `t192` deliberately excluded; both leads were parked for the `<= 0.50x` follow-on because `t192` projected 8/8 `< 1.00x`. `t193` falsified that projection on strict separation for this one cell.

## Scope

1. **`PP-4` — `GCodePostProcess` stream view (additive WIT):**
   - Add a resource-backed `gcode-stream-view` + `push-input-range`-style import to `crates/slicer-schema/wit/` (canonical source; bump the package/schema version additively), regenerate host bindings and the guest macro path from that single source.
   - Rewire `PostPass::GCodePostProcess` / `com.core.machine-gcode-emit` so the `+2` start/end commands are pushed without marshalling the full `102K–977K` command list across WIT.
   - Output must remain byte-identical; no command ordering, layer-marker or error-code change.
2. **`PP-1b` — `part-cooling` `entity-roles` query (additive WIT):**
   - Replace the eager entity-role materialisation with a precise `entity-roles` query for `com.core.part-cooling`; keep the guest's decisions and emitted commands byte-identical.
3. **Standing gate (all 8 cells, both arms):**
   - Literal `0.80 %` paired wall floor and CPU corroboration (or the standing `t137` 4-cell form only if a candidate is structurally inert on supports-off), byte-identical output in all eight against `t182/reference.json`, both guest freshness gates exit `0`, no `degraded`/code-1200 diagnostics.
   - If a change cannot be made output-identical, stop and return to the human with the exact moved lines; do not waive identity.
4. **Final closing board:** after landing, run one fresh single-HEAD `n >= 4` eight-cell × three-tool board (`pnp-ordinary` binding, `pnp-accelerated` disclosed) under the `t193` protocol and require **8/8** closing passes. If `benchy-classic-on` still fails strict separation, record the measured miss and return to the human rather than re-running until separation appears.
5. **Evidence:** `docs/specs/perf-vs-orca/evidence/t194-benchy-classic-on-postpass-wit/` with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, and a checkout-only `verify.py` (`--self-test` exit `0`).

## Non-goals

- No geometry change, no reference-set move, no parity exception.
- No other WIT additions or `PostPass` rewrites; `PP-4` and `PP-1b` only.
- No change to the `t176` binding-arm or CPU-disclosure rules.
- No `<= 0.50x` work; that remains a fresh follow-on map after 8/8 `< 1.00x`.
