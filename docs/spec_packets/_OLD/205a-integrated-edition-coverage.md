---
status: implemented
packet: 205a-integrated-edition-coverage
task_ids:
  - ADR-0056
  - ADR-0057
---

# 205a-integrated-edition-coverage

## Goal

Integrate the sixteen core modules whose native dispatch stages are already committed by packet 202's transport — `fuzzy-skin`, `gyroid-infill`, `infill-linker`, `layer-planner-default`, `lightning-infill`, `overhang-classifier-default`, `part-cooling`, `rectilinear-infill`, `seam-placer`, `seam-planner-default`, `skirt-brim`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`, `tree-support`, `wipe-tower` — into `crates/slicer-integrated-modules/` behind per-module cargo features, each gated by a dual-dispatch parity contract test, so that the Integrated edition's coverage gate (packet 205's `verify_integrated_feature_coverage`) reports only the two transport-blocked modules (`path-optimization-default`, `machine-gcode-emit`) as missing.

## Problem Statement

ADR-0057's Integrated edition is defined as "every core module integrated". Packet 204 piloted three; packet 205 deliberately made `cargo xtask dist --edition integrated` fail loudly with a named list until coverage is complete. The plan's "Also unscheduled" note requires a follow-on packet (205a+) to integrate the remaining core modules. This packet is that follow-on for the sixteen modules whose native dispatch stages are already committed by packet 202's transport. It does not close the plan: two modules (`path-optimization-default`, `machine-gcode-emit`) map to native transports that return a fatal error today, and integrating them requires packet 205b to complete those transports first.

## Architecture Constraints

- **Byte-equality is not the gate (ADR-0056 Decision item 4).** The comparator asserts structural invariants plus a `1e-3` mm coordinate tolerance. It must never compare `f32`/`f64` bit patterns, and it must never be relaxed to make a red parity test green — a red parity test removes its module from the integrated set (it is simply not added to `dist/editions.toml`'s membership), it does not relax the assertion.
- **Single-threaded module logic on both paths (ADR-0056 Decision item 5).** Enforced cheaply by AC-7's static negative check over the sixteen crates' `Cargo.toml` and `src/**`. Verified at authoring: none of the sixteen declares `rayon` (the only `rayon` hits under `modules/core-modules/` are transitive entries in `wit-guest/Cargo.lock` files, which AC-7's `src/`-and-`Cargo.toml`-scoped greps correctly ignore).
- **The native-transport scope is the packet's load-bearing fact.** Sixteen of the eighteen remaining modules map to stages the native marshal commits; two (`path-optimization-default` → `Layer::PathOptimization`, `machine-gcode-emit` → `PostPass::GCodePostProcess`) map to stages that return a fatal `Err`. This packet integrates only the sixteen. The two excluded modules are packet 205b's scope, which must first complete the two transports. **Do not attempt to integrate a transport-blocked module** — the parity test would fail on the native path's fatal error, and the correct response is to defer, not to weaken the gate.
- **Feature unification is load-bearing for native capability.** As in 204, a module's native arm may depend on `host-algos` arriving through `slicer-sdk`'s `cfg(not(target_arch = "wasm32"))` dependency. `crates/slicer-integrated-modules/` must not be built with `--no-default-features` in a way that severs `slicer-sdk`. Each newly-integrated module's native path must be exercised by its parity test (a compile-time success does not prove the feature reached it).
- **New stage families beyond 204's pilots.** 204 demonstrated parity for `Layer::Perimeters` (×2) and `PrePass::SupportGeometry`. This packet's sixteen modules span `Layer::Infill`, `Layer::InfillPostProcess`, `Layer::Support`, `Layer::SupportPostProcess`, `Layer::PerimetersPostProcess`, `PrePass::LayerPlanning`, `PrePass::SeamPlanning`, and `PostPass::LayerFinalization`. Each family needs an appropriate stage input fixture and a family-appropriate parity assertion. The layer-family comparator (`assert_parity_structural`) covers the `Layer::*` stages; the prepass comparator (`assert_prepass_parity_structural`) covers `PrePass::*`; finalization needs a new comparator or a structural assertion over the merged `LayerCollectionIR` (see §Open Questions). This is the packet's main design risk and the reason it is rated L.
- **Guest WASM staleness applies.** Sixteen `Cargo.toml` edits invalidate their wasm twins. After any edit, run `cargo xtask build-guests --check` and rebuild without `--check` if `STALE:` is reported before re-running a parity test.
- **Coordinate units:** 1 unit = 100 nm (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
- No public schema/version constant is bumped and no struct gains a field in a crate with existing struct-literal sites; the only new fields are entries in the two registry vectors, which are net-new with no pre-existing literal sites. Blast-radius discipline does not bind.

## Data and Contract Notes

- **IR/manifest contracts:** unchanged. Every module manifest is embedded verbatim by `include_str!`; no `[module]`, `[stage]`, `[ir-access]`, `[claims]`, or `[compatibility]` key is edited. Module ids stay `com.core.<name>`; stage ids stay as declared. Config keys touched: none — this packet does not edit `dist/editions.toml`.
- **WIT boundary:** unchanged. No WIT file is edited, so `crates/slicer-schema/wit/**` staleness does not apply — but the sixteen `Cargo.toml` edits still invalidate those guests' fingerprints, which is why `build-guests --check` is a gate command.
- **Determinism/scheduler constraints:** integrated modules must produce identical scheduling behavior to their wasm twins — claims, DAG position, and IR access are read from the same manifest text, so the scheduler cannot observe the difference (ADR-0056 Decision item 1). DEV-093 run-to-run nondeterminism means a parity test must dispatch both paths within one process on one fixture and must not compare against any stored snapshot.

## Locked Assumptions and Invariants

- **Locked:** edition membership lives in `dist/editions.toml` and nowhere else; this packet does not change it. It only makes more modules registry-available.
- **Locked:** names in the registry are module **directory** names — simultaneously the `slicer-integrated-modules` cargo feature, the `modules/core-modules/<name>` directory, and the `<name>.wasm` / `<name>.toml` stem `xtask dist` stages.
- **Locked:** parity is certified by structural invariants plus tolerance, never by byte-equality and never by a stored snapshot (ADR-0042, ADR-0056 Decision item 4).
- **Locked:** integrated modules stay single-threaded internally (ADR-0056 Decision item 5).
- **Locked:** a module whose native transport returns a fatal error is NOT integrated; it is deferred to 205b. Weakening the gate to admit it is a defect.
- **Reversible:** every module is behind an off-by-default cargo feature. A build with no new feature enabled is byte-identical in behavior to the pre-packet tree.

## Risks and Tradeoffs

- **The new stage families (finalization, seam-plan, layer-planning) lack a demonstrated parity-test pattern.** 204 only demonstrated `Layer::Perimeters` and `PrePass::SupportGeometry`. The finalization comparator over merged `LayerCollectionIR` and the seam-plan comparator are new design work. Mitigated by authoring each new comparator with its own self-tests (the 204 `parity_invariants_selftest_tdd.rs` pattern) before any subject test, and by the `1e-2` mm absolute `coord_mm` ceiling (never widen past it; a divergence needing more is a defect, not drift).
- **A stale wasm twin silently invalidates every parity result.** Mitigated by the freshness gate immediately before each parity run.
- **A module's native arm may not actually reach its host-algos feature.** Mitigated by each parity test exercising the native path (a compile-time success does not prove the feature reached it), mirroring 204's AC-6.
- **The packet is large (sixteen modules).** Mitigated by splitting into per-family steps, each at or under 3 edits, and by the mechanical repetition of 204's pattern. The two transport-blocked modules are explicitly out of scope, keeping the packet to committable work.
