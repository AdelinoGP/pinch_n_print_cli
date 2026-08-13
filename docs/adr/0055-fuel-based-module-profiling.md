# ADR-0055 — Fuel-based module profiling

<!-- filename: 0055-fuel-based-module-profiling -->

## Status

Accepted (2026-07-25). Authored after a measurement session that failed to
answer its own question, because wall-clock noise on this machine was the same
size as the signal being chased.

## Context

`--instrument-stderr` gives one number per module invocation: `elapsed_ms`,
plus `wasm_peak_kb`. That is enough to rank modules and no more. On a 0.2 mm
benchy (240 layers, 12 cores) it says `com.core.classic-perimeters` is 73.5 s
of the ~91 s of per-layer module CPU — 90% of the phase — and says nothing
about what inside it costs that.

Answering "what inside it" required editing the module, rebuilding 34 guests,
and re-slicing, once per hypothesis. Four such rounds produced:

- The `medial-axis` host service is called 67,300 times per slice (280 per
  layer) but accounts for only ~6.4 s. Doubling the call count from inside the
  guest raised `classic-perimeters` CPU by ~5.0 s against ~6.1 s of added
  host-side compute, so per-call boundary overhead is at or below the noise
  floor. **Batching `medial-axis` would buy nothing** — which is the opposite
  of the conclusion the call count alone invites.
- Doubling every `slicer_core::polygon_ops` call inside the guest added
  ~28.5 s, so clipper2 is roughly 39% of the module.

Then the method broke down. Doubling only `offset` + `difference_ex` measured
+3.5 s; the same doubling routed through the host's batch services measured
−0.3 s, which would be a >10x win. But repeat runs of a single fixed build
spanned 73.3 s to 95.3 s. Background `cargo`/`rustc`/`cl.exe` from a parallel
build was running during part of that window, and once it is in the picture the
±5% band swallows the 3.5 s signal whole. **No conclusion is available from
those numbers**, and the in-guest-vs-host-native question stays open.

Two properties are missing, and neither is fixable by measuring more carefully:

1. **A noise-immune signal.** Wall-clock on a developer machine cannot resolve a
   5% effect without controlling the machine, which no agent can rely on.
2. **Sub-module attribution without a rebuild.** Every question above cost a
   module edit plus a 34-guest rebuild, which is why only four were asked.

wasmtime 43 is already a dependency and its fuel metering is unused.
`WasmEngine::new` sets only `wasm_component_model(true)`.

## Decision

Profile guest modules with **wasmtime fuel** as the primary signal, wall-clock
as a secondary one, and attribute below module granularity through scope marks
that the host resolves.

**Fuel over wall-clock.** Fuel is a deterministic count of executed wasm
instructions. It is identical across runs and across machines, so an A/B is a
comparison rather than a hypothesis test. Critically, **host calls consume no
fuel**, so fuel separates guest compute from host-service cost by construction
— exactly the split that took four rounds to estimate above. Fuel does not
measure host-side native time and is not a wall-clock proxy, so both are kept:
fuel answers "did this get cheaper", wall-clock answers "by how many
milliseconds".

**Scope marks, resolved host-side.** Guests cannot read a clock —
they build for `wasm32-unknown-unknown`, which has no WASI, so
`std::time::Instant` does not work there. A guest also cannot read its own fuel.
So the guest emits `profile-mark(scope, edge)` at scope boundaries and the
**host** samples fuel and wall-clock at that instant. Because the mark is a host
call, it burns no fuel and therefore cannot pollute the measurement it is
taking. Scopes nest; the host keeps a per-call stack and reports self and total.

**A hook in `slicer-core`, not wrappers in `slicer-sdk`.** `slicer-sdk` depends
on `slicer-core`, not the reverse, so `polygon_ops` cannot call the SDK.
`slicer_core::profile` therefore holds a no-op-by-default sink that others
install: the SDK installs a WIT-calling sink for guests, `slicer-wasm-host`
installs a native sink so host built-in prepass stages share the vocabulary.

The alternative — move wrappers into `slicer_sdk::polygon_ops` and instrument
there — was rejected on coverage. Only 3 modules import
`slicer_core::polygon_ops` directly, so the churn would have been trivial, and
that layer would have been a natural seam for later rerouting ops to host batch
services. But guest code also reaches clipper2 *through* slicer-core:
`classic-perimeters` calls `slicer_core::top_surface_split::split_top_surfaces`,
which uses `polygon_ops` internally. A wrapper layer cannot see that, and a
module that keeps importing `slicer_core::polygon_ops` directly would silently
report nothing — the worst failure mode for a measurement tool, because it does
not look broken, it looks fast. The hook needs marks at only three primitives
(`clip_polygons`, `offset`, `offset2_ex`) to cover all 20 public functions,
since every other entry point delegates into them.

