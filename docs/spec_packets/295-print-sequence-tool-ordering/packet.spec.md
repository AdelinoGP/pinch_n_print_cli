---
status: draft
packet: 295-print-sequence-tool-ordering
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/73-author-packet-p66-quality-layer-height-tool-ordering.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 73 (P66).
---

# Packet Contract: 295-print-sequence-tool-ordering

## Goal

Make the P66 print-sequence trio drive per-layer tool visitation order at parity with canonical `ToolOrdering` — `first_layer_print_sequence` / `other_layers_print_sequence` as global tool-index lists plus `other_layers_print_sequence_nums` as the repeat count — ported as one sequence-driven stable re-sort inside the host emitter, with no new module, IR field, WIT change, or manifest row.

## Scope Boundaries

P66 is three Tier B keys whose canonical behaviour is global tool ordering in `ToolOrdering.cpp`, not purge accounting: the first-layer list re-sorts first-layer tools, the other-layer list plus repeat count override per-layer order for layer ranges. Claim-time grounding (ticket 73) holds all three in: each is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree (no `crates/`/`modules/`/`xtask/` read, no `ORCA_CONFIG_PADDING` row, no prior packet). The tier table's `tool-ordering` owner is corrected to `crates/slicer-gcode` (ticket-27/39/40 precedent): this tree has no `ToolOrdering` module — per-layer tool order is owned by the host emission stage (`DefaultGCodeEmitter::emit_gcode` over `LayerCollectionIR`, with the existing `apply_cross_layer_tool_rotation`), so the sequence sort lands there. The packet declares three scalar-global `ResolvedConfig` fields (canonical defaults `[0.0]` / `[0.0]` / `0`, inert via the ported ignore-conditions), stable-sorts each covered layer's tools by sequence position, and leaves geometry untouched (pure reorder — entities carrying an `order_lock` tag stay pinned at authored positions per ADR-0062, so locked blocks never split or interleave). Canonical per-object first-layer area ordering and grouping-record ranges are recorded simplifications (DEV-187(b)+(c)); canonical's unenforced bounds are deliberately enforced here (DEV-187(d)); the `coInts`-vs-`float-list` spelling is a recorded shape divergence (DEV-187(a)).

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); the emission stage and `with_resolved_config` test seam are landed tree code, not packet dependencies.
- Related work, not a blocker: ticket 125 (per-tool model) — explicitly NOT this packet's axis (these are global ordered tool-index lists, not per-extruder vectors; sub-agent verified `coInts`/`coInt` scalar-global shape); ticket 122 (prime tower body) — ordering composes with any tower output, no sequencing edge; packet 294 (P65 flush-into) — purge-volume accounting, not order; no shared helper, no dep.
- Unblocks: wayfinder ticket 73 (P66 closes with all three keys in). No edge to any other draft packet.
- Activation blockers: none. DEV-187 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-186 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** default config, **when** `ResolvedConfig` is inspected, **then** all three keys exist with canonical defaults — `first_layer_print_sequence = [0.0]`, `other_layers_print_sequence = [0.0]`, `other_layers_print_sequence_nums = 0` — and a `1,0` / `1,0` / `2` config round-trips through `apply_cli_key` exactly. | `cargo test -p slicer-ir --test resolved_config_print_sequence_tdd schema_declares_print_sequence_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config, **when** a two-tool multi-layer print is emitted, **then** the emitted tool order per layer is byte-identical to the pre-packet baseline (defaults inert: first-layer list shorter than the layer's tool set, other-layer count zero). | `cargo test -p slicer-gcode --test print_sequence_ordering_tdd defaults_are_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `first_layer_print_sequence = [1, 0]` over a layer-0 carrying tools `{0, 1}`, **when** emission runs, **then** layer-0's tool visitation order is `[1, 0]` while layers ≥ 1 keep baseline order; and with the default `[0]` the order is unchanged from baseline. | `cargo test -p slicer-gcode --test print_sequence_ordering_tdd first_layer_sequence_resorts_layer_zero_only 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `other_layers_print_sequence = [1, 0]` with `other_layers_print_sequence_nums = 1` over layers 1..2 carrying tools `{0, 1}`, **when** emission runs, **then** each covered layer's tool visitation order is `[1, 0]`; and with nums `0` the order is baseline (count-zero inert). | `cargo test -p slicer-gcode --test print_sequence_ordering_tdd other_layer_sequence_and_zero_count 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `other_layers_print_sequence = [1, 0]` with `other_layers_print_sequence_nums = 2` over layers 1..4 carrying tools `{0, 1}`, **when** emission runs, **then** layers 1..2 (first cycle) and layers 3..4 (second cycle) each visit `[1, 0]`, while layer 5 (beyond `len(seq) × nums`) keeps baseline order. | `cargo test -p slicer-gcode --test print_sequence_ordering_tdd repeat_count_covers_first_n_cycles_only 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** `first_layer_print_sequence = [1, 0]` over a layer-0 whose tool-1 entities include a locked run (`path.order_lock = Some(tag)`) authored between two tool-0 entities, **when** emission runs, **then** every locked entity stays at its authored index while unlocked entities stable-partition around them (ADR-0062: locked blocks stay adjacent in authored order). | `cargo test -p slicer-gcode --test print_sequence_ordering_tdd locked_entities_pinned_at_authored_positions 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** the authored tree, **when** the CONFIG_BLOCK padding table is inspected, **then** no `*_print_sequence*` row exists in `ORCA_CONFIG_PADDING` and the table is untouched by this packet (host-only omission per the packet-287/288 precedent — ordering directives need no reader-visible spelling; canonical spelling rides ticket 132). | `rg -q 'print_sequence' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0`
- **AC-N2. Given** `first_layer_print_sequence = [-1, 0]`, **when** config resolution runs, **then** resolution rejects with a `TypeMismatch`-family error naming the key (negatives are never tool indices; canonical never enforces — DEV-187(d)). | `cargo test -p slicer-ir --test resolved_config_print_sequence_tdd negative_sequence_entry_rejected 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test print_sequence_ordering_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the host-emitter-seam shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (emission-stage ownership only — the sequence sort parameterises the existing host ordering stage; rule 4 trigger test does not fire: in-stage ordering parameter, not cross-module algorithm selection)
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (`from_declared` whitelist section only — inapplicable by design: host emitter reads `ResolvedConfig` directly, so no manifest row is declared; ticket-34 lesson cited as the reason no manifest work exists)

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-187" - `rg -q 'DEV-187' docs/DEVIATION_LOG.md` (filed in Step 4 with (a)+(b)+(c)+(d) clauses)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` section "Quality / Layer height" - `rg -q 'first_layer_print_sequence.*slicer-gcode' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (Step 4 owner correction)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` section "P66" - `rg -q '295-print-sequence' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (Step 4: 3 keys in at packet 295)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (the two `coInts` + one `coInt` declarations with defaults `{0}`/`{0}`/`0`; borrow the defaults exactly — port spells the lists `float-list`, DEV-187(a))
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `apply_first_layer_order` (borrow the sort-by-position shape and the shorter-than-tool-order ignore-condition; port the stable-sort with absent-tools-last)
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `ToolOrdering::generate_first_layer_tool_order` (both overloads) (named non-borrow for the area-ordered base — the port has no per-object first-layer area computation; cite as the DEV-187(c) evidence)
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `ToolOrdering::get_recommended_filament_maps` (borrow the empty-input / zero-count ignore shape; grouping-record ranges named non-borrow — DEV-187(b) evidence)
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `ToolOrdering::reorder_extruders_for_minimum_flush_volume` (borrow the repeated range/list record shape; flush-optimisation override named non-borrow — the port has no ordering optimisation to override, DEV-187(b) evidence)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
