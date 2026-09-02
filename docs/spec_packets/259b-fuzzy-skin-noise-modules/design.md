# Design: fuzzy-skin-noise-modules

## Tier Derivation

**Tier C.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds four: a host-side generator selection and three noise parameters, realized as five new core-module crates plus one new library crate plus a generalization of the scheduler's claim-dedup path. That is well above the Tier B single-module ceiling. Ticket 14's tier table needs the correction (reported in the session handoff; this packet does not edit the map).

## Approach

**The enum becomes a claim.** `fuzzy_skin_noise_type`'s six values are six algorithms, so under map Authoring rule 4 they are six modules holding one claim, not six branches inside one module. The claim is net-new: `claim:fuzzy-skin-generator`.

The interesting question is how the claim is *resolved*, and the answer already exists in this tree. The four fill claims are resolved per region through `ResolvedConfig` fields (`top_fill_holder` and friends), but the `perimeter-generator` claim is resolved **at module-load dedup time** by reading `wall_generator` straight from the raw config source, before `ResolvedConfig` exists — `dedup_same_claim_modules_with_wall_generator` and `WALL_GENERATOR_CONFIG_KEY` / `DEFAULT_WALL_GENERATOR` in `crates/slicer-scheduler/src/execution_plan.rs`, called from `crates/slicer-runtime/src/run.rs`, documented in `docs/04_host_scheduler.md` § "Perimeter-generator selection".

That is the right precedent here, and choosing it is what keeps this packet unblocked: fuzzy skin is a whole-module behaviour selected once per print, not a per-region fill role, and resolving it at load time means **no new `ResolvedConfig` field, no new WIT interface, and no IR change**. The work is to generalize the existing function from one hardcoded claim/key pair to a small registry of `(claim_id, config_key, default_value, value→module map)` entries, then register two: the existing `perimeter-generator` / `wall_generator` pair with unchanged behaviour, and the new one.

One behaviour deliberately does **not** carry over. The `wall_generator` path falls back to `classic` when the value is unrecognised. For noise types that would silently give a user a texture they did not ask for, so the new registration rejects unknown values by name (AC-N3). The fallback stays for `wall_generator` because changing it is out of scope here.

**The parameters become generator inputs.** `apply_fuzzy_skin` samples noise at exactly **two** expressions today, both spelled `let displacement = rng.next_f32() * fuzzy_skin_thickness;` — one inside the `while dist < seg_len` sampling loop and one in the `if !emitted_sample` fallback branch (verified at authoring). Both must move behind the trait; patching only the loop leaves short segments on the old uniform sampler, a defect that would surface only on geometry with segments shorter than `point_distance`. Canonical's equivalent is `noise->GetValue(unscale_(pa.x()), unscale_(pa.y()), slice_z) * cfg.thickness`. So the seam is a trait with one method — sample a scalar in `[-1, 1]` from a millimetre-space point — and the three parameters are constructor inputs to whichever implementation is selected.

**Why one shared crate.** Six modules that each re-implement the resampling walk, the gate, and the mode switch would be six copies of packet 259a's work, drifting apart on the first bug fix. So the walk moves into a `fuzzy-skin-core` library crate parameterised by the noise trait, and each module contributes a sampler plus `LayerModule` wiring. AC-11 pins that the sharing is real rather than aspirational.

**What each generator honours.** Canonical is specific and asymmetric here, and reproducing the asymmetry exactly is most of the parity value:

| Value | Module | scale | octaves | persistence |
| --- | --- | --- | --- | --- |
| `classic` | `fuzzy-skin` | ignored | ignored | ignored |
| `perlin` | `perlin-fuzzy-skin` | frequency `1/scale` | yes | yes |
| `billow` | `billow-fuzzy-skin` | frequency `1/scale` | yes | yes |
| `ridgedmulti` | `ridged-multi-fuzzy-skin` | frequency `1/scale` | yes | **ignored** |
| `voronoi` | `voronoi-fuzzy-skin` | frequency `1/scale` | **ignored** | **ignored** |
| `ripple` | `ripple-fuzzy-skin` | ignored | ignored | ignored |