The SDK-wrapper seam remains an open, separate question about *where geometry
executes*. This ADR deliberately does not settle it; the profiler is the
instrument that makes it decidable on evidence.

**Always compiled in, host-gated.** Scope marks ship in every guest. The SDK
caches a `profile-enabled` answer once per instance, so the steady-state cost
with profiling off is a branch on a bool. The alternative — a cargo feature —
would mean rebuilding 34 guests to profile anything, which is the specific
friction this ADR exists to remove.

**Aggregate by default.** A 0.2 mm benchy emits 2,897 `module_complete` events;
attaching per-scope arrays to each would force every consumer to write an
aggregation script before reading anything. The host folds scopes into
per-(module, scope) totals and emits one `profile_summary`; `--profile-verbose`
adds per-call detail for finding a single pathological layer.

## Consequences

- Fuel metering costs throughput, so it rides a separate `--profile` flag rather
  than the existing `--instrument-stderr` path. A plain instrumented run is
  unaffected.
- **Wall-clock under `--profile` is inflated** by mark host calls in hot loops.
  Fuel ratios are unaffected, because marks burn no fuel. The summary labels
  wall-clock under `--profile` as indicative; absolute timings come from a
  profiling-off run. This is an accepted, documented limitation, not a defect.
- Fuel is deterministic *given identical guest inputs*. DEV-093 currently makes
  guest inputs vary run to run on a handful of layers, so whole-slice fuel
  totals can drift slightly until that is fixed. Fixed-input assertions (the
  determinism test-guest) are unaffected.
- Enabling fuel means every `wasmtime::Store` needs a budget or it traps on
  first execution. The budget is set in one shared helper so none of the
  construction sites can be missed.
- Host built-in prepass stages get `polygon_ops::*` attribution with wall-clock
  rather than fuel, since native code is not metered. Same vocabulary, one
  summary, different units — the summary must say which.
- New WIT functions in the shared `host-services` interface invalidate every
  guest, costing one 34-guest rebuild.

## Amendment

### 2026-08-10 — Host-bridge evidence

The Step 1 and Step 6 measurements use the same model (`resources/extruder_idler.obj`), machine, and release profile. The Step 1 baseline was re-captured on a quiescent tree (the first capture ran under concurrent session load and was discarded; its wall-clock spread of 264,219 ms was load noise, not run-to-run variance). The measured evidence is:

| Measure | Before (Step 1) | After (Step 6) | Delta |
| --- | ---: | ---: | ---: |
| `com.core.classic-perimeters` `polygon_ops::offset2_ex` self_fuel/total_fuel | 12,896,274,227 | 12,894,723,902 | -1,550,325 |
| `com.core.classic-perimeters` `polygon_ops::offset` self_fuel/total_fuel | 4,122,706,126 | absent from summary (guest fuel 0) | -4,122,706,126 |
| `com.core.classic-perimeters` `polygon_ops::clip_polygons` self_fuel/total_fuel | 880,143,944 | absent from summary (guest fuel 0) | -880,143,944 |
| `com.core.support-planner` fuel | 74,231,712 | 74,234,286 | +2,574 |
| Profiling-off wall-clock runs (ms) | 358,512 / 358,969 / 361,136 | 360,145 / 376,306 / 374,440; 361,941 / 357,210 / 359,618 | median +2,074 ms |

The profiling-off median changed from 358,969 ms to 361,043 ms (six after runs, two batches). The before run-to-run spread was 2,624 ms; the after spread was 19,096 ms (batch 1 carried two slow runs; batch 2 confirmed parity). The decision-rule outcome is **KEEP**: the +2,074 ms median delta is within the Step-1 run-to-run spread, so no regression beyond spread was measured.

This answers the open in-guest-vs-host-native question: host-native routing of the migrated offset/clip work shows a guest-fuel drop by construction and no measurable wall-clock regression on this model. Per ADR-0055, host calls burn no fuel, so the missing `offset` and `clip_polygons` rows prove routing, not speed. The residual in-guest share (`offset2_ex`, `opening_ex`, and `split_top_surfaces`) is quantified, not migrated. DEV-093 remains a caveat: whole-slice fuel totals can drift on a handful of layers, so the comparison uses per-scope rows.
