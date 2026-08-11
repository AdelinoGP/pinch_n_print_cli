---
status: draft
packet: 218-support-gcode-e2e
task_ids:
  - TASK-327
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 218-support-gcode-e2e

## Goal

Add a visual-debug G-code-mode regression that renders the fixed support artifact and proves its canonical support `;TYPE:` markers coexist with the expected final-G-code layer images.

## Scope Boundaries

This is verification-only: it adds a targeted end-to-end check and uses the existing `tmp/SupportTest_Normal_Orca.gcode` reproduction. It does not modify G-code emission, serialization, support geometry, or role definitions.

## Prerequisites and Blockers

- Depends on: activation-blocking FORWARD-DEPs `TASK-322`, `TASK-323`, `TASK-324`, and `TASK-325`.
- Unblocks: support-generation remediation closure.
- Activation blockers:
  - `TASK-322` FORWARD-DEP: produces the fixed support-planner lone-node columns and tip-radius floor output.
  - `TASK-323` FORWARD-DEP: produces fallback support geometry clipped to overhang areas and the corrected `needs_support` boundary result.
  - `TASK-324` FORWARD-DEP: produces raft geometry in `SupportIR.raft_paths`, including the negative-layer raft prefix.
  - `TASK-325` FORWARD-DEP: produces planner-generated top and bottom support-interface layer geometry without the code-1003 warning.
  - This packet cannot activate until all four geometry packets are implemented; it verifies their fixed output end-to-end.
  - The named `tmp/` request and G-code inputs are gitignored on-disk fixtures and must be present before the acceptance commands run.

## Acceptance Criteria

- **AC-1. Given** `tmp/visual-debug-gcode.json` requests G-code source `tmp/SupportTest_Normal_Orca.gcode`, tap `final_gcode`, `gcode_line_width_mm` `0.4`, and layer `30`, **when** `pnp_cli visual-debug` runs, **then** `target/vd-gcode-e2e/manifest.json` is valid JSON with `source.kind == "gcode"`, one `images` entry whose `tap == "final_gcode"`, `source == "gcode"`, and `layer_index == 30`. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-gcode.json --output target/vd-gcode-e2e --overwrite >/dev/null && python3 -c "import json; m=json.load(open('target/vd-gcode-e2e/manifest.json')); assert m['source']['kind']=='gcode'; a=[x for x in m['images'] if x['tap']=='final_gcode']; assert len(a)==1 and a[0]['source']=='gcode' and a[0]['layer_index']==30"
- **AC-2. Given** `tmp/SupportTest_Normal_Orca.gcode` contains the exact lines `;TYPE:Support` and `;TYPE:Support interface`, **when** the G-code-mode visual-debug check renders layers `31`, `32`, `33`, and `34`, **then** the rendered bundle contains exactly four `final_gcode` image entries with `layer_index` values `31`, `32`, `33`, and `34`; this criterion asserts the raw source markers and rendered images only, not typed role semantics in the standalone G-code manifest. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-gcode2.json --output target/vd-gcode-e2e-roles --overwrite >/dev/null && rg -q '^;TYPE:Support$' tmp/SupportTest_Normal_Orca.gcode && rg -q '^;TYPE:Support interface$' tmp/SupportTest_Normal_Orca.gcode && python3 -c "import json; m=json.load(open('target/vd-gcode-e2e-roles/manifest.json')); a=[x for x in m['images'] if x['tap']=='final_gcode']; assert len(a)==4 and {x['layer_index'] for x in a}=={31,32,33,34}"
- **AC-N1. Given** a G-code-mode request selects `filled_areas` without `gcode_line_width_mm`, **when** validation runs, **then** it rejects with `ValidationError::GcodeLineWidth` and writes no `manifest.json`; emission behavior is not changed. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd ac_n1_rejects_filled_areas_without_line_width`

## Verification

- `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct full read; approved queue row 6 and dependency list.
- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges 178-231 and 246-265; exact G-code requests, manifest caveat, role/emission scope.
- `docs/19_visual_debug.md` - direct ranges 17-46 and 158-180; request and manifest reading contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionEntity.cpp` — verify the canonical raw support `;TYPE:` marker spelling represented in final G-code.

## Doc Impact Statement (Required)

**`none`** - this packet adds verification only and changes no IR, WIT, scheduler, manifest, host-service, SDK, or emission contract.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
