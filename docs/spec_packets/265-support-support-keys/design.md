# Design: support-support-keys

## Selected Approach

Five decision points, all built inside owners that already exist. No new module, no new claim, no new seam.

**Two host-side filters in `slicer-core`.** `support_critical_regions_only` and `support_remove_small_overhang` belong in `slicer_core::algos::overhang_annotation`, because that is where the port already computes both of the sets canonical's critical-regions branch keeps: `detect_support_contacts_with_annotations` returns a `SupportContactAnnotations` carrying `cantilever_surfaces`, and the same function already has the sharp-tail branch gated on `SupportContactParams::support_sharp_tails`. Canonical's `TreeSupport::detect_overhangs` clears ordinary overhangs and re-appends cantilevers, with sharp tails appended later and enforcers after that. The port reproduces that exact ordering with the pieces it already has: filter small clusters, restrict to critical regions, and let `support_analysis_producer` union the enforcers afterwards — which it already does, in that position.

**One line of sourcing in `slicer-runtime`.** `enforce_support_layers` needs no new logic at all. `detect_support_contacts`'s `force_support` branch already reads `params.layer_id < params.enforce_support_layers`, and the producer's per-layer `SupportContactParams` literal already sets `layer_id` from the real layer index through a functional-update over `base_params`. The only defect is that `resolve_contact_params` hardcodes `enforce_support_layers: 0` behind a comment asserting the knob has "no production config source yet" — which stopped being true when the host key was declared. Sourcing it from `ResolvedConfig` makes the key live.

**Two geometry overrides in the planners.** `support_object_first_layer_gap` is a *substitution*, not an addition: canonical `TreeSupport::draw_circles` uses it in place of `m_xy_distance` when the object layer index is 0. Both planners already have exactly one clearance quantity each (`traditional-support-planner`'s trim offset, `tree-support-planner`'s `inflate_model_occupancy` argument at its two call sites), so the change is a selection on the layer index at those sites.

`support_bottom_z_distance` is the mirror of a computation that already exists. `traditional-support-planner` computes `target_top_z = overhang_plane_z - self.support_top_z_distance` and then walks actual layer Z downward until it finds the highest layer at or below that plane. The bottom gap is the same walk in the other direction, applied only when the column terminates on a *model* surface — `model_termination_layer` is `Some`. Canonical's `gap_object_support` is likewise an object-support gap; a column standing on the build plate has nothing beneath it to gap against, and the port already distinguishes those two cases deliberately (`None` collapsing to layer 0 carries the comment explaining that the plate carries no bottom interface).

## Rule 4 Trigger Test

Authoring rule 4 routes an Orca enum whose values are different algorithms to `claim:*` holders. The map's own trigger test says the rule fires on **cross-module** algorithm selection and explicitly does not fire on a module branching internally over a mode it implements itself, naming `support_style` among the latter. This packet's keys are three booleans, two floats and an integer — no algorithm selection at all — plus `support_type`, which already rides the existing `support-family:` claim seam described in `docs/04_host_scheduler.md` § Claim Resolution. Rule 4 does not fire here. `support_style` is returned to the queue for a different reason (see `requirements.md`), not refactored.

## Claims Held

- No new claim is introduced.
- `support_type` continues to resolve through `slicer_scheduler::execution_plan::select_support_family` and the `support-family:` claim prefix matched by `module_claims_match_active_region`. `traditional-support-planner` and `tree-support-planner` keep the claims they hold today; this packet changes what they read, never what they claim.

## Which Existing Mechanism Carries the New Data