The "ignored" cells are asserted as byte-identity across two different values (AC-5, AC-6, AC-7, AC-8), not left unstated. That is what stops a well-meaning implementer from wiring persistence into RidgedMulti because it "seems like it should apply".

## Controlling Code Paths

- `dedup_same_claim_modules_with_wall_generator`, `WALL_GENERATOR_CONFIG_KEY`, `DEFAULT_WALL_GENERATOR` (`crates/slicer-scheduler/src/execution_plan.rs`) — the function generalized and the consts it reads; re-exported from `crates/slicer-scheduler/src/lib.rs` and `crates/slicer-runtime/src/lib.rs`, so the generalization must keep those re-exports resolving.
- `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) — **the** non-test call site, and where the raw read actually happens: it holds `config_source: &HashMap<ConfigKey, ConfigValue>` and does `.get(WALL_GENERATOR_CONFIG_KEY)`, passing the extracted `Option<&str>` down. The dedup fn itself takes pre-extracted scalars, so the second selection key is extracted here too.
- `validate_claim_conflicts` (`crates/slicer-scheduler/src/validation.rs`) — the duplicate-claim safety net that must still fire if dedup is bypassed (AC-N4).
- `apply_fuzzy_skin` (a free fn, not a method) and `Rng::next_f32` (`modules/core-modules/fuzzy-skin/src/lib.rs`) — the resampling walk that moves into `fuzzy-skin-core`, and the uniform sampler that becomes the `classic` implementation. **Two** sampling sites, not one (see § Approach).
- `FuzzySkinModule::run_wall_postprocess` (same file) — the `LayerModule` wiring each of the six modules mirrors.
- `load_modules_from_roots` (`crates/slicer-scheduler/src/manifest.rs`) — the loader whose module count AC-1 moves.

## What Carries the New Data

- The generator *choice* travels as a raw-config string read at load-dedup time and consumed by discarding five of six candidate modules. It never enters `ResolvedConfig`, which is exactly why no field is added.
- The three noise *parameters* travel as ordinary module config keys declared on each generator's manifest and read in its `LayerModule::from_config` impl.
- The noise *sample* travels as an `f32` in `[-1, 1]` across a Rust trait boundary inside one guest — never across the WIT boundary, which is what makes per-point sampling affordable.

No prepass IR change, no new `SliceRegionView` metadata, no new `PostPass` claim, no WIT change, no schema bump.

## Recorded Divergences (port improves on or intentionally differs from canonical)

- **DIV-1 — unknown noise types are rejected, not silently defaulted.** Canonical's config layer coerces; this port's selection fails loudly naming the key, the value, and the six legal values. Rationale: silently substituting a different surface texture is a worse outcome than a startup error, and the port's claim system can express the failure. Note this deliberately differs from the sibling `wall_generator` behaviour, which keeps its `classic` fallback — the asymmetry is intentional and is documented in `docs/04_host_scheduler.md`.
- **DIV-2 — the four coherent generators are Rust ports, not libnoise.** Canonical links libnoise as an external CMake dependency (`deps/libnoise/libnoise.cmake`), which this project will not take on. The ports reproduce the *structure* canonical relies on — fractal octaves, frequency scaling, persistence weighting, Voronoi cell displacement — but not libnoise's exact gradient tables, so displacement values will differ numerically from OrcaSlicer's. This is why every AC asserts a structural property rather than a golden coordinate. Needs a `docs/DEVIATION_LOG.md` row; re-derive both the ID **and the convention** from `docs/DEVIATION_LOG.md` at the moment of writing: the log carries two schemes — a dominant `DEV-###` series and a minority `D-<packet>-<SLUG>` series — so follow whichever the recent rows use rather than assuming.
- **DIV-3 — `fuzzy_skin_mode` reaches every generator.** Packet 259a establishes that the port applies the mode switch on both wall generators where canonical applies it only on Arachne. Because all six noise modules share `fuzzy-skin-core`, they all inherit that improvement rather than each re-deriving it.
- **DIV-4 — the noise seam is a Rust trait, not a module boundary.** A strict reading of Authoring rule 4 might place each noise function behind its own claim *and its own WIT crossing*. That would mean one host call per resampled vertex — thousands per loop — which the architecture cannot afford. The claim boundary is therefore drawn at the *generator module* (a whole `Layer::PerimetersPostProcess` participant, which is a real pipeline unit), and the scalar sampler stays a trait inside it. Rule 4's intent — alternative algorithms are selectable modules, not enum branches — is satisfied; its mechanism is applied at the granularity the runtime supports.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Noise is sampled in millimetres.** Canonical samples `GetValue(x_mm, y_mm, slice_z_mm)`. The port's points are in 100 nm units, so the conversion happens once per sample at the trait boundary, never inside a generator. A generator that receives units instead of millimetres produces a frequency wrong by a factor of 10⁴ and will still look plausible — this is the likeliest silent defect in the packet.
- **Claim uniqueness.** `docs/04_host_scheduler.md` § Claim Resolution: global validation fails if a claim has more than one effective holder. Six modules declaring `claim:fuzzy-skin-generator` is only safe because dedup resolves the claim before `validate_startup_dag` runs — the same arrangement `classic-perimeters` / `arachne-perimeters` already live under. The six modules must **not** be `incompatible-with` one another.
- **Determinism.** `fuzzy-skin`'s seed is derived from `layer_index` and `wall_index`. Every new generator must be equally deterministic: the same layer, wall, and config must produce the same displacement across runs and across machines. No wall-clock or address-derived entropy.
- **ADR-0056** governs registration of a new core module: workspace member, integrated-modules optional dep + feature + `manifest_const!` + `integrated_registry!` entry and its `#[cfg(not(feature = …))]` arm, and a `pnp-cli` passthrough feature.

