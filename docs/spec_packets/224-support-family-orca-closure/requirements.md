# Requirements: support-family-orca-closure

## Packet Metadata
- Grouped task IDs: `TASK-335`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The support defect cannot close on typed captures or self-captured goldens. The decisive model and Orca reference files are present in this checkout, so closure must regenerate and inspect evidence from those fixtures.

**Closure basis (locked 2026-08-18).** This packet closes on **correctness plus honest tests**, not on canonical feature completeness. Every remaining canonical feature gap routes to a named follow-on packet through `docs/specs/support-parity-gap-register.md` (unnumbered stubs under `docs/spec_packets/stubs/`: AGG rasterizer, independent support-layer Z, base/interface patterns + expansion + bottom-Z, raft, `needs_support` eligibility). A gap that is registered and routed is not a 224 blocker; an incorrect behaviour or a test that asserts nothing is.

## In Scope
- Real-fixture invariants for demand termination, exact-Z collision freedom, routing, overlap rejection, and support-disabled behavior.
- **Tree `support_top_z_distance_mm` (DONE — `d97fb2b8`, RC-11).** `tree-support-planner` declared the key in two manifests and read it in none. It is now read in `from_config` and honoured by shifting the contact layer along **actual layer Z**, the technique `traditional-support-planner::plan_for_object` already uses. Dividing by `LayerPlanViewEntry.effective_layer_height` remains prohibited (the field is unreliable in the guest view). The resulting gap **structure** differs from canonical; recorded as a deviation in `design.md` §Recorded deviation — top-Z gap structure.
- **Interface layer-count correctness in both families.** `support_interface_top_layers` / `support_interface_bottom_layers` must be honoured. The measured 1-versus-3 `;TYPE:Support interface` block count on the normal family at `top_layers=2` / `bottom_layers=2` is a 224 blocker, not a routed gap.
- **Config-key reconciliation limited to the four support modules** (`tree-support-planner`, `traditional-support-planner`, `tree-support`, `traditional-support`): every key declared in a manifest is either read by that module or recorded as a dead key in the gap register. No new xtask gate is introduced by this packet.
- **Tree support-density diagnosis (DONE — root cause is RC-15).** The originally stated figures ("486.33 mm against Orca's 1538.36 mm (31.6%)") are **void**: they summed de-retraction prime `E`, which deposits nothing. On deposited material the tree deficit is **388.73 mm vs 683.96 mm = 56.8%, i.e. 1.76x**, decomposing into a **1.949x** support XY-path-length shortfall against a **1.107x** higher flow per mm. See `design.md` §Measured Baseline and `tree-density-diagnosis.md`.
- **Tree contact-point derivation (RC-15) — in scope for 224.** Tree contacts are currently mesh overhang-triangle centroids (one per triangle). Canonical `TreeSupport::generate_contact_points` samples the per-layer overhang `ExPolygon` (contour corners, arc walk at `tree_support_branch_distance`, rotated interior grid), deduped by a `base_radius` hash grid. Classified a **GAP**, but explicitly **not** routed to the gap register: it is the dominant cause of the coverage deficit and every other tree parity claim in this packet is unmeasurable without it.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` visual-debug taps with manifest-indexed PNG review. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- Structural-invariant parity gating plus a written human/LLM `/visual-debug` inspection checklist with side-by-side Orca renders at matched heights. **No test may read the Orca G-code**, and no Orca-derived constant may be hardcoded into a test.
- Final G-code support/interface role checks.
- Honest-test remediation: delete tests that assert nothing and dead helpers; replace them with invariants on tracked `resources/` models.
- Regeneration of the benchy golden **last**, after every fix, renamed off `orca_parity` to a regression-tripwire name and carrying a provenance header stating it is a PnP self-capture and **not** parity evidence.
- Closure disposition for packet 213, `TASK-329`, and `TASK-163b-orca-ref` without claiming exact path parity.
- Decisive fixture: `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` (tracked in-repo, authoritative). Orca references `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode` are gitignored inspection aids. **`tmp/` copies of the model are not authoritative** and must not be referenced as the fixture path.

## Out of Scope
Routed to follow-on packets via `docs/specs/support-parity-gap-register.md`; none of these may be implemented here:
- Support **base pattern** and **interface pattern** generators, including `support_base_pattern` and `support_base_pattern_spacing` behaviour (packet 226).
- `support_expansion` (packet 226).
- `support_bottom_z_distance` (packet 226).
- Raft geometry and all raft config keys (packet 227).
- Independent support-layer Z (packet 225). **The original justification is void.** It read: "Orca's `normal` reference emits 205 distinct print Z for a 150-layer print; PnP emits 150." That divergence no longer exists in the evidence: the Orca references were **regenerated on 2026-08-18 with `independent_support_layer_height` disabled**, and both PnP and Orca now emit **150 distinct print Z** (see `design.md` §Measured Baseline). Do not quote the 205 figure. The routing to packet 225 stands on a different and correct basis: independent support-layer Z is a **canonical feature PnP does not have**, not a divergence measurable against the current references. It is out of scope here because implementing an absent feature is not closure work, and because it cannot be gated by the references this packet uses.
- The `SupportGridPattern` AGG rasterizer and any `support_area_algorithm` key (stub `support-agg-rasterizer`).
- Raft keys and `support_base_pattern` stay as-is (dead) in the four support modules; they are recorded in the gap register rather than removed or wired.
- New global scheduler, opaque family schema, or exact Orca path identity.
- New xtask gates or workspace-wide config-key audits.
- Treating PNG existence, byte sizes, manifest greps, extruding-move counts, or self-captured goldens as sufficient parity evidence.
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
| `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact` | Run fixture invariants and role closure checks through the aggregator target. | FACT pass/fail |

**Test-name filters, corrected 2026-08-18.** Every command in this packet previously carried `support_family_closure` as a filter token. **That form runs ZERO tests** — it reports `0 passed; ... 306 filtered out` and exits `0`, which reads as a pass. The closure tests are registered in `crates/slicer-runtime/tests/integration/main.rs` as bare `#[test] fn` wrappers (`fn fixture_invariants()` calling `support_family_closure::fixture_invariants()`), so **no test name carries a module prefix** and nothing matches `support_family_closure`. Always filter on the explicit test name with `--exact`, and always check the `N passed` count is non-zero.
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-support-family-tree --overwrite` | Render the **tree** family plus analysis/routing and support taps for matched-height inspection. | FACT plus manifest paths; PNG review delegated |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-normal.json --output target/vd-support-family-normal --overwrite` | Render the **traditional** family at the same physical heights. Two distinct per-family requests; the old single-request-twice command is void. | FACT plus manifest paths; PNG review delegated |
| `cargo check --workspace --all-targets` | Compile closure target and workspace. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint closure changes. | FACT pass/fail |

## Step Completion Expectations
Evidence must be regenerated from the tracked fixture (`crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`), inspected at matched heights against side-by-side Orca renders, and tied to manifest entries. Parity gating is structural invariants plus the written inspection checklist; no test reads the Orca G-code. Only an authority/provenance failure may remain an external TASK-163b blocker; no exact parity claim is permitted.

## Context Discipline Notes
Delegate all OrcaSlicer and docs reads, all cargo commands, and PNG/manifest inspection. Do not read target bundles directly in the authoring workflow.