| New behaviour | Carrier | Why not something else |
| --- | --- | --- |
| `enforce_support_layers`, `critical_regions_only`, `remove_small_overhang` reaching the detector | `SupportContactParams`, the existing host-side params struct built by `resolve_contact_params` and specialised per layer via functional update | These are host-algo inputs, not guest config; the struct is the established carrier and already holds four sibling knobs |
| `support_bottom_z_distance`, `support_object_first_layer_gap` reaching the planners | module manifests (`[config.schema.*]`) plus `ConfigView::get` in each planner's `from_config`, the same path `support_top_z_distance` and `support_object_xy_distance` already take | Host-side special-casing would violate rule 4's "new decision points go where the architecture puts them"; both keys are per-object geometry settings, which is what a module manifest key is for |
| Per-region variation of `support_threshold_angle` / `support_type` | `slicer_core::algos::region_mapping` overlay, already live | Nothing to add |

No `SliceRegionView` metadata, no prepass IR field, no `PostPass` claim, and no SDK surface change is required.

## Recorded Divergences

- **DIV-1 — zero bottom gap does not fall back to the top gap.** Canonical's `support_bottom_z_distance == 0 ? support_top_z_distance` rule lives only in `GCode::collect_layers_to_print`, an air-gap sanity check at G-code assembly time. In `Slicing.cpp` a zero value instead sets `gap_object_support = 0` through the zero-gap-interface path. The port has no equivalent of the G-code-side check, so importing the fallback at the geometry seam would invent a coupling canonical does not have there — and would make `0` unable to express "no gap", which is the one thing a user setting it to `0` means. The port treats `0` as zero gap, matching `Slicing.cpp`. Rationale recorded in both planner manifests' `description` fields.
- **DIV-2 — the bottom gap applies to model-terminated columns only.** Canonical's `gap_object_support` is an object-support gap by name and by construction. The port makes the distinction explicit and testable because it already tracks `model_termination_layer` as an `Option`, where canonical infers it. AC-4 asserts both halves, so the divergence cannot silently become "gap everything".
- **DIV-3 — the port improves on canonical's cluster exemption ordering.** Canonical exempts a cluster from small-overhang removal when it overlaps a sharp tail or cantilever, but the two features are computed in different passes and the exemption depends on pass ordering. The port computes the sharp-tail and cantilever sets in the same function that runs the filter, so the exemption is a pure predicate over data already in hand rather than an ordering constraint. Behaviourally identical; structurally not fragile.

## Tier Derivation

Ticket 04's rubric: **Tier A** is plumbing into a decision point that already exists; **Tier B** is new logic in an existing owner; **Tier C** is a new module at a new seam.

`enforce_support_layers` alone would be Tier A — the decision point exists and only the sourcing is missing. The other four build decision points that do not exist, all of them inside owners that do: two filter stages in `slicer-core`'s existing detector, and two geometry overrides in two existing planner modules. No module is created and no seam is added, so Tier C does not apply. The packet is **Tier B** (was Tier A). The map's tier table needs the correction; it is listed in § Map and Ticket Updates Required and is not applied from here.

## Code Change Surface

**Editable:**

- `crates/slicer-core/src/algos/overhang_annotation.rs` — two new `SupportContactParams` fields, the `Default` impl, the two new filter stages inside `detect_support_contacts_with_annotations`, and the two in-file struct literals.
- `crates/slicer-core/Cargo.toml` — one net-new `[[test]]` entry with `required-features = ["host-algos"]`.
- `crates/slicer-core/tests/support_critical_and_small_overhang_tdd.rs` — net-new (AC-2, AC-3).
- `crates/slicer-core/tests/support_overhang_detection_tdd.rs` — the eleven existing struct literals, plus the non-default cases for AC-7 and AC-8 if the current cases assert only at defaults.
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — `resolve_contact_params`'s three newly sourced fields, its stale comment, its three struct literals, and new cases in its `tests` module (AC-1, AC-9).
- `modules/core-modules/traditional-support-planner/src/lib.rs` — two new struct fields plus their `from_config` reads; the trim-offset selection; the emit-floor walk.
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` — two net-new `[config.schema.*]` tables.
- `modules/core-modules/traditional-support-planner/Cargo.toml` — one net-new `[[test]]` entry.
- `modules/core-modules/traditional-support-planner/tests/support_gap_keys_tdd.rs` — net-new (AC-4, AC-5).
- `modules/core-modules/tree-support-planner/src/lib.rs` — two new struct fields plus their `from_config` reads; the two `inflate_model_occupancy` call sites; the descent-termination floor.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` — two net-new `[config.schema.*]` tables.
- `modules/core-modules/tree-support-planner/Cargo.toml` — one net-new `[[test]]` entry.
- `modules/core-modules/tree-support-planner/tests/support_gap_keys_tdd.rs` — net-new (AC-6).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — one bounds arm (AC-N1).
- `docs/15_config_keys_reference.md` — regenerated.