## Code Change Surface

**New:**

- `crates/fuzzy-skin-core/{Cargo.toml,src/lib.rs,tests/noise_config_schema_tdd.rs}` — library crate: the noise trait, the resampling walk, `should_fuzzify`, the mode switch, and the shared manifest guard.
- `modules/core-modules/perlin-fuzzy-skin/{Cargo.toml,src/lib.rs,perlin-fuzzy-skin.toml,wit-guest/**,tests/perlin_fuzzy_skin_tdd.rs}`
- `modules/core-modules/billow-fuzzy-skin/{…,tests/billow_fuzzy_skin_tdd.rs}`
- `modules/core-modules/ridged-multi-fuzzy-skin/{…,tests/ridged_multi_fuzzy_skin_tdd.rs}`
- `modules/core-modules/voronoi-fuzzy-skin/{…,tests/voronoi_fuzzy_skin_tdd.rs}`
- `modules/core-modules/ripple-fuzzy-skin/{…,tests/ripple_fuzzy_skin_tdd.rs}`
- `crates/slicer-scheduler/tests/integration/fuzzy_skin_generator_selection_tdd.rs` + its `mod` line in `tests/integration/main.rs`

**Edited:**

- `modules/core-modules/fuzzy-skin/src/lib.rs` — delegate to `fuzzy-skin-core`; keep `Rng` as the `classic` sampler; output for a fixed seed unchanged.
- `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` — add `claim:fuzzy-skin-generator` to `holds`.
- `modules/core-modules/fuzzy-skin/Cargo.toml` — depend on `fuzzy-skin-core`.
- `crates/slicer-scheduler/src/execution_plan.rs` — generalize the dedup to a selection-key registry; keep the existing re-exported const names resolving.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` — in `load_live_modules_for_plan_with_integrated`, extract `fuzzy_skin_noise_type` from `config_source` alongside the existing `WALL_GENERATOR_CONFIG_KEY` read and pass it into the generalized dedup.
- `Cargo.toml` (root) — six new workspace members (five modules + the library crate).
- `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`, `crates/pnp-cli/Cargo.toml` — registration for the five modules.
- `crates/slicer-scheduler/tests/integration/{manifest_ingestion_tdd.rs,config_bounds_enforcement_tdd.rs}` — AC-1, AC-10.
- `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` — AC-8.
- `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/DEVIATION_LOG.md` — hand-maintained.
- `docs/15_config_keys_reference.md` — regenerated, never hand-edited.
- Guest `.wasm` artifacts for the six affected modules.

## Files in Scope (read + edit)

The change surface above is the authoritative list. No file outside it may be edited.

## Read-Only Context

- `crates/slicer-scheduler/src/validation.rs` — `validate_claim_conflicts`, for AC-N4's safety-net assertion.
- `crates/slicer-sdk/src/views.rs` — `PerimeterRegionView` / `WallLoop` accessors.
- `docs/04_host_scheduler.md` § "Perimeter-generator selection" — the precedent being generalized.
- `docs/spec_packets/259a-fuzzy-skin-gate-and-mode-keys/design.md` — via a SUMMARY dispatch only, to confirm the final shape of the gate and mode switch being extracted. Never edit anything under that directory.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING`. Zero diff lines (AC-N2, map Authoring rule 2).
- `crates/slicer-schema/wit/**` — no WIT change. If a worker concludes one is needed, **stop and report**: the design's central claim is that load-time selection avoids it.
- `crates/slicer-ir/**` — no IR change in this packet (259a owns the only IR surface in the P07 pair).
- `crates/slicer-ir/src/resolved_config.rs` — no new field; not read.
- Every other packet directory under `docs/spec_packets/`.
- `docs/specs/orca-feature-gap/map.md` and `issues/**` — read-only; required updates are reported, not applied.
- `docs/15_config_keys_reference.md` — generated; regenerate, never hand-edit.

