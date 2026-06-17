# Implementation Plan: 113_marshal-boundary-extraction

## Execution Rules

- TDD where a behaviour is new (`OriginBucket`): write the failing unit test, then implement.
- The crate stays green: every step ends on `cargo check --workspace --all-targets` passing. Never leave the crate broken between steps.
- Delegate every `cargo` run; absorb only `FACT` pass/fail + first failing assertion. Tee test output to `target/test-output.log` (CLAUDE.md).
- No step edits more than 3 files or rates L. Steps are ordered; do not reorder.

## Steps

### Step 1 — Delete stale per-world converter duplicates
- Task ids: ADR-0021 (B-step-1). Objective: delete the per-world copies confirmed byte-identical to layer in **both** directions (outbound role, path, expolygon); repoint callers to the unified converter. The two **inbound** role converters (`finalization_role_wit_to_ir`, `convert_postpass_role`) are NOT deleted here — they diverge (latent `PrimeTower`/`Skirt` loss) and are relocated unchanged in Step 6, fixed in packet 115.
- Precondition: the byte-identity diff is already done — outbound role / path / geometry copies confirmed identical; the inbound role pair confirmed divergent. Delete only the confirmed-identical set.
- Postcondition: AC-1 grep is empty; `cargo check --workspace --all-targets` passes.
- Read: `host.rs:1859–1888` (expolygon_prepass), `:3696–3719` (finalization_role_ir_to_wit, finalization_path_ir_to_wit); `dispatch.rs:92–117` (convert_postpass_role_to_wit).
- Edit (≤3): `host.rs`, `dispatch.rs`.
- Dispatches: `cargo check` (FACT pass/fail + first error).
- Context cost: **S**.
- Authoritative docs: ADR-0002 "Deferred"; ADR-0021 §Amendment. OrcaSlicer refs: none.
- Verify: `! rg -n 'fn (finalization_role_ir_to_wit|finalization_path_ir_to_wit|convert_postpass_role_to_wit|ir_to_wit_expolygons?_prepass)\b' crates/slicer-wasm-host/src`
- Cheapest falsifier: AC-1 grep returns any match → fail.

### Step 2 — `marshal` skeleton: `OriginId` + `MarshalError`
- Objective: create `src/marshal/{mod,origin}.rs`; declare `mod marshal;` in `lib.rs`; define `OriginId`, `MarshalError`, `From<MarshalError> for String`.
- Precondition: Step 1 done.
- Postcondition: crate compiles with the empty module wired in; `struct OriginId` present (AC-3 first clause).
- Read: ADR-0021 §Data and Contract Notes (design.md).
- Edit (≤3): `marshal/mod.rs`, `marshal/origin.rs`, `lib.rs`.
- Dispatches: `cargo check` (FACT).
- Context cost: **S**.
- Verify: `rg -n 'struct OriginId' crates/slicer-wasm-host/src/marshal/origin.rs`
- Exit condition: compiles; `OriginId` defined.

### Step 3 — `OriginBucket` + unit tests (TDD)
- Objective: implement `OriginBucket<R>` (`new`/`drain`/`into_regions`) and the four unit tests.
- Precondition: Step 2 done.
- Postcondition: AC-5, AC-N1, AC-N2 pass; `any_tagged`/bucket loop exist only here.
- Read: design §Data and Contract Notes.
- Edit (≤1): `marshal/origin.rs`.
- Dispatches: `cargo test -p slicer-wasm-host --lib marshal::origin` (FACT `^test result` + failing assertion).
- Context cost: **M**.
- Verify: `cargo test -p slicer-wasm-host --lib marshal::origin 2>&1 | tee target/test-output.log; rg '^test result' target/test-output.log`
- Cheapest falsifier: any of the four named tests fails or is absent.

### Step 4 — Move `*Collected` accumulators into `marshal`
- Objective: move the five `*Collected` structs to `marshal/accumulators.rs`; replace `Option<PerimeterRegionOrigin>`/`Option<SliceRegionOrigin>` fields with `Option<OriginId>`; delete the two aliases; re-export so `host.rs` builder impls still name them.
- Precondition: Step 3 done.
- Postcondition: AC-3 fully passes; builder methods unchanged on `HostExecutionContext`.
- Read: `host.rs:532–654`.
- Edit (≤3): `host.rs`, `marshal/accumulators.rs`, `marshal/mod.rs`.
- Dispatches: `cargo check` (FACT).
- Context cost: **M**.
- Verify: `! rg -n 'type (PerimeterRegionOrigin|SliceRegionOrigin)\b' crates/slicer-wasm-host/src`
- Exit condition: crate compiles; aliases gone; accumulators in `marshal`.

