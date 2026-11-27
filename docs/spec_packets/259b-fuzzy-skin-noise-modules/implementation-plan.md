# Implementation Plan: fuzzy-skin-noise-modules

## Execution Rules

- **Gate:** packet **259a** must read `status: implemented` before Step 1 — re-derive from `docs/spec_packets/259a-fuzzy-skin-gate-and-mode-keys/packet.spec.md` at that moment. 259a carries its own open `[BLOCK]`, so this gate is not a formality.
- Steps are ordered. Do not start a step before its predecessor's exit condition is met.
- `design.md` § Code Change Surface is the authoritative files-in-scope list; § Out-of-Bounds Files must not be edited or loaded.
- Every OrcaSlicer read is a delegated dispatch. Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Every test invocation tees to `target/test-output.log` and is inspected by reading that file, never by re-running.
- Ledger facts (module counts, deviation IDs, generated-doc row counts) are re-derived from disk at the moment of use.
- **Every generator receives millimetres.** Convert once at the trait boundary. A generator that sees 100 nm units produces plausible but wrong output — see `design.md` § Architecture Constraints.

## Steps

### Step 1: Extract `fuzzy-skin-core`

- **Objective:** move 259a's resampling walk, gate, and mode switch into a library crate parameterised by a noise trait, with `fuzzy-skin` delegating to it and its behaviour unchanged.
- **Preconditions:** 259a `implemented`.
- **Allowed reads:** `modules/core-modules/fuzzy-skin/src/lib.rs`.
- **Edits (≤ 3 logical units):** net-new `crates/fuzzy-skin-core/{Cargo.toml,src/lib.rs}`; `modules/core-modules/fuzzy-skin/{src/lib.rs,Cargo.toml}`; root `Cargo.toml` workspace member.
- **Dispatches:** `SUMMARY` ≤ 200 words — 259a's final gate and mode-switch signatures.
- **Cost:** M
- **Verification:** `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — 259a's entire suite must stay green.
- **Exit / falsifying condition:** `fuzzy-skin`'s output for a fixed seed is unchanged and every 259a test passes. This is a pure refactor; any behaviour change here is a defect, not progress.

### Step 2: Generalize the selection-key dedup

- **Objective:** turn the hardcoded `wall_generator` dedup into a registry of `(claim_id, config_key, default_value, value→module map, unknown-value policy)` entries, and register the two claims.
- **Preconditions:** Step 1 exit met.
- **Allowed reads:** `crates/slicer-scheduler/src/execution_plan.rs` — `dedup_same_claim_modules_with_wall_generator`, `WALL_GENERATOR_CONFIG_KEY`, `DEFAULT_WALL_GENERATOR`, `SPIRAL_VASE_CONFIG_KEY`; the single non-test call site `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`), which is where the raw `config_source` read happens; `docs/04_host_scheduler.md` § "Perimeter-generator selection".
- **Edits:** `crates/slicer-scheduler/src/execution_plan.rs`; `crates/slicer-wasm-host/src/execution_plan_live.rs`; net-new `crates/slicer-scheduler/tests/integration/fuzzy_skin_generator_selection_tdd.rs` **plus its `mod` line in `crates/slicer-scheduler/tests/integration/main.rs`** (without the `mod` registration the file never compiles and the AC reports a false green).
- **Out of bounds:** `crates/slicer-schema/wit/**`, `crates/slicer-ir/src/resolved_config.rs`.
- **Cost:** M
- **Authorities:** `docs/04_host_scheduler.md` § Claim Resolution.
- **Verification:** `cargo test -p slicer-scheduler --test scheduler_integration fuzzy_skin_generator_selection 2>&1 | tee target/test-output.log | grep -E "^test result"`, then the existing perimeter-generator selection tests, both delegated.
- **Exit / falsifying condition:** AC-2, AC-N3, AC-N4 green **and** every pre-existing `wall_generator` selection test still green with its `classic` fallback intact. Regressing `perimeter-generator` to make the new registration simpler fails the step. If the generalization cannot avoid a new `ResolvedConfig` field, **stop and report** — `design.md` invariant 1 is falsified.

### Step 3: `perlin-fuzzy-skin`

- **Objective:** port Perlin fractal noise honouring scale, octaves, and persistence.
- **Preconditions:** Step 2 exit met.
- **Allowed reads:** own crate; `crates/fuzzy-skin-core/src/lib.rs`.
- **Edits:** the crate's `src/lib.rs`, manifest, `tests/perlin_fuzzy_skin_tdd.rs`; the four registration surfaces for this module.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 2 snippets ≤ 30 lines — `get_noise_module`'s Perlin branch (`SetFrequency(1/scale)`, `SetOctaveCount`, `SetPersistence`) and the `GetValue(x_mm, y_mm, slice_z) * thickness` sample site.
- **Cost:** M
- **Authorities:** `docs/08_coordinate_system.md`; ADR-0056.
- **Verification:** `cargo test -p perlin-fuzzy-skin --test perlin_fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-3 and AC-4 green — scale 10 has strictly fewer displacement sign changes than scale 1; 8 octaves strictly more than 1; persistence 0.9 higher variance than 0.1. If any comparison is equal rather than ordered, the parameter is not reaching the generator.

### Step 4: `billow-fuzzy-skin` and `ridged-multi-fuzzy-skin`

- **Objective:** port the two remaining fractal generators, with RidgedMulti provably ignoring persistence.
- **Preconditions:** Step 3 exit met.
- **Allowed reads:** the two own crates; `crates/fuzzy-skin-core/src/lib.rs`.
- **Edits:** the two crates' `src/lib.rs`, manifests, test files; their registration surfaces.
- **Dispatches:** `SUMMARY` ≤ 200 words — `get_noise_module`'s Billow and RidgedMulti branches, specifically which setters each receives.
- **Cost:** M
- **Verification:** `cargo test -p billow-fuzzy-skin --test billow_fuzzy_skin_tdd` and `cargo test -p ridged-multi-fuzzy-skin --test ridged_multi_fuzzy_skin_tdd`, each `2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-5 green — both distinguishable from Perlin on identical input, Billow responding to all three parameters, and RidgedMulti **byte-identical** across two persistence values. A RidgedMulti that responds to persistence is wrong even though it looks like more parity.

### Step 5: `voronoi-fuzzy-skin`

- **Objective:** port Voronoi cell noise honouring only frequency, with the fixed displacement canonical sets.
- **Preconditions:** Step 4 exit met.
- **Edits:** the crate's `src/lib.rs`, manifest, `tests/voronoi_fuzzy_skin_tdd.rs`; its registration surfaces.
- **Dispatches:** `SUMMARY` ≤ 200 words — `get_noise_module`'s Voronoi branch including `SetDisplacement(1.0)`.
- **Cost:** M
- **Verification:** `cargo test -p voronoi-fuzzy-skin --test voronoi_fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-6 green — responds to scale, byte-identical across two octave values and across two persistence values.

### Step 6: `ripple-fuzzy-skin`

- **Objective:** port the arc-length sine path and its three keys.
- **Preconditions:** Step 5 exit met.
- **Edits:** the crate's `src/lib.rs`, manifest (including the three ripple `[config.schema]` tables), `tests/ripple_fuzzy_skin_tdd.rs`; its registration surfaces.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 3 snippets ≤ 30 lines — `fuzzy_polyline_ripple`, `fuzzy_extrusion_line_ripple`, `ripple_phase_shift_rad`, `ripple_anchor_arc_mm`, and the three keys' bounds from `PrintConfigDef::init_fff_params`.
- **Cost:** M
- **Verification:** `cargo test -p ripple-fuzzy-skin --test ripple_fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-7 green — exactly 4 and exactly 8 periods around the loop at the two `fuzzy_skin_ripples_per_layer` values, the phase shift recurring only every `fuzzy_skin_layers_between_ripple_offset` layers, and byte-identity across scale/octaves/persistence. **Note for the closing agent:** the three ripple keys are outside ticket 14's list; the ticket update is reported in the session handoff and must be applied before this packet closes the ticket.

### Step 7: Manifest ingestion, bounds, classic honest-absence

- **Objective:** land the shared claim across six manifests without a startup conflict, and pin what each generator ignores.
- **Preconditions:** Step 6 exit met.
- **Edits:** `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` (add the claim); `crates/slicer-scheduler/tests/integration/{manifest_ingestion_tdd.rs,config_bounds_enforcement_tdd.rs}`; `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` (AC-8); net-new `crates/fuzzy-skin-core/tests/noise_config_schema_tdd.rs`.
- **Dispatches:** `FACT` ≤ 5 lines — the currently asserted module count, re-derived now.
- **Cost:** M
- **Verification:** the AC-1, AC-8, AC-9, AC-10, AC-N5 commands.
- **Exit / falsifying condition:** six holders of `claim:fuzzy-skin-generator` load with zero `Error` diagnostics, and AC-8 shows `classic` byte-identical across all three noise parameters. An `Error`-level duplicate-claim diagnostic here means the Step 2 registration is not being consulted at load time.

### Step 8: Shared-core proof, docs, deviation, guests, closure gates

- **Objective:** prove the sharing is real, land the hand-maintained docs, and close the freshness gates.
- **Preconditions:** Step 7 exit met.
- **Edits:** `docs/03_wit_and_manifest.md` § Known claim IDs; `docs/04_host_scheduler.md` § Claim Resolution; `docs/DEVIATION_LOG.md` (one row for DIV-2, and note DIV-1's deliberate asymmetry with `wall_generator`); `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; the six guest `.wasm` artifacts.
- **Dispatches:** `FACT` ≤ 5 lines — the next free deviation ID **and which of the log's two ID conventions (`DEV-###` or `D-<packet>-<SLUG>`) the recent rows use**, re-derived from `docs/DEVIATION_LOG.md` now.
- **Cost:** M
- **Verification:** the AC-11, AC-12, AC-13 commands; `cargo xtask build-guests --check; echo "exit=$?"` (exit 0 required — never grep for `STALE:`); `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` must print `0`; the AC-N1 command.
- **Exit / falsifying condition:** AC-11, AC-12, AC-13, AC-N1, AC-N2 green and `build-guests --check` exits 0. AC-11 failing means a module kept a private copy of the resampling walk — fix by deleting the copy, not by relaxing the assertion.

## Per-Step Budget Roll-Up

| Step | Cost | Primary surface |
| --- | --- | --- |
| 1 | M | `fuzzy-skin-core` extraction |
| 2 | M | selection-key dedup generalization |
| 3 | M | `perlin-fuzzy-skin` |
| 4 | M | `billow-fuzzy-skin` + `ridged-multi-fuzzy-skin` |
| 5 | M | `voronoi-fuzzy-skin` |
| 6 | M | `ripple-fuzzy-skin` + three out-of-ticket keys |
| 7 | M | manifests, claim, bounds, honest absence |
| 8 | M | sharing proof, docs, deviation, guests, gates |

Aggregate: **L**. No single step is L, so no split is required before activation.

## Packet Completion Gate

All of the following, each delegated with a `FACT pass/fail` return:

1. Packet 259a is `implemented` (its BLOCK-1 resolved)
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo xtask check-literals`
5. `cargo xtask build-guests --check; echo "exit=$?"` — exit 0
6. Every AC command in `requirements.md` § Verification Matrix, green
7. Ticket 14 updated with the three ripple keys (reported in the session handoff; a packet may not edit the ticket itself)
8. The two map gates re-checked by the closing agent: (a) zero declaration-only keys; (b) a non-default behaviour AC per key

## Acceptance Ceremony

`cargo test --workspace` is **not** an AC command here. Because six guests and a shared load-time claim-resolution path changed, the closing agent should run `cargo xtask test --summary --workspace` (never bare `cargo test --workspace`) once, dispatched to a sub-agent returning `FACT pass/fail` only, after every narrower command above has passed.