## Expected Sub-Agent Dispatches

- **`get_noise_module`** — `SUMMARY` ≤ 200 words + ≤ 2 snippets ≤ 30 lines. The single most important canonical read: which setter each type receives, and which it does not.
- **The ripple functions** — `SUMMARY` ≤ 200 words + ≤ 3 snippets ≤ 30 lines: `fuzzy_polyline_ripple`, `fuzzy_extrusion_line_ripple`, `ripple_phase_shift_rad`, `ripple_anchor_arc_mm`.
- **259a's extracted shape** — `SUMMARY` ≤ 200 words: the final signature of the gate and mode switch to be moved into `fuzzy-skin-core`.
- **Module-count ledger fact** — `FACT` ≤ 5 lines, re-derived at the moment of editing.
- **Deviation ID** — `FACT` ≤ 5 lines: the next free `D-` ID for DIV-2, re-derived at the moment of writing.
- **Cargo runs** — all delegated with a `FACT pass/fail` return.

## Data and Contract Notes

- Config key strings are snake_case (`CLAUDE.md` § Config Key Naming Convention).
- `fuzzy_skin_noise_type` is declared in **no** manifest — it is a host-side selection key, like `wall_generator`. Its legal-value list therefore lives in the selection registry and in `docs/04_host_scheduler.md`, and its rejection path is the registry's own error, not `ConfigBoundsIndex`.
- The three noise parameters are declared identically on every module that reads them, so a user's value is in bounds regardless of which generator is selected. `cargo xtask gen-config-docs` must not report them as conflicting owners; if it does, the duplicate-declaration convention needs checking before the manifests land.
- Module ids in the value→module map are the full manifest ids (`com.core.perlin-fuzzy-skin`); `module_id_matches_holder`'s short-name tolerance is a fill-holder concept and does not apply on this path.

## Locked Assumptions and Invariants