**Read-only context:** `docs/config/host-keys.toml` (confirm the five keys' declared ranges; do not edit — all five are already declared), `crates/slicer-ir/src/resolved_config.rs` (confirm the typed fields exist; do not edit), `crates/slicer-scheduler/src/execution_plan.rs` (`select_support_family`, for AC-11 only).

**Out of bounds — must not be loaded or edited:**

- `crates/slicer-gcode/src/serialize.rs` (AC-N3 asserts it is untouched).
- `modules/core-modules/traditional-support/**` and `modules/core-modules/tree-support/**` — the renderers. This packet is planner-side and host-side only; `support_style` lives there and is returned to the queue.
- Every other packet directory under `docs/spec_packets/`, in particular `240-support-raft/`.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**`.
- `crates/slicer-schema/wit/**` — no WIT change is in scope; touching it means the packet was mis-scoped, stop and report.

## Blast-Radius Discipline

`SupportContactParams` is a `pub` struct with more than five named fields under `crates/*/src`, so it is a **watched type** under the struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`). Adding two fields breaks every exhaustive literal. Re-derived from disk at authoring time, the literal sites are: **2** in `crates/slicer-core/src/algos/overhang_annotation.rs`, **11** in `crates/slicer-core/tests/support_overhang_detection_tdd.rs`, and **3** in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`. Re-derive that count at point of use — it is a ledger fact and other packets touch these files.

Rules for the step that adds the fields:

- Production `src/` literals stay exhaustive; test literals must use a `..` rest or carry an `// exhaustive: <reason>` waiver.
- The producer's per-layer literal already uses `..base_params.clone()` and needs no change.
- The step's exit condition is `cargo check --workspace --all-targets` **plus** `cargo xtask check-literals`, both green, in the same step. Discovering these sites via a later step's failing check is a defect.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Do not divide by `effective_layer_height`.** Register rows G-09 and RC-11 record that the field disagrees across transports (a max on one leg, a first-match on the other) and that dividing by it previously produced a zero offset in the traditional planner and tens of layers in the tree planner. The bottom-Z gap walks actual layer Z, exactly as the live `target_top_z` computation does.
- **`slicer-core` offsets are millimetres at this seam.** `slicer_core::polygon_ops::offset` scales internally, so canonical's `scale_()` calls are deliberately not ported — the existing `SupportContactParams` doc comment states this and the new filter must honour it. The small-overhang erosion is therefore `-external_perimeter_width_mm`, not a scaled value.
- **Canonical support offsets use a square join.** `SUPPORT_SURFACES_JOIN` is `OffsetJoinType::Square` for every offset in the contact pipeline; the new erosion uses it.
- **The `host-algos` feature gate.** `slicer-core`'s support test targets carry `required-features = ["host-algos"]`. The net-new test target must be registered the same way, and every narrow command must pass `--features host-algos`. A bare `cargo test -p slicer-core` compiles zero of these targets and prints a clean `ok`.

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| Confirm the exact erode-and-measure smallness expression and the sharp-tail/cantilever exemption | `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` `top_contact_layers` | `SUMMARY` ≤ 200 words |
| Confirm the clear-and-re-append ordering in the critical-regions branch | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` `detect_overhangs` | `SUMMARY` ≤ 200 words |
| Confirm `gap_xy_first_layer` is a substitution for `m_xy_distance`, not an addition | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` `draw_circles` | `SUMMARY` ≤ 200 words |
| Re-derive the `SupportContactParams` literal sites before editing | `crates/**` and `modules/**`, Rust only | `LOCATIONS` ≤ 20 entries |
| Run each verification command | as listed in `requirements.md` § Verification Matrix | `FACT` pass/fail |

