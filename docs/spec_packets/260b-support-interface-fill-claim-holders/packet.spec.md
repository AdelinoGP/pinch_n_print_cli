---
status: draft
packet: support-interface-fill-claim-holders
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/18-author-packet-p11-support-interface-support-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P11, re-authored to the map's Authoring rules 1–7; split 260 → 260a/260b)
context_cost_estimate: L
---

# Packet Contract: support-interface-fill-claim-holders

> **This packet carries open `[BLOCK]` entries and MUST NOT be activated until they are resolved.** See `design.md` §Open Questions. The blockers are structural (a new host selector field, a new claim ID, an unresolved angle-metadata question, and an unchecked ADR-0059 conformance question), not authoring gaps — the packet is written to be executable the moment they are ruled.

## Goal

Make OrcaSlicer's `support_interface_pattern` real in this tree the PnP way (Authoring rule 4): ship the non-rectilinear interface fill algorithms as **separate modules holding a new `claim:support-interface-fill`**, selected per print and per region through the claim seam rather than through an enum declared on the support renderer — so that a print configured for concentric or grid interface fill gets concentric or grid interface toolpaths, with canonical's branch order and per-pattern interface angles.

## Scope Boundaries

The packet ships the claim seam and at least the `concentric` and `grid` interface fillers (the two canonical values whose geometry the port has no equivalent of), leaves `rectilinear` to the support renderers' existing scan-line filler (already structurally faithful — canonical's sparse-density default resolves to `ipSupportBase`, a `FillSupportBase : FillRectilinear` filler at `spacing/density`), and implements canonical's `auto` resolution as the branch order in `SupportParameters::SupportParameters`. `rectilinear_interlaced` may ship in this packet or be returned to the queue; §Returned to Queue in `requirements.md` records whichever the implementer's canonical dispatch supports.

It does **not** touch the interface *spacing* keys, the contact-loop pass, or the manifests' spacing entries — those are packet `260a-support-interface-spacing-and-loops`, which must land first. It does not add `support_interface_pattern` as a config key anywhere: Authoring rule 4's "holder-only, always" ruling forbids declaring the Orca enum as an input key, even as a host-side alias onto a holder name. It does not touch `ORCA_CONFIG_PADDING` or any CONFIG_BLOCK twin.

## Prerequisites and Blockers

- Depends on: `260a-support-interface-spacing-and-loops` (it owns the interface emission seam this packet's fillers replace, and the manifests both packets touch).
- **Activation blockers — all four are open `[BLOCK]`s in `design.md`:** the holder selector does not exist; two modules writing `SupportIR` in one layer stage is unruled; the per-pattern interface angle may need plan metadata the IR does not carry; conformance with ADR-0059 is unchecked. Status stays `draft`.

## Acceptance Criteria

- **AC-1. Given** the new filler modules' manifests, **when** each `[claims]` table is parsed, **then** each holds exactly `claim:support-interface-fill` and no other claim (in particular not `support-generator`, which stays with the renderers), and each declares `[stage] id = "Layer::Support"` with `[ir-access] reads` including `SupportPlanIR` and `writes = ["SupportIR"]`. | `cargo test -p slicer-scheduler --test scheduler_integration support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a slice over a fixture with top-interface geometry, **when** the interface-fill holder selects the concentric module (a non-default selection — the default holder remains the renderer's built-in rectilinear path), **then** the emitted `SupportInterface` extrusions for that layer are closed, nested loops whose count equals the number of inward offsets that fit the island at the interface line width, and the run differs from the default-holder run; **and** with the grid module selected the same layer emits two crossing line families instead. | `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** canonical's `auto` resolution, **when** the resolved interface density is above canonical's 0.95 threshold, **then** the selected filler is the rectilinear path, and **when** the interface gap resolves to zero, **then** it is the concentric path — reproducing the `contact_fill_pattern` branch order in `SupportParameters::SupportParameters` (grid → grid; interlaced → rectilinear; (auto ∧ zero-gap) ∨ concentric → concentric; density > 0.95 → rectilinear; else the support-base rectilinear family). | `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** the per-pattern interface angle rules in canonical `support_interface_angle()`, **when** the grid filler runs, **then** its line family angle equals the support base angle, **and when** the rectilinear path runs on a snug support, **then** its angle is the interface angle less 45° — each asserted against the emitted path direction, not against a config value. | `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a region-level override selecting a different interface filler than the print-level holder, **when** the slice runs, **then** only that region's interface extrusions change and every other region's are byte-identical to the un-overridden run — the per-region capability canonical does not have, recorded as a divergence in `design.md`. | `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Negative Test Cases

- **AC-N1. Given** an interface-fill holder naming a module no loaded manifest matches, **when** the DAG is validated at startup, **then** validation fails with a structured scheduler error naming the unmatched holder — it must not silently yield an interface-free support surface. This is the failure mode Authoring rule 4's holder-only ruling explicitly calls out as today's gap (`resolve_held_claims` in `crates/slicer-scheduler/src/validation.rs` returns empty for every module and no `SchedulerError` variant covers it), so this packet either adds the variant or inherits it from whichever packet does. | `cargo test -p slicer-scheduler --test scheduler_integration support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** no interface-fill holder is configured, **when** a slice runs, **then** the support renderers emit their own interface scan fill exactly as they do before this packet, byte-identical on the default path — shipping the seam must not move the default output. | `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest lint, **when** any manifest in the tree declares a config key named `support_interface_pattern`, **then** the lint fails — the Orca enum is never an input key (Authoring rule 4, holder-only ruling). | `rg -l 'support_interface_pattern' modules/core-modules/*/[a-z-]*.toml; test $? -ne 0; echo "exit=$?"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contract), then `cargo xtask build-guests --check; echo "exit=$?"` — the new modules are guest-fingerprint inputs and must return exit 0 before closure.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System and § Claim Conflict Resolution — the resolution order this packet's seam must obey.
- `docs/03_wit_and_manifest.md` § Known claim IDs and § Support-family claims — where the new claim ID is registered and how it must relate to `support-generator` / `support-family:<id>`.
- `docs/04_host_scheduler.md` § Claim Resolution — holder matching and the planner/renderer pairing rules the new claim must not break.
- `docs/08_coordinate_system.md` — the fillers are geometry; see `design.md` §Architecture Constraints.

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` — § Known claim IDs gains a `claim:support-interface-fill` row (purpose, dedup rule, owner modules), and § Support-family claims gains a paragraph stating how the new claim relates to `support-generator` (the renderer keeps `support-generator`; the filler holds only the fill claim; an unmatched holder is a validation failure, not a degraded pair). Verification: `rg -q 'claim:support-interface-fill' docs/03_wit_and_manifest.md`.
- `docs/15_config_keys_reference.md` — regenerated for the new modules' own config keys, if any. No `support_interface_pattern` row is added anywhere: the enum is not a key.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters` (the `contact_fill_pattern` branch order AC-3 reproduces) and `support_interface_angle()` (the per-pattern angles AC-4 reproduces: snug −45°; interlaced ±45° by layer parity; grid = base angle; auto/concentric = interface angle).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (filler construction via `Fill::new_from_type`, and what parameters the filler receives).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — the `SupportMaterialInterfacePattern` enum members, for the value→module mapping.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillConcentric.cpp` and `Fill/FillRectilinear.cpp` — the concentric and grid generators being ported (`FillGrid` is a `FillRectilinear` subclass emitting two crossing families).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