1. `WALL_GENERATOR_CONFIG_KEY` (`"wall_generator"`), `DEFAULT_WALL_GENERATOR` (`"classic"`), and `dedup_same_claim_modules_with_wall_generator(modules, diagnostics, wall_generator: Option<&str>, spiral_vase: bool, support_type: Option<&str>)` exist in `crates/slicer-scheduler/src/execution_plan.rs`; the raw-config read lives one level up in `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) — all verified at authoring. The claim is resolved from raw config at load time, before `ResolvedConfig` exists. **If a worker finds otherwise, stop: the packet's no-new-field, no-WIT claim is falsified and it must be re-scoped, probably into a `[BLOCK]`.**
2. `apply_fuzzy_skin` has exactly **two** noise-sampling expressions — the `while dist < seg_len` loop and the `if !emitted_sample` fallback — verified at authoring. Both must move behind the trait together. If 259a's implementation changed the count, re-derive it before Step 1 rather than trusting this line.
3. `Rng::next_f32` already returns `[-1, 1]`, matching canonical `UniformNoise`. `classic`'s output must not change.
4. Default resolution selects `com.core.fuzzy-skin`, and the default `fuzzy_skin` is `disabled_fuzzy`, so AC-N1 stays byte-identical.
5. Exactly one module holds `claim:fuzzy-skin-generator` after dedup; the six candidates are not mutually `incompatible-with`.
6. RidgedMulti ignores persistence; Voronoi ignores octaves and persistence; classic and ripple ignore all three. These are asserted, not assumed.

## Risks and Tradeoffs

- **Four noise algorithms with no in-tree precedent.** Perlin, Billow, RidgedMulti, and Voronoi are all net-new geometry-adjacent code. Mitigation: each gets its own step and its own crate, so a stalled generator blocks only itself; and every AC asserts structure rather than golden values, so the tests do not depend on matching libnoise bit for bit (DIV-2).
- **The millimetre/unit boundary is the silent-failure spot.** A generator fed 100 nm units produces plausible-looking but wrong-frequency output. Mitigated by converting once at the trait boundary and by AC-3 asserting a *directional* frequency response rather than an absolute one.
- **Generalizing a working dedup path** risks regressing `wall_generator`. Mitigation: the existing `perimeter-generator` registration keeps its fallback behaviour and its existing tests unchanged; the new registration is the only one that rejects unknown values.
- **Six holders of one claim** is a fatal conflict if the dedup registration is missing or mis-ordered. Mitigated by landing the dedup before or with the manifests, and by AC-N4 asserting the safety net still fires.
- **Ripple pulls three keys outside ticket 14.** Reported for a ticket update rather than silently absorbed.

## Context Cost Estimate

**L aggregate.** No single step exceeds M: each generator is one step reading one algorithm; the dedup generalization is one step in one host file; the shared-crate extraction is one step.

## Open Questions

**No `[BLOCK]` in this packet.** The three blocker triggers were each checked against the tree at authoring and none applies: no new WIT interface (the noise sample is a Rust trait call inside one guest), no IR schema bump (no IR type changes), and no new host `ResolvedConfig` field (the selection key is read from raw config at load-dedup time, per the `wall_generator` precedent). The packet is nonetheless gated by packet **259a**'s open BLOCK-1, since 259a must be `implemented` first.

- `[FWD]` Canonical merges regions with equal noise parameters (`collect_merged_fuzzy_regions` / `same_fuzzy_effect`, ignoring `type` and `first_layer`) to avoid re-generating identical noise. The port has no equivalent optimisation and does not need one at this size; if fuzzy skin becomes a profiling hotspot, this is the shape of the fix.
- `[FWD]` `fuzzy_skin_noise_type` is resolved once per print at load time, so it cannot vary per object or per region the way the fill-holder keys can. If per-region noise selection is ever wanted, it needs the per-region holder mechanism instead — a different and larger design.
- `[FWD]` The `wall_generator` unknown-value fallback (silently keeping `classic`) is inconsistent with this packet's reject-unknown rule (DIV-1). Harmonizing them is a small follow-up packet, deliberately not taken here to keep this packet's diff to its own claim.