### Step 5 — Move marshal-out converters; rewrite on `OriginBucket`
- Objective: move `convert_infill_output`, `convert_support_output`, `convert_perimeter_output`, `merge_slice_postprocess_into`, `collect_postpass_output` and their private leaf helpers into `marshal/out.rs`; rewrite the three bucketing converters to use `OriginBucket`; repoint `dispatch.rs::deconstruct_layer_ctx` to `marshal::convert_*`.
- Precondition: Step 4 done.
- Postcondition: AC-4 passes (`any_tagged` absent from `host.rs`/`dispatch.rs`); AC-6 contract bucket still `0 failed`.
- Read: `host.rs:4505–5177`; `dispatch.rs:201–272, 2216–2448`.
- Edit (≤3): `host.rs`, `dispatch.rs`, `marshal/out.rs`.
- Dispatches: field-name FACT (`InfillRegion`/etc.); `cargo check`; `cargo test -p slicer-wasm-host --test contract` (FACT pass/fail + first failing assertion).
- Context cost: **M**.
- Verify: `! rg -n 'any_tagged' crates/slicer-wasm-host/src/host.rs crates/slicer-wasm-host/src/dispatch.rs` and contract bucket `0 failed`.
- Cheapest falsifier: any contract test regresses (output change) → behaviour broke.

### Step 6 — Move leaf maps into `marshal/leaf.rs`
- Objective: move the surviving single leaf converters (`*extrusion_role*`, `*expolygon*`, `*paint*`, `*wall*`, `*gcode*`, `*retract*`, `*extrusion_path*`) into `marshal/leaf.rs`; repoint `host.rs` Host impls and `dispatch.rs` callers. Additionally relocate the two divergent **inbound** role converters (`finalization_role_wit_to_ir`, `convert_postpass_role`) into `marshal/leaf.rs` **verbatim** as a clearly-named lossy variant (e.g. `extrusion_role_from_wit_keep_custom`) carrying `// TODO(packet-115): collapse to recovering form; latent PrimeTower/Skirt loss`, and keep the finalization/postpass call sites pointed at it. Do **NOT** unify it with the recovering `extrusion_role_from_wit` — that behaviour change is packet 115.
- Precondition: Step 5 done.
- Postcondition: leaf converters have one home; crate compiles; contract bucket green.
- Read: `host.rs:1667–2400` (IR→WIT leaves), `host.rs:4505–4917` (WIT→IR leaves).
- Edit (≤3): `host.rs`, `dispatch.rs`, `marshal/leaf.rs`.
- Dispatches: `cargo check` (FACT).
- Context cost: **M**.
- Verify: `cargo check --workspace --all-targets` clean.
- Exit condition: compiles; no leaf converter remains defined in `host.rs`/`dispatch.rs`.

### Step 7 — Move IR→WIT projections into `marshal/in_.rs`
- Objective: move `sliced_region_to_data`, `perimeter_region_to_data`, `project_layer_plan_view`, `project_region_segmentation_view`, `project_support_geometry_view`, `object_mesh_to_wit_mesh_object_view`, and the `dispatch.rs` `push_*`/`harvest_*_from` marshal-in helpers into `marshal/in_.rs`; repoint callers.
- Precondition: Step 6 done.
- Postcondition: AC-2 passes (marshal subtree exists, no `wasmtime`); `dispatch.rs` retains only wasmtime mechanics + the stage router.
- Read: `host.rs:2039–2199`; `dispatch.rs:1331–1807`.
- Edit (≤3): `host.rs`, `dispatch.rs`, `marshal/in_.rs`.
- Dispatches: `cargo check`; `! rg wasmtime marshal/` (FACT).
- Context cost: **M**.
- Verify: `test -d crates/slicer-wasm-host/src/marshal && ! rg -n 'wasmtime' crates/slicer-wasm-host/src/marshal/`
- Exit condition: AC-2 passes.

### Step 8 — Packet completion gate
- Objective: full gate green; no regressions.
- Precondition: Steps 1–7 done.
- Postcondition: all ACs pass.
- Edit: none (fixes only if a gate fails, within already-in-scope files).
- Dispatches: `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p slicer-wasm-host --lib marshal` and `--test contract` and `--test unit` (FACT pass/fail each).
- Context cost: **S**.
- Verify: gate subset in `packet.spec.md` all green; AC-1…AC-6, AC-N1, AC-N2 all pass.
- Exit condition: every AC verification command passes.

## Per-Step Budget Roll-Up

S, S, M, M, M, M, M, S → aggregate **M**. No L step. Largest: Steps 3/5.

## Packet Completion Gate

- AC-1…AC-6 and AC-N1/AC-N2 all pass (commands in `packet.spec.md` / `requirements.md`).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `marshal/` contains no `wasmtime` reference; `grep -c 'bindgen!' host.rs` still 4 (ADR-0005 untouched).

## Acceptance Ceremony

Run the gate subset, then the per-AC commands; record each FACT. This packet does **not** require `cargo test --workspace` — its slice is fully covered by the targeted `slicer-wasm-host` buckets plus `cargo check --workspace --all-targets`. If closure policy nonetheless mandates the full suite, delegate it to a sub-agent returning only `FACT pass/fail + first failing test`.
