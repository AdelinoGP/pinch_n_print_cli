# Implementation Plan: brim-ears

Steps are ordered and strictly sequential. **Step 0 is a gate: this packet cannot start until `257a-brim-type-and-object-gap` is implemented.**

Every `cargo` command tees to `target/test-output.log`. When a run fails, **read the log**; never re-run to see more output (`CLAUDE.md` § Test output must always tee). `skirt-brim/src/**` feeds the guest build, so run `cargo xtask build-guests --check` and judge by its exit code before attributing any module-test failure to your changes.

---

## Step 0 — Confirm the dependency landed

- **Task IDs:** none (queue packet, `task_ids: []`).
- **Objective:** establish that `257a`'s contour derivation and mode dispatch exist before any ear work begins.
- **Preconditions:** clean tree.
- **Postconditions:** `skirt-brim` contains a per-object layer-0 contour helper and a `brim_type` mode dispatch with a rejecting `brim_ears` arm; `docs/spec_packets/257a-brim-type-and-object-gap/packet.spec.md` reads `status: implemented`.
- **Allowed reads:** `docs/spec_packets/257a-brim-type-and-object-gap/packet.spec.md` frontmatter; `docs/spec_packets/257a-brim-type-and-object-gap/design.md` § Recorded Divergences **only** (for DIV-1's half-line-width compensation, which this packet inherits as its DIV-2).
- **Edits (0):** none. This step is a gate.
- **Out of bounds:** everything else in `257a`'s directory.
- **Dispatch:** `FACT` ≤ 5 lines — does `skirt-brim/src/lib.rs` contain a per-object contour helper and a `brim_type` match with a `brim_ears` arm, and what is `257a`'s frontmatter `status`?
- **Cost:** S.
- **Authorities:** `packet.spec.md` § Prerequisites and Blockers.
- **Verification:** `grep -q 'status: implemented' docs/spec_packets/257a-brim-type-and-object-gap/packet.spec.md && echo PASS || echo FAIL`
- **Falsifying exit:** `257a` is not implemented. **Stop.** Do not build a private contour derivation here — that duplicates `257a` and the two will drift.

---

## Step 1 — Declare and read both ear keys, and build the decimation helper

- **Task IDs:** none.
- **Objective:** both keys are declared with canonical types, defaults and bounds and read in `from_config`; `decimate_contour(points, tolerance_mm)` exists with canonical's two guards.
- **Preconditions:** Step 0 passed.
- **Postconditions:** AC-4 and AC-N1 pass. `brim_ears_max_angle` is converted to radians once, in `from_config`.
- **Allowed reads:** `from_config` and the existing `[config.schema]` tables `257a` added; `docs/03_wit_and_manifest.md` for the table shape.
- **Edits (3):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/skirt-brim/tests/brim_ears_tdd.rs` (net-new).
- **Out of bounds:** `docs/config/host-keys.toml` — these are module keys, not host keys.
- **Dispatch:** `SUMMARY` ≤ 200 words on canonical `MultiPoint::_douglas_peucker` — the tolerance semantics `brim_ears_detection_length` parameterises.
- **Cost:** M.
- **Authorities:** `requirements.md` § Canonical semantics this packet borrows exactly; `design.md` DIV-1, § Architecture Constraints.
- **Implementation notes, binding:** tolerance `0` returns the input untouched — this is canonical's disable, not "decimate with tolerance zero". A result below four points is discarded in favour of the input. The helper parameter is named `angle_threshold_rad` where it takes an angle; the degrees→radians conversion `(180 - max_angle) * PI / 180` happens once, in `from_config`. Re-derive `SkirtBrim`'s field count first — `257a` added two fields and the struct may now be watched by the churn gate.
- **Verification:** `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd decimation 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`; plus `cargo xtask check-literals` if the struct is watched.
- **Falsifying exit:** the manifest gains a table for `brim_use_efc_outline` or `brim_ears_outer_only`. The first is returned to the queue by `257a`; the second is not a ticket-12 key. Declaring either is the disposition rule 1 prohibits.

---

## Step 2 — Corner detection

- **Task IDs:** none.
- **Objective:** `detect_ear_anchors(contour, angle_threshold_rad, convex)` selects the vertices whose turn exceeds the threshold, convex for the outer band and concave for the inner.
- **Preconditions:** Step 1 complete.
- **Postconditions:** AC-2 passes.
- **Allowed reads:** the decimation helper from Step 1.
- **Edits (2):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/brim_ears_tdd.rs`.
- **Out of bounds:** `257a`'s contour derivation and its four shipped mode arms.
- **Dispatches:** (a) `SUMMARY` ≤ 200 words on canonical `Polygon::convex_points` / `concave_points` — the exact angle convention the threshold is compared against; (b) `FACT` ≤ 5 lines — `POLY_SIDE_COUNT`'s value and `size_ear`'s exact expression, needed by Step 3.
- **Cost:** M.
- **Authorities:** `design.md` § Risks (the angle-sense trap), § Architecture Constraints.
- **Implementation note, binding:** the threshold is a *turn* angle derived as `180 - max_angle`, so a **larger** `brim_ears_max_angle` makes **more** corners qualify. Getting this backwards yields a plausible ear set that is exactly wrong, which no smoke test would catch. AC-2 names two specific corner angles precisely to pin the sense.
- **Verification:** `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd max_angle_selects_which_corners_become_ears 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** lowering `brim_ears_max_angle` increases the ear count. The angle sense is inverted; fix it before proceeding rather than adjusting the test.

---

## Step 3 — Ear emission and the `brim_ears` mode arm

- **Task IDs:** none.
- **Objective:** each anchor emits a regular polygon of the canonical side count at radius `size_ear`, the gap-offset island is subtracted, and `brim_type = brim_ears` dispatches to the generator instead of rejecting.
- **Preconditions:** Steps 1–2 complete.
- **Postconditions:** AC-1, AC-3, AC-5 pass; AC-N3 and AC-N5 pass.
- **Allowed reads:** `257a`'s band generator and its gap-offset island computation (ranged); the two helpers from Steps 1–2.
- **Edits (2):** `modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/skirt-brim/tests/brim_ears_tdd.rs`.
- **Out of bounds:** `generate_skirt_entities`; `257a`'s four shipped mode arms; `crates/slicer-gcode/src/serialize.rs`.
- **Dispatch:** `SUMMARY` ≤ 200 words on canonical `outer_inner_brim_area` — how `size_ear` is computed and how the island is subtracted to leave the annulus.
- **Cost:** M.
- **Authorities:** `design.md` § Selected Approach, DIV-3, § Invariants.
- **Implementation notes, binding:** `size_ear` is `brim_width - brim_object_gap - line_width` in this tree's terms — confirm the spacing term maps onto `line_width` via the Step-2 dispatch before relying on it. The subtraction uses `slicer_sdk::host::clip_polygons(ears, &island, ClipOperation::Difference)`; the island is the one `257a` already computes, not a new one. `painted` stays rejected — do not add an arm for it. `brim_ears_outer_only` is fixed at canonical's default (DIV-3) and is not declared.
- **Verification:** `cargo xtask build-guests --check` (judge by exit code), then `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd 2>&1 | tee target/test-output.log && cargo test -p skirt-brim --test brim_type_tdd 2>&1 | tee -a target/test-output.log; grep -c "test result: ok" target/test-output.log`
- **Falsifying exit:** any of `257a`'s four mode arms changes output (AC-N5), or an emitted ear point lies inside the gap-offset island (AC-5). The second means the island subtraction was skipped — ears would print on top of the part.

---

## Step 4 — Bounds arm, gates, and docs regeneration

- **Task IDs:** none.
- **Objective:** both new keys are bounds-enforced, all gates are green, and the generated key reference reflects two newly live keys.
- **Preconditions:** Steps 0–3 complete.
- **Postconditions:** AC-N2, AC-N4 pass; the gate commands are green; `docs/15_config_keys_reference.md` regenerated.
- **Allowed reads:** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (existing arms, for shape — including the arm `257a` added).
- **Edits (2):** `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `docs/15_config_keys_reference.md` (regenerated, not hand-edited).
- **Out of bounds:** `crates/slicer-gcode/src/serialize.rs`; `docs/specs/orca-feature-gap/**`.
- **Dispatch:** `FACT` pass/fail per gate command.
- **Cost:** S.
- **Authorities:** `packet.spec.md` § Gate Commands; `requirements.md` § Verification Matrix.
- **Verification:** `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check` (exit 0 fresh, 1 stale, 3 `wasm-tools` missing; never grep for `STALE:`); then `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **Falsifying exit:** `git diff --stat -- crates/slicer-gcode/src/serialize.rs` is non-empty (AC-N4).

---

## Aggregate Context Cost

S + M + M + M + S = **M aggregate, no step above M.** No further split required.

## Closing Obligations

- Report the five items in `design.md` § Map and Ticket Updates Required. Items 1–3 duplicate `257a`'s list deliberately; if `257a` closed first and already reported them, say so rather than filing twice. Do **not** apply them; the map and tickets are out of bounds.
- Re-derive every ledger fact at point of use: `SkirtBrim`'s field count after `257a`, `POLY_SIDE_COUNT`, whether the crate still uses test auto-discovery, and the next free packet number for the `brim_ears_outer_only` gap.
- Run `/spec-review 257b-brim-ears --preflight` before requesting activation, and re-check the two map gates by hand: (a) zero declaration-only keys; (b) a non-default-value AC for every in-packet key.
