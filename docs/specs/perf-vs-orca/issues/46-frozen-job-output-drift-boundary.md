# Frozen-job output drift boundary

Type: task
Status: resolved (2026-10-05; intended behavior, not a defect — see ## Answer)
Assignee: current OpenCode session (wayfinder), 2026-10-05 — human authorized
this take from the open frontier; diagnosis only (no repair/re-measure/commit)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Why does the current host+guest tree change the frozen supports-off Benchy
Arachne job's actual G-code moves and TYPE counts under identical model/config
and external-only flags: intended config/scheduler behavior, a correctness
defect, or a provenance problem? Which output/fairness boundary must future
optimization and adoption takes use?

Surfaced by [Clip-universe preparation hoist](44-clip-universe-hoist.md).
Its paired candidate is output-identical to the fresh baseline, but both differ
from the old reference. Exact-flag frozen control still reproduces that
reference. See [hoist findings](../evidence/t44-clip-universe-hoist/FINDINGS.md)
and the two reduced controls in `evidence/t44-clip-universe-hoist/evidence.json`.
The hoist experiment did not attribute this drift to any merge or fix it.

## Work — separate human take required

- Verify current inputs, ordinary guest freshness, and exact module isolation
  before diagnosing. Do not mix host/guest schema eras or assume a hash change
  means a geometry defect.
- Localize the first changed resolved config/IR/output boundary against the
  reproducible frozen control. Distinguish intended current-job behavior from
  lost work. The existing emitter's unclosed-loop/missing-edge warnings must
  remain visible; clean completion counters do not prove their absence.
- Return the intended-vs-defect decision and next scope to the human. A repair,
  reference replacement, acceptance retry or timing campaign needs its own
  authorization; do not alter either preserved reference to make a gate pass.

This question is about output/job provenance and fairness, not a new Orca
geometry-parity program. No investigation, fix, measurement or commit is
authorized merely by creating this ticket.

## Answer — intended behavior: config-scope-resolution + the t47 fan repair, composing exactly

Resolved 2026-10-05 (diagnosis only; no code, reference, or gate changed).
Full evidence: [FINDINGS.md](../evidence/t46-drift-boundary/FINDINGS.md),
reduced machine-checkable census in `evidence/t46-drift-boundary/census.json`,
re-derivation in `reduce-t46.py`, verification in `verify-t46.py` (exit 0 from
tracked files alone; fails closed on tampering), visual bundles under
`evidence/t46-drift-boundary/visual/`.

**The drift is not lost work and not a defect. It is two documented intent
programs composing exactly, and every era's own reference is reproducible on
this machine:**

| era | tree | sha256 (first 12) | bytes | reproduced |
| --- | --- | --- | ---: | --- |
| frozen t38 snapshot | `2ebc372a` | `7049a06d68ee` | 3,821,661 | ✓ (own snapshot exe+modules) |
| t44-era baseline | `0d65dbd9` | `1b71f83db378` | 3,806,297 | ✓ (own snapshot) |
| **current HEAD** | `a21f119c`+claim | **`5a61a1066a46`** | 3,809,027 | deterministic re-run ✓ |

- **Current vs t44-era is exactly two intended edits.** The geometry move
  stream is *identical* (92,519 G0/G1 moves, same order/values). Differences:
  +277 fan lines (2,760 B; the t47 finalization-annotation repair, ×275 `M106`
  + ×2 `M107`) and −1 appendix key line (30 B; the `support_overhang_angle`
  alias canonicalized by `06958e77`). Byte arithmetic:
  `3,806,297 + 2,760 − 30 = 3,809,027` ✓. Reconstruction: current + the alias
  line == t47's t44-job reference `11e7d38a…` (3,809,057 B) and, for the
  matched-pair classic config, == t47's `e1088b3d…` (4,332,881 B) ✓; current −
  fans + alias == t44-era `1b71f83d…` ✓.
- **t44-era vs frozen is the config-scope-resolution program (intended).** The
  frozen snapshot predates the `config-schema-registry`/`config-scope-resolution`
  merge; its guests read raw-source views (`bind_module_config_view` at
  `2ebc372a` filtered the *raw* source), so declared defaults never arrived
  (no `machine_start_gcode` block; `infill_angle` fell to the module-local
  0.0; the emitter's feedrate table used `FeedrateConfig::from_raw_config(raw)`
  → 60 mm/s default while the appendix disclosed 50). The resolved-view
  program fixed exactly that class: start block now emitted, fill follows the
  disclosed 45°, walls emit the resolved F, `CONFIG_BLOCK` is registry-driven
  (102 → 240 keys). The `M73` estimate follows the changed stream (a pure
  distance/feedrate walk: 38.6 → 46.9 min).
- **Correctness controls:** every era completes clean (`status=ok`,
  `degraded=false`, 0 fatal/non-fatal); sparse containment re-derived zero
  violations/worst overshoot 0.000 mm on every era; unclosed-loop warnings
  (975 → 1,110, inner-wall dominated) are pre-existing on both sides and
  disclosed.

**Fairness boundary for future takes** (the question's second half):
**t44-era `1b71f83d…` and frozen `7049a06d…` are retired as timing/equivalence
baselines for this job** (no current tree reproduces them; they remain valid
controls of their own trees). Current references: t44 job = `5a61a106…`;
classic-off = `e1088b3d…` modulo the removed alias line
(`current + '; support_overhang_angle = 30'`); fan-bearing jobs must use
post-t47 references or strip `M106`/`M107` (as ticket 47 already disclosed).
The ticket-45 gate prerequisite is satisfied by this boundary; it is not
map-wide authorization for any take. No repair, re-freeze, re-measure,
acceptance retry, or default switch occurred or is recommended.
