# Implementation Plan: brim-type-and-object-gap

Steps are ordered and strictly sequential: Step 1 creates the contour every later step consumes.

Every `cargo` command tees to `target/test-output.log`. When a run fails, **read the log**; never re-run to see more output (`CLAUDE.md` § Test output must always tee). `skirt-brim/src/**` feeds the guest build, so run `cargo xtask build-guests --check` and judge by its exit code before attributing any module-test failure to your changes.

---

## Step 1 — Derive per-object layer-0 contours from outer-wall entities

- **Task IDs:** none (queue packet, `task_ids: []`).
- **Objective:** a private helper in `skirt-brim` returns, for the first layer, a map from `object_id` to `Vec<ExPolygon>` built by unioning that object's closed `ExtrusionRole::OuterWall` loops. No brim behaviour changes yet — the existing bbox path still runs.
- **Preconditions:** clean tree; `cargo check --workspace --all-targets` green.
- **Postconditions:** the helper exists and is exercised by a test asserting that a square object with a square hole yields one `ExPolygon` with one contour and one hole; objects with no outer-wall loop yield an empty entry plus a diagnostic (DIV-2).
- **Allowed reads:** `modules/core-modules/skirt-brim/src/lib.rs` in full (it is short); ranged reads of `slicer_sdk::host`'s `clip_polygons` signature and `ClipOperation` variants in `crates/slicer-sdk/src/host.rs`; the `ExtrusionRole` variant list and `RegionKey` shape in `crates/slicer-ir/src/slice_ir.rs`, ranged.
- **Edits (2):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/brim_type_tdd.rs` (net-new).
- **Out of bounds:** `crates/slicer-schema/wit/**`; `crates/slicer-gcode/src/serialize.rs`; every other module.
- **Dispatch:** `LOCATIONS` ≤ 20 — confirm the crate declares no `[[test]]` entries (auto-discovery), and re-derive `SkirtBrim`'s field count for the churn gate.
- **Cost:** M.
- **Authorities:** `design.md` § Selected Approach, DIV-1, DIV-2, § Architecture Constraints.
- **Implementation notes, binding:** close each loop before unioning — `make_rect_loop` repeats its first point but perimeter-module entities may not, and `clip_polygons` on an open polyline returns degenerate results. Use `ClipOperation::Union` with an empty clip set. The role to filter on is `ExtrusionRole::OuterWall` (**not** `ExternalPerimeter`, which is not a variant in this tree). Offsets are in millimetres.
- **Verification:** `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd contour_derivation 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** the derivation returns nothing for a fixture that plainly has outer walls. That means `skirt-brim` runs before outer walls exist in the entity stream — the packet's core premise is wrong. **Stop and report**; do not work around it by falling back to the bounding box.

---

## Step 2 — Declare and read `brim_type`

- **Task IDs:** none.
- **Objective:** `brim_type` is declared in the manifest as a seven-value enum in the canonical order with default `auto_brim`, held as a `SkirtBrim` field, and read in `from_config` with unknown values rejected by name.
- **Preconditions:** Step 1 complete.
- **Postconditions:** AC-N1 passes. The value is resolved per object where the finalization stage permits it; if it does not, the `[FWD]` question in `design.md` § Open Questions is answered in writing there and the key degrades to global.
- **Allowed reads:** `modules/core-modules/skirt-brim/skirt-brim.toml` (the existing `brim_width` table, for shape); the `from_config` body.
- **Edits (3):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/skirt-brim/tests/brim_type_tdd.rs`.
- **Out of bounds:** `docs/config/host-keys.toml` — `brim_type` is a module key, not a host key; declaring it there is wrong.
- **Dispatch:** `FACT` ≤ 5 lines — the `BrimType` declared value order and default from canonical `PrintConfig.hpp` / `PrintConfig.cpp`.
- **Cost:** S.
- **Authorities:** `requirements.md` § Canonical semantics this packet borrows exactly; `docs/03_wit_and_manifest.md` for the enum table shape.
- **Implementation note:** declare all seven canonical values so the manifest is honest about the enum's shape, but `brim_ears` and `painted` are **unshipped values** — `from_config` must reject them with a message naming the packet that will ship `brim_ears` (`257b`) and the missing carrier for `painted`. Rejecting is the rule-4 ruling applied consistently: silently treating an unimplemented value as the default is the "silently hollow part" failure that ruling forbids.
- **Verification:** `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd unknown_brim_type_is_rejected 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** the manifest gains a table for `brim_use_efc_outline`, `brim_ears_max_angle` or `brim_ears_detection_length` (AC-N3). Those are returned or deferred; declaring them is the disposition rule 1 prohibits.

---

## Step 3 — Replace the bbox brim with per-object contour bands

- **Task IDs:** none.
- **Objective:** `generate_brim_entities` is replaced by a per-object, per-band generator driven by `brim_type`; each emitted entity carries the owning object's `object_id` in its `RegionKey`.
- **Preconditions:** Steps 1–2 complete.
- **Postconditions:** AC-1, AC-2, AC-3, AC-4, AC-6 pass; AC-N5 still passes.
- **Allowed reads:** `generate_brim_entities`, `make_rect_loop`, and the `run_finalization` brim block in `modules/core-modules/skirt-brim/src/lib.rs`.
- **Edits (2):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/brim_type_tdd.rs`.
- **Out of bounds:** `generate_skirt_entities` — the skirt keeps the global bounding box (AC-N5).
- **Dispatch:** (a) `SUMMARY` ≤ 200 words on canonical `outer_inner_brim_area` — the `has_outer_brim` / `has_inner_brim` derivation per value; (b) `LOCATIONS` ≤ 20 — every assertion in the tree that expects `object_id == "brim"`, which this step must update.
- **Cost:** M.
- **Authorities:** `design.md` DIV-1 (the `line_width / 2` centreline compensation), DIV-4 (`auto_brim` maps to the outer band), § Invariants.
- **Implementation notes, binding:** loop spacing stays exactly one `line_width`. `make_rect_loop` is retained for the skirt and must not be deleted. The outer band walks outward from the compensated contour; the inner band offsets each hole by a **negative** distance and walks inward — assert containment (AC-2) rather than trusting the sign.
- **Verification:** `cargo xtask build-guests --check` (judge by exit code), then `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd 2>&1 | tee target/test-output.log && cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee -a target/test-output.log; grep -c "test result: ok" target/test-output.log`
- **Falsifying exit:** any skirt entity changes. The skirt path was touched; revert that part.

---

## Step 4 — Declare, read and apply `brim_object_gap`

- **Task IDs:** none.
- **Objective:** `brim_object_gap` is declared (float, default `0.0`, `[0.0, 2.0]`), read, and applied as the stand-off between the object contour and the innermost loop of each band.
- **Preconditions:** Step 3 complete.
- **Postconditions:** AC-5 passes; AC-1 through AC-4 and AC-6 still pass.
- **Allowed reads:** the band generator from Step 3.
- **Edits (3):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/skirt-brim/tests/brim_object_gap_tdd.rs` (net-new).
- **Out of bounds:** everything outside `modules/core-modules/skirt-brim/`.
- **Dispatch:** `SUMMARY` ≤ 200 words — how canonical `outer_inner_brim_area` applies `brim_offset` to the contour and to the reversed holes.
- **Cost:** S.
- **Authorities:** `requirements.md` § Canonical semantics this packet borrows exactly; `design.md` DIV-1.
- **Implementation note:** the gap shifts where the band starts; it does not change loop spacing and does not change `brim_width`. A gap that consumes the whole width yields zero loops for that object, not a negative count.
- **Verification:** `mkdir -p target && cargo test -p skirt-brim --test brim_object_gap_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** loop spacing changes with the gap. The gap was folded into the spacing rather than the start offset.

---

## Step 5 — Bounds arm, gates, and docs regeneration

- **Task IDs:** none.
- **Objective:** both new manifest keys are bounds-enforced, all gates are green, and the generated key reference reflects two newly live keys.
- **Preconditions:** Steps 1–4 complete.
- **Postconditions:** AC-N2, AC-N3, AC-N4 pass; the gate commands are green; `docs/15_config_keys_reference.md` regenerated.
- **Allowed reads:** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (existing arms, for shape).
- **Edits (2):** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `docs/15_config_keys_reference.md` (regenerated, not hand-edited).
- **Out of bounds:** `crates/slicer-gcode/src/serialize.rs`; `docs/specs/orca-feature-gap/**`.
- **Dispatch:** `FACT` pass/fail per gate command.
- **Cost:** S.
- **Authorities:** `packet.spec.md` § Gate Commands; `requirements.md` § Verification Matrix.
- **Verification:** `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check` (exit 0 fresh, 1 stale, 3 `wasm-tools` missing; never grep for `STALE:`); then `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** `git diff --stat -- crates/slicer-gcode/src/serialize.rs` is non-empty (AC-N4), or a returned/deferred key appears in the manifest (AC-N3).

---

## Aggregate Context Cost

M + S + M + S + S = **M aggregate, no step above M.** No split required beyond the `257a` / `257b` split already taken.

## Closing Obligations

- Report the seven items in `design.md` § Map and Ticket Updates Required. Do **not** apply them; the map and tickets are out of bounds.
- If the `[FWD]` per-object-config question resolved to "global only", write the answer into `design.md` § Open Questions, drop AC-4 with the reason, and report the gap. Do not fake per-object behaviour.
- Re-derive every ledger fact at point of use: `SkirtBrim`'s field count, the `object_id == "brim"` assertion sites, whether the crate still uses test auto-discovery, and the next free packet number for the EFC and paint-carrier gaps.
- Run `/spec-review 257a-brim-type-and-object-gap --preflight` before requesting activation, and re-check the two map gates by hand: (a) zero declaration-only keys; (b) a non-default-value AC for every in-packet key.
