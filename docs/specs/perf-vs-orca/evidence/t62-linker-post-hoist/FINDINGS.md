# Ticket 62 — infill-linker post-hoist residual attribution (the per-path execute owns the linker)

**The measured answer:** the accepted
[Clip-universe preparation hoist](../../issues/44-clip-universe-hoist.md) worked
exactly as designed — `t62::clip_prepare` (boundary flatten + 1-unit inflate)
is now **2.60% of linker fuel** on matched ordinary Benchy Arachne
supports-off, prepared exactly once per invocation (393 prepares == 393
invocations) instead of once per path. The residual is different work: the
linker's cost is now the **per-path engine round trip** — every path still
builds a fresh `Clipper64`, re-adds the whole prepared clip universe, and
executes its own independent intersection:
[`PreparedPolylineClip::clip_open_subjects`](../../../../../crates/slicer-core/src/polygon_ops.rs).

| scope (linker self fuel, arachne supports-off ordinary) | fuel | share of linker |
| --- | ---: | ---: |
| `t62::execute` — `Clipper64::execute` per path call | 62,524,472,575 | **85.96%** |
| `t62::engine_add` — engine construction + add subjects + add clip per call | 3,215,819,329 | **4.42%** |
| `t62::connect_infill` — `chain_or_connect_infill` | 3,442,526,642 | 4.73% |
| `t62::clip_prepare` — universe flatten + inflate (once per invocation) | 1,890,706,516 | 2.60% |
| `t62::meta_convert` — `point_with_metadata` nearest-point scan | 907,887,504 | 1.25% |
| `polygon_ops::clip_polygons` — the boundary-prep footprint `difference_ex` | 198,798,468 | 0.27% |
| all other named terms (overlap offset, graph build, records, emit, loop glue) | 375,611,308 | 0.52% |
| unmarked wrapper self (lib entry, config reads, ironing copy, flush) | 184,859,441 | 0.25% |
| **linker total** | **72,740,681,783** | 100% |

Classic cells split identically in shape (execute 83.99%, engine_add 5.44%,
prepare 3.13%, connect 4.94%). The full per-cell table, the reconciliation,
the census and the ceilings are re-derived by
`python docs/specs/perf-vs-orca/evidence/t62-linker-post-hoist/verify.py`
(exit 0 from the in-repo `captures/` alone).

**One candidate follows: batch each invocation's paths into one engine + one
execute, re-deriving per-path output attribution.** Under the same-cost
assumption (see limits) the uniform-by-invocation ceiling removes
85.96% × (1 − 393/12,022) of execute and 4.42% × (1 − 393/12,022) of
engine_add — the linker drops to **≈9.15 B fuel ≈ 12.6% of its current
total** (arachne supports-off), from 44.28% of slice guest fuel to ≈5.6%.
Attribution and representation analysis only: **no candidate was implemented
this take; an execution ticket graduates below.**

## Provenance and integrity

- Captured at commit `921306dc` (HEAD; the t58 tracker reconciliation). The
  release executable's SHA-256 matches the
  [2026-10-13 refresh](../perf-refresh-2026-10-13/inputs.json)'s
  `exe_sha256` (`b84c0e1a…`) — the tree is in exactly the captured state.
- `cargo xtask build-guests --check` and `--accelerated --check` exited 0
  before any capture.
- **Output qualification before the probe, both modes (8/8 cells):** Benchy
  classic/arachne × supports off/on, ordinary + accelerated, every output
  byte-identical to the
  [t58 qualification references](../t58-finalization-copies/qualification.json)
  (ordinary `34886ccb…`/`76eb4e1d…`/`b76d846a…`/`ae9d9061…`; accelerated
  identical hashes). Completion `ok`, `degraded=false` on every run.
- **Probe neutrality:** the six probe captures' outputs are byte-identical to
  the same references — the probe changed only instrumentation. Probe sources
  are preserved in `probe.patch` (this directory) and were dropped from the
  working tree before any commit; production was restored and verified (both
  freshness checks green; staged ordinary linker hash back to the pre-probe
  `544f6183…`; the standing accelerated snapshot was never modified — the
  accelerated probe ran from a scratch copy at `target/t62/accel-probe/`;
  post-restore spot qualification reproduce `b76d846a…` byte-identically).
