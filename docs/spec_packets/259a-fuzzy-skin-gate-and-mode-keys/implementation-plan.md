# Implementation Plan: fuzzy-skin-gate-and-mode-keys

## Execution Rules

- **This packet carries an unresolved `[BLOCK]` (`design.md` BLOCK-1).** Step 1 must not begin until an architecture owner has accepted the `LoopType` change — which is **both** an IR schema change and a WIT interface change, verified at authoring — and answered BLOCK-1's three sub-questions, or has refused it and selected the documented fallback (ship `none`/`external`/`allwalls`/`disabled_fuzzy`, reject `hole` and `all` by name).
- Steps are ordered. Do not start a step before its predecessor's exit condition is met.
- `design.md` § Code Change Surface is the authoritative files-in-scope list; § Out-of-Bounds Files must not be edited or loaded.
- Every OrcaSlicer read is a delegated dispatch. Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Every test invocation tees to `target/test-output.log` and is inspected by reading that file, never by re-running.
- Ledger facts (schema-version values, deviation IDs, generated-doc row counts) are re-derived from disk at the moment of use.

## Steps

### Step 1: Resolve BLOCK-1 and land the contour/hole distinction

- **Objective:** add the IR carrier for contour-vs-hole and populate it from both perimeter generators.
- **Preconditions:** BLOCK-1 accepted, with the shape (peer `LoopType` variant vs. a separate boundary field) and the schema-version answer both settled.
- **Allowed reads:** `crates/slicer-ir/src/slice_ir.rs` — `LoopType` and `WallLoop` by symbol, ranged; the wall-emission sites in the two perimeter generators.
- **Edits (≤ 3 logical units):** `crates/slicer-ir/src/slice_ir.rs` **together with its WIT mirror `crates/slicer-schema/wit/deps/ir-types.wit`** (`enum wall-loop-type`, consumed by `record wall-loop-view` — verified at authoring, so this is one change in two files, not an optional extra); the two perimeter generators (**5 `WallLoop` construction sites total** — 3 in `emit_walls` and 1 in `emit_nonplanar_shells` in `modules/core-modules/classic-perimeters/src/lib.rs`, 1 in `build_walls` in `modules/core-modules/arachne-perimeters/src/lib.rs`); net-new `crates/slicer-ir/tests/loop_type_hole_tdd.rs` (auto-discovered as `--test loop_type_hole_tdd` — `crates/slicer-ir/Cargo.toml` declares no `[[test]]` targets and no `autotests = false`, so no aggregator registration is needed).
- **Blast radius owned by this step:** every non-wildcard `match` on `LoopType` workspace-wide **and every guest-side match on the WIT `wall-loop-type`**, plus the struct-literal churn gate on `WallLoop` (each test literal needs a `..` rest or an `// exhaustive: <reason>` waiver). Follow `CLAUDE.md` § WIT/Type Changes Checklist: search `wit_host.rs`, `dispatch.rs`, and the `wit_guest` modules for the affected type, then `cargo build --tests`. Discover the Rust half with `cargo check --workspace --all-targets` inside this step; do not defer. Note also that `WallBoundaryTypeWire` exists alongside `WallBoundaryType` for pre-4.2.0 migration — if the settled shape touches `WallBoundaryType` instead of `LoopType`, the wire type must mirror the change.
- **Dispatches:** `FACT` ≤ 5 lines — which schema constant governs the IR carrying `WallLoop`, and its live value (re-derive; never freeze a version here). `LOCATIONS` ≤ 20 — every non-wildcard `LoopType` match site, **host and guest side**, since the enum is mirrored across the component boundary. The "is `LoopType` mirrored in WIT?" question is **already answered: yes** (`enum wall-loop-type` in `crates/slicer-schema/wit/deps/ir-types.wit`, verified at authoring and folded into BLOCK-1) — do not re-ask it.
- **Cost:** M
- **Authorities:** `docs/21_data_defaults_and_fixtures.md`; `docs/03_wit_and_manifest.md`.
- **Verification:** `cargo test -p slicer-ir --test loop_type_hole_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`; then `cargo check --workspace --all-targets` and `cargo xtask check-literals`, both delegated.
- **Exit / falsifying condition:** AC-5 green — a clockwise loop and a counter-clockwise loop emitted through `classic-perimeters` carry different boundary facts, and the same holds through `arachne-perimeters`. If either generator cannot tell them apart at its emission site, the carrier has no producer and the step has not succeeded — do not paper over it by deriving winding inside `fuzzy-skin`.

### Step 2: Manifest — three keys in, `apply_to_all` out

