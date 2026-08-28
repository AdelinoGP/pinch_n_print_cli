---
status: draft
packet: 251-visual-debug-silhouette-seam-overlays
task_ids:
  - TASK-455
  - TASK-456
  - TASK-457
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #5)
---

# Packet Contract: 251-visual-debug-silhouette-seam-overlays

## Goal

Bring seam glyphs to silhouettes (plan D18): the existing isolated form `overlays: ["seams"]` becomes legal on model-source silhouette specs with its exact 1.1.0 meaning (a `FAINT_BASE`-gray silhouette base plus the legend-1.1.0 red filled-circle seam glyphs at projected `(x_or_y, z)`, every rendered seam mirrored into the entry's `overlay_events`), the seam event shape gains an additive optional `z` field that is absent from all 1.0/1.1 serialization, a new 1.2.0-only option `composited_overlays: ["seams"]` draws the same glyphs onto the colored silhouette base image, and the full R9 validation matrix fails closed with named errors — retiring packet 247's `deny_unknown_fields` interim pin and retargeting packet 248's gcode-overlay pin.

## Scope Boundaries

Model source only; seams are `SeamPlanIR`-sourced from the blackboard (`SeamPlanEntry.chosen_candidate.point`, mm — fact 11), never re-derived. Surface: `OverlayEvent::Seam` (`crates/slicer-runtime/src/visual_debug_style.rs`) and its match sites, a seam-glyph silhouette entry point beside the packet 247/249 composite (`crates/slicer-runtime/src/visual_debug_render.rs`), and options/validation/assembly/manifest in `crates/pnp-cli/src/visual_debug.rs`. Travel/retraction/z-hop/tool-change glyphs on silhouettes stay excluded (plan §8); the gcode source stays seam-free; top-down overlay behavior and every 1.0/1.1 byte stay frozen. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 247 only (`247-visual-debug-silhouette-core`, currently `draft` — FORWARD-DEP; every step consuming its exports states "packet 247 implemented" as a precondition; the swarm executes queue order). Steps touching pins authored by 248/249 carry those packets as per-step FORWARD-DEP preconditions with vacuity annotations.
- Unblocks: nothing (last queue row).
- Activation blockers: packet 247 not yet `implemented`.

## Acceptance Criteria

- **AC-1. Given** a `schema_version: "1.2.0"` model-source request over `resources/regression_wedge.stl` with tap `Layer::Slice` and one silhouette spec `{"type": "silhouette", "options": {"overlays": ["seams"]}}`, **when** `run_visual_debug` succeeds, **then** the bundle contains exactly one isolated-overlay PNG named `images/Layer__Slice_silhouette_front_overlay_seams.png` whose non-background base pixels are all `FAINT_BASE` (`[210, 210, 210]`) or the seam color `[220, 0, 0]`, and its manifest entry has `"visualization": "silhouette"`, `"overlay": "seams"`, `"view": "front"`, a `"layers_rendered"` list, **no** `"layer_index"`/`"layer_z"` key, and an `"overlay_events"` array in which every element is `{"event": "seam", "x": <num>, "y": <num>, "z": <num>}` with `z` present. | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- isolated_seam_overlay_faint_base_and_events_carry_z 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a `SeamPlanIR` fixture with seams at `region_key.global_layer_index` 0, 1, and 2 and a schedule/selection covering only layer 1, **when** the silhouette seam path renders, **then** exactly the layer-1 seam draws a glyph and exactly that seam appears in the returned events (selection filters by `region_key.global_layer_index` ∈ rendered layers; out-of-band seams contribute neither pixels nor events), and each event's `x`/`z` equal the source `chosen_candidate.point`'s per-view horizontal coordinate and `z`. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- seam_glyphs_filter_by_rendered_layers_and_carry_source_coords 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the same wedge request with `{"options": {"composited_overlays": ["seams"]}}`, **when** the bundle renders, **then** exactly one silhouette PNG exists at the **base** filename `images/Layer__Slice_silhouette_front.png` (no extra file), seam-colored `[220, 0, 0]` glyph pixels are present on the colored base, and the entry carries `"composited_overlays": ["seams"]` plus the same z-carrying `"overlay_events"` mirror. | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- composited_seams_draw_on_colored_base_no_extra_file 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a spec carrying **both** `overlays: ["seams"]` and `composited_overlays: ["seams"]` (the plan's §5 example shape), **when** the bundle renders, **then** both images exist per (tap, view) — the isolated `_overlay_seams` image and the composited base — with no filename collision, and each entry mirrors its own events. | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- both_forms_coexist_one_isolated_one_composited 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a declared `schema_version: "1.1.0"` model-source request whose `diagnostic_overlay` spec has `overlays: ["seams"]` over a seam-carrying tap, **when** the bundle renders after this packet's `OverlayEvent::Seam` change, **then** every serialized seam event is byte-identical to the pre-change shape — the JSON object contains exactly the keys `event`, `x`, `y` and **no** `z` key (the new field is `Option` + `skip_serializing_if`, and every top-down construction site passes `None`; pinned on serialization output, not parsing). | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- legacy_seam_events_serialize_without_z_key 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the same captures, seam plan, view, scale, viewport, and schedule, **when** the isolated seam-overlay render runs twice, **then** the two PNG byte vectors are identical and the two event lists are equal element-for-element (events in `SeamPlanIR.entries` source order, filtered); **and** the packet 247/249 composite entry points, called without seams, remain byte-equivalent to their pre-251 output (their own suites pass unchanged). | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- seam_overlay_render_is_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a tool-colored composited request (`color_by: "tool"` + `composited_overlays: ["seams"]`) over a tool-carrying silhouette tap, **when** the bundle renders, **then** the glyphs draw in the same fixed seam color `[220, 0, 0]` on the `_tool` base image (glyph shape/color identical across role and tool palettes — visibility is a documented caveat, not a per-palette restyle). | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- composited_seams_on_tool_colored_base 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8 (docs).** `docs/19_visual_debug.md` documents `composited_overlays` (the literal key, absent from the doc today, so the grep fails until written): seams-only membership, model-source-only, isolated-vs-composited semantics, the z-carrying event mirror, and the glyph-over-palette visibility caveat. | `rg -q 'composited_overlays' docs/19_visual_debug.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a 1.2.0 model-source silhouette spec with (a) `overlays: ["travel"]` and (b) `composited_overlays: ["travel"]`, **when** validated, **then** both are rejected with `ValidationError::InvalidOverlays` whose message names `seams` as the only silhouette overlay kind (travel/retraction/z-hop/tool-change glyphs need their own Z story — plan §8), and no bundle is written. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_overlays_reject_non_seam_kinds 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a 1.2.0 `filled_areas` spec carrying `composited_overlays: ["seams"]`, **when** validated, **then** it is rejected with `ValidationError::InvalidOverlays` naming the silhouette-only rule (a named R9 error, not a `deny_unknown_fields` parse failure). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- composited_overlays_rejected_on_non_silhouette_kind 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** a 1.2.0 **gcode-source** silhouette spec with (a) `overlays: ["seams"]` and (b) `composited_overlays: ["seams"]`, **when** validated, **then** both are rejected with `ValidationError::OverlayUnsupportedOnGcode` naming `seams` (R10's variant — seams are model-source-only, fact 11), and a gcode `diagnostic_overlay` with `overlays: ["seams"]` is still rejected with the same variant (unchanged arm). This retargets packet 248's `gcode_silhouette_overlay_rejections_unchanged`, whose arm (b) pinned the pre-251 blanket `InvalidOverlays`; the old test name is absent and the replacement covers all three arms. The absence clause is vacuously true until packet 248 is implemented — a queue-order artifact. | `! rg -q 'gcode_silhouette_overlay_rejections_unchanged' crates/ && cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_seam_overlay_forms_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** a spec carrying `composited_overlays` under declared `schema_version: "1.1.0"` (and separately `"1.0.0"`), **when** validated, **then** both are rejected with `ValidationError::InvalidOverlays` whose message names `"1.2.0"` as the required schema (the `OptionRequiresSchema11` pattern — never a silent stray-key tolerance under 1.0.0). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- composited_overlays_require_schema_1_2 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N5. Given** a 1.2.0 silhouette spec with `composited_overlays: []` (empty list), **when** validated, **then** it is rejected with `ValidationError::InvalidOverlays` (mirrors the existing empty-`overlays` rule). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- composited_overlays_empty_list_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N6. Given** a 1.2.0 request with two silhouette specs that resolve to the same (tap, view, color mode) group but disagree on `composited_overlays` (one present, one absent), **when** validated, **then** it is rejected with `ValidationError::InvalidOverlays` stating the conflicting-group rule (two specs may not demand different content for one base filename — protects packet 247's filename-uniqueness invariant). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- conflicting_overlay_options_in_one_group_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N7. Given** the tree after this packet, **when** searched, **then** packet 247's interim pin `composited_overlays_not_accepted_by_247` is absent from `crates/` (its `deny_unknown_fields` rejection is replaced by the named R9 matrix above — AC-N2/N4/N5 are the replacement contract), while an unknown option key (e.g. `options.compositedoverlays`) is still rejected via `ValidationError::InvalidVisualizationOptions` (`deny_unknown_fields` loosens for exactly one new field). The absence clause is vacuously true until packet 247 is implemented — a queue-order artifact. | `! rg -q 'composited_overlays_not_accepted_by_247' crates/ && cargo test -p pnp-cli --test visual_debug_validation_tdd -- unknown_option_keys_still_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N8. Given** a seams-overlay silhouette request whose prepass committed **no** `SeamPlanIR` (the blackboard's `seam_plan()` is `None`), **when** the bundle assembles, **then** it fails closed with an error whose Display names the missing seam plan (never a silently glyph-less image), pinned at the extracted seam-event helper level. | `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd -- missing_seam_plan_fails_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative plan; long (~811 lines): ranged reads only (facts 5/11, D18, §5 request sketch, §6 R9/R10, §7, §8 exclusions).
- `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` + `design.md` — exports and the AC-N7 pin this packet retires (read-only; never edit that directory); 248's `packet.spec.md` AC-N3 only (the retargeted pin).
- `docs/19_visual_debug.md` — current user-facing contract; range-read post-247 (grown past its pre-247 232 lines); its "isolated overlay" section defines the 1.1.0 semantics this packet mirrors.

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — extend the silhouette section with a "Seam overlays" subsection: `overlays: ["seams"]` isolated form (exact 1.1.0 meaning on a silhouette base), `composited_overlays` semantics and R9 rules (seams-only, silhouette-only, model-source-only, 1.2.0-only), the `z` field on mirrored seam events (and its absence on 1.0/1.1 output), the `_overlay_seams` filename, and the glyph-over-palette visibility caveat — `rg -q 'composited_overlays' docs/19_visual_debug.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