- Fuel figures are deterministic under `--profile` (ADR-0055); shares are
  exact ratios. Instrumented/bracket walls are attribution-only (map §3.2).
- Raw source (optional, gitignored): `target/t62/raw/`. The in-repo reduced
  set (`captures/#`) is derived by `reduce.py`; `verify.py` runs from the
  reduced set alone.

## The per-cell split

| run | linker fuel | share of guest fuel | execute | engine_add | clip_prepare | connect | meta_convert |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary arachne supports-off | 72,740,681,783 | 44.28% | 85.96% | 4.42% | 2.60% | 4.73% | 1.25% |
| ordinary arachne supports-on | 72,744,400,932 | 34.06% | 85.95% | 4.42% | 2.60% | 4.73% | 1.25% |
| ordinary classic supports-off | 79,929,332,403 | 9.77% | 83.99% | 5.44% | 3.13% | 4.94% | 1.08% |
| ordinary classic supports-on | 79,924,906,509 | 9.21% | 83.99% | 5.44% | 3.13% | 4.94% | 1.08% |
| accelerated arachne supports-off | 72,743,085,959 | 57.09% | 85.95% | 4.42% | 2.60% | 4.73% | 1.25% |

Consistency with the t61 refresh: the primary cell's linker total is 72.7407 B
against t61's 72.716 B (+0.034%, the probe's own mark self fuel); t61's
44.51% share vs this capture's 44.28% — the guest `fuel_total` denominator
also carries probe marks in other modules.

Every named term reconciles to the module total within 0.25% (the residual is
the unmarked lib-entry wrapper: config reads, ironing copy, flush,
`spacing_mm` match). The 319/310 `polygon_ops::clip_polygons` calls are a
different site than the per-path loop: `boundary_prep`'s
`difference_ex(&boundary, &footprint)` in
`process_bucket_role` (`modules/core-modules/infill-linker/src/orchestrate.rs`)
— its total equals that scope's self + the `clip_polygons` row exactly.

## Workload census (T62-PROBE counters, layers 0–2 + middle controls)

| layer (run) | clip calls | invocations | universe pts/call | execute fuel | call fuel | fallbacks |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 (arachne off) | 188 | 1 | 1,594 | 1.209 B | 1.512 B | 0 |
| 1 (arachne off) | 593 | 5 | 6,475 | 34.118 B | 36.895 B | 1 |
| 2 (arachne off) | 355 | 3 | 4,871 | 15.915 B | 17.251 B | 0 |
| 104 (arachne off, middle control) | 100 | 2 | 564 | 0.115 B | 0.146 B | 1 |
| 105 (arachne off, middle control) | 100 | 2 | 566 | 0.115 B | 0.150 B | 1 |
| 1 (classic off) | 559 | 5 | 6,632 | 29.958 B | 32.787 B | 1 |
| 2 (classic off) | 354 | 3 | 4,928 | 15.985 B | 17.299 B | 0 |
| 104 (classic off, middle control) | 100 | 2 | 1,264 | 0.240 B | 0.297 B | 1 |

Whole-job census (arachne supports-off): **12,022 clip calls across 393
invocations** (≈30.6 calls/invocation), 393 prepare calls — one per
invocation, the t44 hoist invariant. Across all invocations the prepared
universe totalling 198,320 points is re-fed to fresh engines 11,360,962
times — **the universe is rebuilt and re-added into clipper state 57.3× per
preparation** on arachne (classic: 49.5×: 15,823,925/319,889). That ratio is
the batchable overhead in one number.

