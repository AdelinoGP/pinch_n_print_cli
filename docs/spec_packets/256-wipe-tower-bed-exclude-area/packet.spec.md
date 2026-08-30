---
status: draft
packet: 256-wipe-tower-bed-exclude-area
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P04)
context_cost_estimate: M
---

# Packet Contract: 256-wipe-tower-bed-exclude-area

This packet was authored per the useful grounding in `requirements.md` §Verified Grounding; all symbol paths below were verified against the current tree (and the canonical checkout, via delegation) at authoring time.

## Goal

Close gap packet P04 by declaring OrcaSlicer's `bed_exclude_area` (`coPoints`, Orca default a degenerate single point at (0,0)) in the `wipe-tower` module manifest and wiring it into the module's existing bed-bounds validation decision point (`run_finalization`), so a tower corner inside a configured exclusion polygon fails the slice with a collision-risk error — mirroring canonical's fatal outcome — while an unset or degenerate key stays byte-identical to today's output.

## Scope Boundaries

One key (`bed_exclude_area`) is declared in `modules/core-modules/wipe-tower/wipe-tower.toml` (type `float-list`, no default, `group = "Printer"`, `advanced = true` mirroring Orca's `comAdvanced`) and consumed by the wipe-tower module: `from_config` parses the polygon (flat interleaved floats, or Orca 3MF point strings via the existing `slicer_ir::parse_orca_point_string` reader arm), and `run_finalization` extends its 4-corner bed-polygon validation with an exclusion-polygon check. The port's decision point validates the **tower rectangle** (the only live bed-validation decision point in this tree); canonical additionally intersects **object volume convex hulls** — that reduced-semantics gap is recorded, not built (Tier B/C future work). No host-crate logic changes, no WIT/IR shape change, no schema bump, no new module.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 + 100 (both resolved — packet-number rule; the `printable_area`/Orca-point-string adaptation this packet's reader arm builds on), ticket 05 for P04 membership, ticket 04 for tier placement.
- Queue order: packets 254/255 (P02/P03, same owner, `draft`) may land before or after this packet; nothing here conflicts with their keys, and the schema-test union assertion re-derives the manifest base from disk at implementation time.
- Unblocks: the next packet in `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` order (P05 — Others / Brim — skirt-brim, ticket 12).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1 (declaration).** Given the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares `bed_exclude_area` with `type = "float-list"`, no `default` key, no `min`/`max`, `display = "Excluded bed area"`, `group = "Printer"`, `advanced = true` — alongside the pre-existing keys re-derived from disk (8 today; plus P02/P03 keys if packets 254/255 landed first). | `cargo test -p wipe-tower --test wipe_tower_bed_exclude_area_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo xtask build-guests --check; echo "exit=$?"` (manifest feeds the guest fingerprint; ticket 101 established guests embed config key names).
- **AC-2 (wiring, both directions + identity).** Given the wired `bed_exclude_area`, **when** `WipeTower::from_config` + `run_finalization` run, **then**: (i) with no `bed_exclude_area` entry the validation is byte-identical to today (a fitting tower still `Ok`, an out-of-bed tower still fails with the pre-existing "outside bed polygon" error); (ii) a tower corner inside a configured exclusion polygon → `Err` whose message names `bed_exclude_area` and the corner coordinates; (iii) a tower fully outside the exclusion polygon but inside the bed → `Ok`; (iv) an empty list or any too-short/odd raw value → treated as degenerate (no exclusion) rather than an error — mirroring Orca's own degenerate single-point default, which excludes nothing. All four cases pass on **both** execution paths' shared decision point. | `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3 (Orca 3MF ingest).** Given `bed_exclude_area` supplied in Orca's 3MF serialisation (an array of point strings `["0x0", "20x0", "20x20", "0x20"]`), **when** config resolution binds the module's view, **then** the polygon expands to interleaved floats and a tower corner at (10, 10) is rejected — the same regression shape `bed_bounds_tdd.rs` pins for `printable_area` (ticket 100's silent-default lesson applies to this key). | `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N1 (no leakage).** Given a core module other than `wipe-tower` (e.g. `part-cooling`, whose manifest declares none of these keys), **when** it receives resolved config containing `bed_exclude_area`, **then** `ConfigView::from_declared` still hides the key — the declaration leaks no wipe-tower config into modules that did not opt in. | `cargo test -p slicer-scheduler --test wipe_tower_p04_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4 (docs).** `docs/15_config_keys_reference.md` regenerated tables list `bed_exclude_area` under owner `wipe-tower` (module-config-keys table), and its Orca-deviations table gains **no** new row for the key (the manifest carries no numeric default, so the generator's comparator — booleans and numerics only, per ticket 100's fix — has no comparand; this is the same non-row `printable_area` already renders as `—`). | `cargo xtask gen-config-docs --check 2>&1 | tail -3 && rg -q 'bed_exclude_area' docs/15_config_keys_reference.md && echo AC4-PASS`

## Negative Test Cases

- **AC-N1 above** is the packet's rejection case (cross-module hiding).
- **AC-2(iv)** is the malformed-value negative case: partial garbage decays to "no exclusion" (canonical `get_bed_excluded_area` likewise builds a degenerate polygon from any point count and excludes nothing), not a slice failure.

## Verification

Gate commands (the authoritative full matrix lives in `requirements.md` §Verification Commands):

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md` — the wayfinder ticket defining this packet's scope (direct read).
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P04 row: 1 key, Tier A (ranged read, ~7 lines around the P04 heading).
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the `bed_exclude_area` Tier A row (ranged read ~15 lines; over 300 lines total: delegate beyond these rows).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — evidence standard (direct read).
- `docs/15_config_keys_reference.md` — regeneration target; never hand-edited (delegated regen + grep verification only).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` module-config-keys table — regenerated by `cargo xtask gen-config-docs`; verification grep: `rg -q 'bed_exclude_area' docs/15_config_keys_reference.md`
- No prose doc describes bed-exclusion behaviour today (`bed_exclude_area` has zero code and zero prose-doc occurrences outside the reference/gap docs at authoring time); if implementation finds one (grep `exclude` under `docs/*.md`), it names the packet in its amendment.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` (`bed_exclude_area` definition: `coPoints`, `comAdvanced`, default `Vec2d(0,0)`, no min/max) and `get_bed_excluded_area` (all points → one counter-clockwise polygon, no rectangle pairing).
- `src/libslic3r/Print.cpp` — `Print::validate` routing to `layered_print_cleareance_valid` / `sequential_print_clearance_valid`: object volume convex hulls intersected with the exclude polygon, fatal `"<object> is too close to exclusion area, there may be collisions when printing."`; the wipe tower itself is never tested against the key.
- `src/libslic3r/GCode.cpp` — `get_path_of_change_filament` (4-point cutter-area form) — secondary consumer, recorded as a future-work gap, not imitated.
- `src/libslic3r/GCode/GCodeProcessor.cpp` — `apply_config` (viewer copy) and `src/libslic3r/GCode/TimelapsePosPicker.cpp` — `construct_printable_area_by_printer` (subtractive use) — secondary consumers, gap-recorded.
<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).