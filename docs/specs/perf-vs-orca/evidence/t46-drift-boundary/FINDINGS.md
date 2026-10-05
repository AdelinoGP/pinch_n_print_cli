# Ticket 46 — Frozen-job output drift boundary: FINDINGS

Route ticket: [Frozen-job output drift boundary](../../issues/46-frozen-job-output-drift-boundary.md).
Human-authorized take 2026-10-05 (diagnosis only; no repair, re-measure, or
commit of any production behavior).

**Verdict: the drift is intended behavior, not a defect — it is the
config-scope-resolution program plus the ticket-47 annotation repair, and the
current tree composes them exactly.** Three references were re-derived
byte-for-byte from their own era snapshots on the same machine, and the
current HEAD result reconstructs the t47 references by one re-inserted
appendix line. No work is lost; the outputs carry *more* disclosure and *more*
containment, and every era completes clean. The boundary future takes must use
is recorded below: **current HEAD `5a61a106…` for the t44 job, `e1088b3d…` for
classic-off, with frozen `7049a06d…` and t44-era `1b71f83d…` retired as timing
baselines for this job** (they cannot be reproduced by any current binary; they
document older trees).

## Measured reference chain (this take, same machine)

All four runs: same model (`6a07f34c…`), same config
(`benchy-arachne.json` = t41 corpus `arachne.json`, `2b8314ed…`), 12 threads,
external-only (`--no-default-module-paths --no-integrated-modules`),
uninstrumented, 24/24 `external` provenance (`module diagnose` pass, this take).

| era | tree | binary/modules used | output sha256 (first 12) | bytes | fan lines |
| --- | --- | --- | --- | ---: | ---: |
| frozen (t38 campaign snapshot) | `2ebc372a` | `.local-artifacts/.../t38-campaign-...Z/ordinary/` | `7049a06d68ee` | 3,821,661 | 0 |
| t44 recon baseline | `0d65dbd9` | `target/t44-ab/snapshots/ordinary-baseline/` | `1b71f83db378` | 3,806,297 | 0 |
| current HEAD (fresh `xtask dist`) | `a21f119c` (+ this claim) | `target/dist/developer/` | `5a61a1066a46` | 3,809,027 | 277 |
| t47 t44-job reference (recorded) | post-`953fa0b1` | recorded in t47 FINDINGS | `11e7d38a60f5` | 3,809,057 | 277 (275+2) |
| t47 classic-off reference (recorded) | post-`953fa0b1` | recorded in t47 FINDINGS | `e1088b3d…` | 4,332,881 | 281 (279+2) |
The frozen snapshot reproduces `7049a06d` and the t44 snapshot reproduces
`1b71f83d` **exactly** — both are honest, reproducible controls for their own
trees. The current tree is the only one that is a candidate reference for
future takes.

## The drift decomposes into exactly two eras of intended change

### 1. Current HEAD vs the t44-era tree: same move stream, plus the t47 repair

`current-head.gcode` and the t44-era `head-exact-flags.gcode` have an
**identical geometry move stream**: 92,519 G0/G1 move lines in each, with the
same coordinates and E values in the same order (verified by an F-stripped
move-sequence comparison; byte-level difference only at inserted fan lines and
one appendix key line).

The differences are:

- **+277 fan lines** (`M106 S255` ×275, `M107` ×2; 2,760 bytes) — exactly the
  ticket-47 finalization-annotation repair (t47 FINDINGS recorded ×275 + ×2 for
  this job). The external leg previously dropped part-cooling's fan commands.
- **−1 appendix key line**: t44-era disclosed 241 `; key = value` lines,
  current discloses 240. The removed line is the `support_overhang_angle`
  alias (30 bytes), which commit `06958e77`'s registry canonicalizes to
  `support_threshold_angle` (still disclosed); the remaining diff is
  canonicalization of spellings, not lost settings.

Byte arithmetic, exact: `3,806,297 + 2,760 − 30 = 3,809,027` ✓.

**Reconstruction proof (both directions):**

- `current-head.gcode` **+** one re-inserted `; support_overhang_angle = 30`
  line == `11e7d38a60f5…` (the t47 t44-job reference), 3,809,057 bytes ✓.