Every subject is an open polyline of 2–6 points (the emitters' scan spans);
the clip universe is up to ~6.5 k points. The per-call cost is
universe-dominated, not subject-dominated: a least-squares fit across all
240 layers of `execute-fuel/call` against `universe-points/call` gives
**≈ −2.8M + 7,990·U** (R² = 0.869); on layer 1 the universe term is 89.9% of
the per-call execute cost. `engine_add` is likewise
≈ 17k + 267·U per call. Inputs are independent paths against one immutable
boundary — nothing about the subject set changes per call.

**Raw-boundary fallback reach (measured, not inferred):** 90 arachne / 102
classic fallback invocations (`link_paths_against` re-clipping the whole
invocation against the raw boundary after the offset arm produced nothing),
spread over 87 arachne layers (1/layer where they fire) and including the t35
sparse-burst middle layers (104/105 clip their sparse scans away entirely on
this Benchy job — `short_kept=0`, `meta_out=0` on arachne layer 104). Each
fallback arm is just another `link_paths_without_offset` invocation and gets
the same batch treatment; no separate mechanism is needed.

## Cross-mode invariance

All five headline scopes agree between the ordinary and accelerated arachne
supports-off profiles within 5.4e-4 relative
(`t62::execute` 1.08e-05, `clip_prepare` 6.3e-06, `connect_infill` 4.2e-07,
`meta_convert` 4.2e-06, `engine_add` 5.35e-04 — the largest, the accelerated
copy differs only in compiled sink glue). The linker remains the #1
accelerated fuel consumer at 57.09% of slice fuel on this cell; its clip work
is untouched by the accelerated cfg (same t21/t49 finding, re-measured at
HEAD).

## The candidate: one engine + one execute per invocation

`link_paths_without_offset` already owns one invocation's immutable boundary
and one `PreparedPolylineClip`: the batch candidate adds all invocation paths
as open subjects of ONE `Clipper64` (universe once, subjects once) and
executes once, replacing the per-call engine construction + universe re-add +
scanbeam rebuild that `clip_open_subjects` performs 12,022×.

**Ceiling (estimates under the same-cost assumption — not measurements; the
batched execute's cost for the universe pass is assumed equal to one current
per-call execute):**

| estimate (arachne supports-off) | execute saved | engine_add saved | post linker fuel | slice share before→after |
| --- | ---: | ---: | ---: | ---: |
| uniform-by-invocation: retain 1 execute per invocation (393/12,022 kept) | 60.481 B | 3.111 B | **9.15 B (12.58%)** | 44.28% → 5.57% |
| layer-aware bound: retain one execute per (layer, universe) group | 62.202 B | 3.181 B | 7.36 B (10.1%) | 44.28% → 4.48% |

Classic supports-off: uniform ceiling posts linker at 10.63 B (13.31% of
current), slice share 9.77% → 1.30%. The true per-invocation result sits
between the two rows and can only be settled by the graduated experiment's
own measurements. Fuel→wall transfer on this map is measured 0–16%
historically; the linker's own wall share on these cells is the
[profiling-off dispatch brackets](../../issues/61-benchy-current-bottleneck-refresh.md)
(3.182/3.191 s layer-1 linker dispatch), so the wall case inherits the same
opportunity evidence and must be proven by the standing paired A/B.

### Exactness and representation conditions (the graduated take's gate)

1. **Per-subject output attribution must be re-derived, not assumed.** The
   vendored `clipper2-rust` 1.1.0 `Clipper64` (`engine_public.rs`) exposes
   `add_open_subject`/`add_clip`/`execute` but **no owner tags on open-path
   solutions** (upstream Clipper2's subject-group owners arrived later than
   this pin). Each output open path is a contiguous clipped span of exactly
   one subject, so attribution is exactly recoverable — but the mechanism
   (bbox prefilter + endpoint/segment incidence, deterministic) is new code
   and must be proven against the per-path chain on real inputs.
2. **One-shot semantics equivalence.** The linker moved to per-path clipping
   for owner tracking, not for different geometry: the batched shape is
   precisely `clip_polylines`'s existing one-shot engine usage (all subjects
   in one execute, per-polyline independence guaranteed — its AC-6), so the
   invariant to prove is that batching produces the same per-path result set.
   Ordering, `remove_short_polylines` filtering, `point_with_metadata`
   metadata, `source_segments`, `majority_owner`, the
   [ran-and-empty replacement contract](../../issues/37-infillpostprocess-empty-output-protocol.md)
   and the raw-boundary fallback arm all keep their current semantics.
