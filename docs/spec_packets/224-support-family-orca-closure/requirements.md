# Requirements: support-family-orca-closure

## Packet Metadata
- Grouped task IDs: `TASK-335`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The support defect cannot close on typed captures or self-captured goldens. The decisive model and Orca reference files are present in this checkout, so closure must regenerate and inspect evidence from those fixtures.

## In Scope
- Real-fixture invariants for demand termination, exact-Z collision freedom, routing, overlap rejection, and support-disabled behavior.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` visual-debug taps with manifest-indexed PNG review. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- PNP versus standalone Orca tree/normal matched-height differential evidence.
- Final G-code support/interface role checks.
- Existing decisive fixtures: `tmp/SupportTest.stl`, `tmp/SupportTest_Tree_Orca.gcode`, and `tmp/SupportTest_Normal_Orca.gcode`.
- Closure disposition for packet 213, `TASK-329`, and `TASK-163b-orca-ref` without claiming exact path parity.

## Out of Scope
- Family planner or renderer implementation.
- New global scheduler, opaque family schema, or exact Orca path identity.
- Treating PNG existence, byte sizes, manifest greps, or self-captured goldens as sufficient parity evidence.

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§Visual And Differential Gates and Supersession And Compatibility.
- `docs/specs/support-generation-defect-verified-findings.md` - delegated SUMMARY; prior fixture/evidence context.
- `docs/19_visual_debug.md` - delegated SUMMARY; tap and manifest behavior.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388`, `:1839`, `:2652`, `:1969`, `:2050`, `:1772` - behavior used for matched-height review.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374`, `:2095`, `:2953`, `:3106` - behavior used for traditional matched-height review.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` - interface role reference.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants plus differential evidence.** Behaviour is pinned with invariant/property tests and inspected matched-height views against the existing standalone Orca references; claims never include exact path identity.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary
- Positive: `AC-1` through `AC-6`.
- Negative: `AC-N1` through `AC-N2`.
- Cross-packet impact: consumes TASK-334 routing diagnostics and exports closure evidence/dispositions.

## Verification Commands
| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test integration support_family_closure -- --exact` | Run fixture invariants and role closure checks through the aggregator target. | FACT pass/fail |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-tree --overwrite` | Render both families plus analysis/routing and support taps for matched-height inspection. | FACT plus manifest paths; PNG review delegated |
| `cargo check --workspace --all-targets` | Compile closure target and workspace. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint closure changes. | FACT pass/fail |

## Step Completion Expectations
Evidence must be regenerated from the existing fixtures, inspected at matched heights, and tied to manifest entries. Only an authority/provenance failure may remain an external TASK-163b blocker; no exact parity claim is permitted.

## Context Discipline Notes
Delegate all OrcaSlicer and docs reads, all cargo commands, and PNG/manifest inspection. Do not read target bundles directly in the authoring workflow.