## Invariants

- The default path is unchanged: `critical_regions_only = false` and `remove_small_overhang = true` are the canonical defaults, and `remove_small_overhang`'s default is `true`, so the filter runs by default. AC-N4 guards the aggregate; if it fails, the filter is over-aggressive and the erosion or the `2 * fw` comparison is wrong.
- Enforcers are unioned **after** both new stages, never filtered by them. Canonical adds enforcers after the clear; a user-painted enforcer must survive `critical_regions_only`.
- The bottom gap never raises a column's floor above its own emit ceiling; a gap larger than the column's height yields an empty column, not an inverted range.
- `support_object_first_layer_gap` replaces `support_object_xy_distance` on layer 0; the two are never summed.

## Risks

- **The small-overhang filter is the one stage that can silently remove real support.** Its default is `true`, so it is on by default and a wrong threshold degrades every default-path slice. AC-N4 is the tripwire; the erosion must be one extrusion width and the comparison `< 2 * fw` on *either* axis, per the canonical evidence in `requirements.md`.
- **`tree-support-planner/src/lib.rs` is long.** Ranged reads only, anchored on `inflate_model_occupancy` and the descent-termination site. A full-file read will blow the step's budget.
- **Same-manifest churn with packet `240-support-raft`** on `tree-support-planner.toml`. Both packets append `[config.schema.*]` tables; whichever lands second rebases its append. No semantic conflict.

## Open Questions

- `[FWD]` The tree planner's descent-termination site is the analogue of the traditional planner's `model_termination_layer`, but the tree family reaches the plate through node propagation rather than a single termination index. The implementer must confirm at Step 6 whether the tree planner exposes a per-column model-termination signal; if it does not, AC-4 is satisfied by the traditional planner alone and the tree-side bottom gap is deferred with a named reason rather than faked. This is a forward question, not a blocker: it cannot change the packet's shape, only whether one of two planners carries one of five behaviours.

**No `[BLOCK]` is open.** The packet needs no new WIT interface, no IR schema bump, and no new host `ResolvedConfig` field — all five host keys are already declared in `docs/config/host-keys.toml` and already carried as typed `ResolvedConfig` fields, verified against the tree at authoring time.

## Map and Ticket Updates Required

Listed only; **not applied by this packet** (the map and tickets are out of bounds).

1. **Tier correction.** The map's P13 entry and ticket 04's tier table carry this packet as Tier A. It is **Tier B**.
2. **Coverage-count correction.** P13 covers **10** keys, not 12. `raft_first_layer_expansion` moves to packet `240-support-raft`'s count; `support_style` moves to the organic-tree-engine row (sibling-plan row 7 / TASK-441), which still needs a packet number derived from disk.
3. **Register row G-05 closes here.** `docs/specs/support-parity-gap-register.md` routes `support_bottom_z_distance` to `238a-support-pattern-config-keys`. That packet is `implemented` and its `design.md` records a "Bottom-z scope split" deferring the consuming semantics to 238b/238c — both of which are also `implemented` and mention the key nowhere. The row's destination must be re-pointed at this packet.
4. **Manifest inconsistency to report, not fix.** `traditional-support.toml` declares `support_style` as `type = "string"` while `tree-support-planner.toml` declares the canonical 7-value `enum`. The correction belongs with the organic-tree-engine work.
5. **A gap this packet does not own.** `bridge_no_support`, `bridge_polygons` and `support_sharp_tails` are still hardcoded neutral in `resolve_contact_params` with the same "no production config source yet" comment that `enforce_support_layers` carried. They are not ticket-20 keys and need a queue home.