3. **Subject–subject adjacency.** Independent clipping of touching/coplanar
   subjects (gyroid scan ends meet at corners) is the one behavioural
   difference a single execute could expose; the replay oracle of scope item
   3 exists exactly for this.
4. **Native rerouting is not proposed.** The linker is a WASM guest driving
   `slicer-core` primitives; relocating the clip to the host would move fuel
   across the WIT boundary and prove nothing about wall (t21's "native work
   must be measured too" rule stays in force).

## Wall brackets and disclosure

Export-call `fn_us` (T62-PROBE, `now_us`, same clock backstamps both deltas),
arachne supports-off ordinary: layer 1 = 2,985 ms `fn_us` / 2,985 ms `orch_us`
on the instrumented-only run; layer 2 = 1,216 ms. Layer 0 = 162 ms.
The profiled runs of the same cells show 3,513 / 1,520 ms — **the fuel marks
inflate instrumented wall by ≈17.9% on layer 1** (six extra host calls per
clip call; fuel is unperturbed, ADR-0055). For scale, the t61 refresh's
profiling-off layer-1 linker module dispatch bracket is 3.182 s; this take's
unprofiled export-internal bracket is 2.985 s + host-side — the two agree
within disclosure precision and both are attribution-only. One arachne-off
capture in the 2026-10-13 refresh had an 11.5 s outside-timer outlier; this
take's captures showed no outlier of that class and none was excluded.

## Decision and graduation

**Graduate the batch-execute candidate to an execution ticket.** The
surviving shape is measured, not speculative: execute + engine_add are 90.4%
of linker fuel per path call at ≥50× universe amplification on every
measured cell and both generators, with invariance across modes. The
execution ticket inherits: the exactness replay against the per-path chain on
real inputs (scope item 3 of this ticket), the fresh 16-run output matrix,
and the standing paired ordinary + accelerated uninstrumented wall/CPU gate
with a human KEEP/DROP ruling. No production source changed in this take;
the probe and its reduced evidence are preserved.

## Validation

- `python docs/specs/perf-vs-orca/evidence/t62-linker-post-hoist/verify.py`:
  exit 0 from the in-repo reduced set — re-derives output identity vs the t58
  qualification references (six probe captures), per-cell shares, the primary
  split + reconciliation, cross-mode agreement (≤5.4e-4, threshold 1e-3),
  both ceiling estimates, the post-candidate ceiling, the prepares ==
  invocations hoist invariant, the 57.3× amplification, and negative controls
  (tampered totals, forged hashes rejected; no integrated provenance).
- `python docs/specs/perf-vs-orca/evidence/t62-linker-post-hoist/reduce.py`
  regenerates `captures/` from the optional raw tree
  (`--raw-root target/t62/raw`).
- Production restored before commit: probe edits dropped,
  `cargo xtask build-guests` + `--accelerated` rebuilt from clean sources,
  both `--check` gates exit 0, staged/linker hashes back to
  `544f6183…`/`2c0a2b4d…`, post-restore spot qualification byte-identical
  (`b76d846a…`).
- No Rust tests were run against production code: nothing in the production
  tree changed after restore (probe dropped; `probe.patch` preserved).

## Gaps

- The same-cost assumption behind both ceilings: the batched execute for
  N subjects is assumed to cost one current per-call execute for the universe
  pass. Subject–subject candidate processing, out-rec building and solution
  conversion could add cost the fuel split cannot see. Only the graduated
  experiment's own measurements settle this.
- Per-invocation batch sizes were not instrumented (per-layer call counts
  are), so the uniform estimate uses the invocation count only; the true
  saving can differ within the stated bounds.
- The wall case is inherited opportunity evidence, not a measured saving;
  fuel→wall transfer remains the paired A/B's question.
- `wasm_peak_kb` on layers 1–2 rises with the probe's prepared universes
  (8.3 MB class, same as t21 observed pre-hoist) — allocation was not
  instrumented; no claim is made about it.
- The metadata-conversion cost (1.25%) is measured as fuel only; if the
  batch take preserves `point_with_metadata` per output path, its share grows
  post-batching (per-path work becomes a larger fraction of a much smaller
  total) — it is not a candidate this take recommends touching.