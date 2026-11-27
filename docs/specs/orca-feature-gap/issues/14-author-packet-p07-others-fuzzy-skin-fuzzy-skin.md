# 14 — Author packet P07 — Others / Fuzzy Skin — fuzzy-skin

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa77daa6affeGO52pdCSfXpjZr) — claimed 2026-08-31
Blocked by: 06, 103
Map: ../map.md

## Question

Author the spec packet for **P07 — Others / Fuzzy Skin — fuzzy-skin** — 7 keys, Tier A plumbing, owner fuzzy-skin. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P07 — Others / Fuzzy Skin — fuzzy-skin):

`fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/259-fuzzy-skin-keys/` authored (`draft`), preflight **PASS** (2026-08-31; S0–S8 + AC-command + Doc-Impact checks, zero blockers, zero highs; report persisted at `preflight-report.md` in the packet dir).

Scope: **7 keys** as listed. Two wired (decision points re-derived in code, not trusted from the tier table), five declared-with-gap:

- **Wired (2):** `fuzzy_skin` (canonical `FuzzySkinType` enum — **not a bool**; the snapshot's hand-maintained "Disabled" default was wrong, the machine-read canonical default is `disabled_fuzzy`) → the module's loop-selection gate: `disabled_fuzzy`/`hole` → no candidates, `external`/`all` → `LoopType::Outer` (perimeter_index 0), `allwalls` → every wall loop, `none` → outer walls with the per-vertex flag gate (painted-only; the port's existing flag path, DEV-126 context); and `fuzzy_skin_first_layer` (`should_fuzzify`'s `!config.fuzzy_first_layer && layer_id <= 0` → layer-0 pass-through at default). Both gates are canonical-alignment behavior changes (default `disabled_fuzzy` is inert; layer 0 no longer fuzzed at default) — the existing layer-0/apply-to-all tests are updated in the same step with measured justification.
- **Declared-with-gap (5):** `fuzzy_skin_mode` (canonical consumes it only in `fuzzy_extrusion_line` — the Arachne extrusion-line path; the port's module is a `fuzzy_polyline` Polygon-path port over `WallLoop` IR; default `displacement` matches the port's point-displacement algorithm), `fuzzy_skin_noise_type` (libnoise coherent modules for all but `classic`; the port's xorshift RNG is the `UniformNoise` (classic) analogue, so the default path is behaviorally faithful), `fuzzy_skin_octaves`/`persistence`/`scale` (consumed only by the coherent modules; unused by classic). AC-N1 pins all five as non-perturbing.
- **Recorded divergences (not fixed):** the IR has no `LoopType::Hole` (classic-perimeters emits hole boundaries as `LoopType::Outer` at `perimeter_index 0` — indistinguishable from the contour), so `hole` is inert and `all` degrades to `external` (contour only); hole-loop identification is IR work, queue-sized. `apply_to_all` (PnP-specific, untouched per ticket 07) keeps its "ignore per-vertex flags" meaning scoped to enum-selected loops.
- Manifest exactness: enum tables in the in-tree `type = "enum"` + `values` form, canonical value order; defaults all canonical-identical (`fuzzy_skin` `disabled_fuzzy`; `fuzzy_skin_mode` `displacement`; `fuzzy_skin_noise_type` `classic`; `octaves` 4 [1,10]; `persistence` 0.5 [0.01,1]; `scale` 1.0 [0.1,500]) — **no deviation rows, no human sign-off consumed**.
- CONFIG_BLOCK honesty: the preflight sweep found `fuzzy_skin` and `fuzzy_skin_mode` **already in `ORCA_CONFIG_PADDING`** (`crates/slicer-gcode/src/serialize.rs`) — the packet corrects the `fuzzy_skin` padding value `"none"` → `"disabled_fuzzy"` (it contradicted the canonical default; no entries gained or lost) and AC-5 pins the post-packet state: explicit `fuzzy_skin = "external"` emits exactly one line (dedup-suppressed padding), defaults emit the two corrected/unchanged padding lines and none of the other five keys.
- Preflight correction (recorded in `preflight-report.md`): the padding discovery rippled through all four contract files; Step 2's test-fallout refined so per-vertex-flag tests map to `fuzzy_skin = "none"` and apply-to-all tests to `"all"`. Packet number 259 derived from disk per ticket 06's rule; queue packet carries `task_ids: []`; implementation order note: packet 258 (P06, different owner) precedes in the queue — no same-module churn.

## Answer