- **Objective:** declare the three keys canonically and retire the PnP-invented boolean.
- **Preconditions:** Step 1 exit met.
- **Edits:** `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`; `modules/core-modules/fuzzy-skin/Cargo.toml` (`toml = "0.8"` dev-dep, add-if-absent); net-new `modules/core-modules/fuzzy-skin/tests/fuzzy_config_schema_tdd.rs`.
- **Cost:** S
- **Authorities:** `docs/03_wit_and_manifest.md` `[config.schema]` contract.
- **Verification:** `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-1 and AC-N3 green. The `apply_to_all` removal must be loud: a config still supplying it either errors or maps to `allwalls` with a diagnostic. Silent acceptance fails the step.

### Step 3: The `should_fuzzify` gate

- **Objective:** replace the loop-selection heuristic with canonical's gate, driven by `fuzzy_skin` and `fuzzy_skin_first_layer`.
- **Preconditions:** Step 2 exit met.
- **Allowed reads:** `modules/core-modules/fuzzy-skin/src/lib.rs`; `crates/slicer-sdk/src/views.rs` `PerimeterRegionView` / `WallLoop` accessors.
- **Edits:** `modules/core-modules/fuzzy-skin/src/lib.rs` (the three fields on `FuzzySkinModule`, the new private `should_fuzzify`, the `run_wall_postprocess` call site); `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs`.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 2 snippets ≤ 30 lines — canonical `should_fuzzify`, clause by clause.
- **Cost:** M
- **Verification:** `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-2 and AC-3 green across all six `fuzzy_skin` values and both `fuzzy_skin_first_layer` states. The hard-coded `wall.loop_type != LoopType::Outer` pass-through must be **gone**; if it survives, `allwalls` cannot reach inner loops and AC-2's `allwalls` arm is failing or vacuous.

### Step 4: The `fuzzy_skin_mode` switch

- **Objective:** make displacement / extrusion / combined geometrically distinct inside `apply_fuzzy_skin`.
- **Preconditions:** Step 3 exit met.
- **Allowed reads:** `modules/core-modules/fuzzy-skin/src/lib.rs` — `apply_fuzzy_skin`, `Rng::next_f32`.
- **Edits:** `modules/core-modules/fuzzy-skin/src/lib.rs`; `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs`.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 1 snippet ≤ 30 lines — canonical `fuzzy_extrusion_line`'s `switch (cfg.mode)`, including the `max(p1.w + r + 0.01, 0.01)` width and the `(rad - p1.w) / 2` combined offset.
- **Cost:** M
- **Authorities:** `docs/08_coordinate_system.md` — the offsets are geometry.
- **Verification:** `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-4 and AC-N4 green. Under `extrusion` the emitted point coordinates must be **bit-identical** to the input and the point count unchanged; if subdivision points are still inserted, the mode was bolted onto the displacement path instead of branching before it.

### Step 5: Bounds, CONFIG_BLOCK, deviation row, generated docs, guests

- **Objective:** close the enforcement, reachability, documentation, and freshness gates.
- **Preconditions:** Step 4 exit met.
- **Edits:** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; the `slicer-runtime` integration CONFIG_BLOCK suite; `docs/DEVIATION_LOG.md` (one row for DIV-1); `docs/03_wit_and_manifest.md` if the schema range moved; `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; all guest `.wasm` artifacts.
- **Dispatches:** `FACT` ≤ 5 lines — the next free deviation ID **and which of the log's two ID conventions (`DEV-###` or `D-<packet>-<SLUG>`) the recent rows use**, re-derived from `docs/DEVIATION_LOG.md` now.
- **Cost:** M
- **Verification:** the AC-6, AC-7, AC-8 commands; `cargo xtask build-guests --check; echo "exit=$?"` (exit 0 required — never grep for `STALE:`; the `slicer-ir` change makes **every** guest stale, so expect a full rebuild); `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` must print `0`; the AC-N1 command.
- **Exit / falsifying condition:** AC-6, AC-7, AC-8, AC-N1, AC-N2 green and `build-guests --check` exits 0.

## Per-Step Budget Roll-Up

| Step | Cost | Primary surface |
| --- | --- | --- |
| 1 | M | `LoopType` carrier + both perimeter generators + blast radius |
| 2 | S | manifest + schema guard |
| 3 | M | `should_fuzzify` gate |
| 4 | M | mode switch |
| 5 | M | bounds, CONFIG_BLOCK, deviation, docs, guests |

Aggregate: **M**. No single step is L, so no further split is required — but the packet cannot activate while BLOCK-1 is open.

## Packet Completion Gate

All of the following, each delegated with a `FACT pass/fail` return:

1. BLOCK-1 resolved and recorded (accepted with a settled shape, or refused with the fallback taken)
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo xtask check-literals`
5. `cargo xtask build-guests --check; echo "exit=$?"` — exit 0
6. Every AC command in `requirements.md` § Verification Matrix, green
7. The two map gates re-checked by the closing agent: (a) zero declaration-only keys; (b) a non-default behaviour AC per key

## Acceptance Ceremony

`cargo test --workspace` is **not** an AC command here. Because the `LoopType` change touches `slicer-ir` and therefore every guest, the closing agent should run `cargo xtask test --summary --workspace` (never bare `cargo test --workspace`) once, dispatched to a sub-agent returning `FACT pass/fail` only, after every narrower command above has passed.