- `classic-off-current.gcode` (fresh HEAD, matched-pair classic config)
  **+** the same line == `e1088b3d…` (the t47 classic-off reference),
  4,332,881 bytes ✓.
- `current-head.gcode` **−** 277 fan lines **+** the alias line ==
  `1b71f83d…` (the t44 reference), 3,806,297 bytes ✓.

So the entire t44→current drift is `{fan-command repair} + {alias-line
canonicalization}`, both disclosed, both intended; nothing else moved.

### 2. t44-era vs frozen: the config-scope-resolution program (intended)

The frozen snapshot was built at `2ebc372a` (2026-09-30 evening, includes the
t37 empty-commit fix). The t44-era tree (`0d65dbd9`) includes the whole
`config-schema-registry` / `config-scope-resolution` merge (`9dd16c31` and its
14 commits) that landed 2026-10-01. That program is the documented intent
(`docs/specs/config-scope-resolution-plan.md`: "Give `ConfigView` one
meaning… make emission registry-driven… accepting the `CONFIG_BLOCK` byte
change"), and its effects are exactly the observed deltas:

| observable | frozen (`2ebc372a`) | t44/current | mechanism |
| --- | --- | --- | --- |
| start block | none (starts at `M73`) | `M190 S60` / `M109 S215` / `PRINT_START …` | the frozen-era `ConfigView` was declared-filtered from the **raw** config source (`bind_module_config_view` at `2ebc372a`), which never authored `machine_start_gcode`, so the template was absent; the registry-seeded resolved view now carries the manifest default (`require_int` reads, packet 06 fail-closed semantics) |
| `M73 R/S` | 70 min | 101 min | the estimate follows the changed move stream — walls at the resolved 50 mm/s instead of the 60 mm/s raw-source default, plus the larger fill stream (measured +21% on a pure distance/feedrate walk, 38.6 → 46.9 min); raw `M190`/`M109` are not timed by the estimator |
| wall feedrate | `F3600` (60 mm/s raw-source default × path factor 1.0) | `F3000` (resolved 50 mm/s × 1.0) | the frozen tree read a 50 mm/s `ResolvedConfig` value but the emitter's feedrate table came from the raw source (`FeedrateConfig::from_raw_config(&config_source)`); it fell back to the 60 mm/s default. Current feeds the expanded global (`from_raw_config(&expanded_global_source)`), so the resolved 50 mm/s is honored — the *disclosed* `outer_wall_speed = 50` finally matches the emitted F |
| bottom/sparse fill angle | axis-aligned (0°/90°) | 45° | the old `rectilinear-infill` read `infill_angle` from its raw-source view, which lacked the key, so it fell back to its local `0.0`; the resolved view now carries the resolved `infill_angle = 45` (the appendix disclosed 45 in both eras — the frozen-era *output* did not follow its own appendix: an era self-inconsistency, now fixed) |
| appendix keys | 102 | 240 | registry-driven `CONFIG_BLOCK` from the full reconciled registry (deliberate per the plan) |
| sparse/top/bottom path length | smaller | larger (sparse +18.2% E) | resolved values (angle, wall placement) change the fill geometry the modules compute, with all of it still inside the wall envelope (see containment below) |
| unclosed-loop warnings | 975 | 1,110 | same mechanism class; current closes the previously-open 0.0001 mm gap at Z=11.4 and moves the warning set with the geometry |

All quantities re-derived from `census.json` (see reproduction).

## Containment and correctness controls

- **Sparse containment (t41's `check_output` re-run on both eras):** zero
  violating points, worst overshoot 0.000 mm, in every era (frozen 3,069
  sparse points; t44/current 3,843; classic-off current 31,996). The extra
  printed length is inside the part, not outside it.
- **Completion:** every era records `status=ok, degraded=false,
  fatal=0, non_fatal=0`.
- **The unclosed-loop warnings are pre-existing on both sides of the drift**
  (975 → 1,110, inner-wall dominated). They are an emitter warning that the
  input loop entity's first/last vertices differ; the current tree closes one
  of the two sub-0.01 mm gaps and otherwise follows the changed geometry. This
  is not caused by the drift and remains disclosed.
- **Determinism:** the current-HEAD slice was repeated (fresh process) and
  produced the identical `5a61a106…` — the new reference is reproducible.

## Visual confirmation (visual-debug, standalone G-code mode)

Bundles in `visual/` (requests + manifests + PNGs tracked):

- `visual/frozen/` — `request-frozen.json` against the frozen output
- `visual/head/` — `request-head.json` against the t44-era output
- `visual/current/` — `request-current.json` against the current output
- `visual/l184/` — the layer-184 pair (frozen vs current)

Layers 0 / 25 / 41 / 104 for all three, plus 184: the frozen-era fill lies on
the axes; the t44-era and current fills are 45°. This is the same resolved
`infill_angle` finding seen numerically, and the t44-era/current renders are
byte-identical per layer (their move streams are identical; the inserted fan
lines do not render as filament).

## Verdict and boundary for future takes

**Intended, not a defect** — the drift is two documented, disclosed behavior
programs (config-scope-resolution; the t47 fan-command repair) composing as
their own evidence predicts, with clean completion and zero containment
violations on every side. No repair, re-freeze, or re-measure is warranted by
this diagnosis.

**Fairness boundary (what future optimization/adoption takes must use):**

1. **Do not use frozen `7049a06d…` or t44-era `1b71f83d…` as equivalence or
   timing references for this job** — no current tree reproduces them.
   They remain valid as controls *of their own trees* only.
2. **Current references:** t44 job (`benchy-arachne.json`, external ordinary)
   = `5a61a106…`; matched-pair classic supports-off (`pnp-classic-supports-off.json`)
   = `e1088b3d…` when the alias line is present (`06958e77` removed it from the
   appendix) — i.e. the **pre-`06958e77` byte stream is recoverable as
   `current + {; support_overhang_angle = 30}`**, and the **post-t47 fan
   commands are part of the accepted output** on fan-bearing jobs.
3. **Fan-bearing jobs:** any reference captured before t47 is outdated (as
   ticket 47 already disclosed); strip `M106`/`M107` only if a pre-t47
   comparison is genuinely required.
4. The ticket-45 gate prerequisite is therefore resolved: fresh-baseline
   gates for the chain may proceed against the current references, with the
   t47 fan-command disclosure applied. **This is not map-wide
   authorization** — the human still authorizes each take.

## Reproduction

Raw captures (gitignored, `target/`): `target/t46-recon/`,
`target/t44-ab/recon/`. Rebuild the current snapshot, then:

```bash
cargo xtask dist --edition developer   # or reuse the existing snapshot
# frozen control (t38 snapshot exe+modules):
".local-artifacts/perimeter-reference-preparation/t38-campaign-20261001T024827Z/ordinary/pnp_cli.exe" slice \
  --model .local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/corpus/supports-off-benchy/model.stl \
  --config .local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/corpus/supports-off-benchy/arachne.json \
  --output target/t46-recon/frozen-control.gcode \
  --module-dir .local-artifacts/perimeter-reference-preparation/t38-campaign-20261001T024827Z/ordinary/modules \
  --no-default-module-paths --no-integrated-modules
# t44 control:
"target/t44-ab/snapshots/ordinary-baseline/pnp_cli.exe" slice … (same shape, t44 snapshot paths)
# current:
"target/dist/developer/pnp_cli.exe" slice … (same shape, target/dist/developer paths)
```

Reduce and verify from tracked files only:

```bash
python docs/specs/perf-vs-orca/evidence/t46-drift-boundary/reduce-t46.py
python docs/specs/perf-vs-orca/evidence/t46-drift-boundary/verify-t46.py
```

`reduce-t46.py` reads the raw tree and writes `census.json`; `verify-t46.py`
exits 0 with no raw tree present and fails closed when census/summary/manifests
disagree (negative control: a tampered summary fails with one `FAIL` line).

## What this take did not do

- No production code, config, or reference was changed, re-frozen, or
  committed beyond this ticket's claim and evidence.
- No timing/keep-drop claim; no adoption-campaign retry; no default-mode
  switch. The classic-off run above is a reference reconstruction, not a
  measurement.
- The accelerated mode was not re-derived (its artifacts are stale in this
  working tree; the ordinary leg is what every reference in this chain uses).
- No claim is made about other jobs (base.stl, supports-on) beyond the
  disclosure that the same two intent programs apply to them.
