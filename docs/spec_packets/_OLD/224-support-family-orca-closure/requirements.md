# Requirements: support-family-orca-closure

## Packet Metadata
- Grouped task IDs: `TASK-335`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The support defect cannot close on typed captures or self-captured goldens. The decisive model and Orca reference files are present in this checkout, so closure must regenerate and inspect evidence from those fixtures.

**Closure basis (locked 2026-08-18).** This packet closes on **correctness plus honest tests**, not on canonical feature completeness. Every remaining canonical feature gap routes to a named follow-on packet through `docs/specs/support-parity-gap-register.md`. **Destinations corrected 2026-09-03: `docs/spec_packets/stubs/` never existed and no gap was ever routed there.** The real owning packets are `241-support-agg-rasterizer` (AGG rasterizer / `support_area_algorithm`), `239a-anchored-host-seams` / `239b-anchored-wit-contract` / `239c-support-layer-height-producer` / `239d-support-coarse-floating-planes` (independent support-layer Z), `238a-support-pattern-config-keys` (base/interface patterns, `support_expansion`, `support_bottom_z_distance`, dead config keys), `240a-support-raft-substrate` + `240b-support-raft-module` (raft), and `238c-support-renderer-flow-interfaces` (renderer/flow/interfaces, incl. `needs_support` eligibility follow-through). Always re-derive a gap's owner from the register's destination column, never from this sentence. A gap that is registered and routed is not a 224 blocker; an incorrect behaviour or a test that asserts nothing is.

## In Scope
- Real-fixture invariants for demand termination, exact-Z collision freedom, routing, overlap rejection, and support-disabled behavior.
- **Tree `support_top_z_distance_mm` (DONE, RC-11).** `tree-support-planner` declared the key in two manifests and read it in none. It is now read in `from_config` and the top gap is honoured. **Corrected 2026-09-03 — the two families honour it differently, and the earlier "shifts the contact layer along actual layer Z" claim is true only of traditional.** `traditional-support-planner::plan_for_object` walks actual layer Z (with a source comment rejecting the division); `tree-support-planner` computes a layer **count**, `z_distance_top_layers`, via `round_up_divide(mm_to_units(z_distance_top), mm_to_units(nominal_layer_height))` with `nominal_layer_height` taken from `layer_plan.layers[0].effective_layer_height` (`modules/core-modules/tree-support-planner/src/lib.rs`). **The tree planner is not a Z-walk and retains a layer-count computation; that is permitted.** What remains **prohibited** is dividing by the **per-entry** guest-view `LayerPlanViewEntry.effective_layer_height` of the layer being processed, which is unreliable in the guest view. The resulting gap **structure** differs from canonical; recorded as a deviation in `design.md` §Recorded deviation — top-Z gap structure.
- **Interface layer-count correctness in both families.** `support_interface_top_layers` / `support_interface_bottom_layers` must be honoured. ~~The measured 1-versus-3 `;TYPE:Support interface` block count on the normal family at `top_layers=2` / `bottom_layers=2` is a 224 blocker, not a routed gap.~~ **Corrected 2026-09-03.** That divergence, and the 2-versus-3 residue later registered as **G-18**, were **closed by `238c-support-renderer-flow-interfaces`** (commit `ea71ebc0`, reachable from HEAD): PnP and Orca now both emit **3** blocks at `top_layers=2` / `bottom_layers=2`, asserted by `interface_band_counts_match_canonical_structure` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`), which describes 3 as the canonical roof/floor structure. G-18's register destination column already names 238c. **This is no longer a 224 blocker and no longer an open 224 gap;** what remains in scope here is only that the two keys keep being honoured, re-verified after the Activation Gate.
- **Config-key reconciliation limited to the four support modules** (`tree-support-planner`, `traditional-support-planner`, `tree-support`, `traditional-support`). **Gated by AC-7 (`packet.spec.md`); this is a deliverable, not a note.** Every key declared in a module's manifest `[config.schema]` is either read by that module's `src/` or recorded as a dead key with an owning destination packet in `docs/specs/support-parity-gap-register.md`. No new xtask gate is introduced by this packet. The reconciliation must:
  1. **Enumerate the declared-but-unread keys.** At authoring time (2026-09-03) there were **five**, of which only `support_base_pattern_spacing` (G-03) is recorded: `tree-support-planner` / `support_branch_merge_distance_mm`; `traditional-support-planner` / `support_threshold_angle`, `support_overhang_angle` (its source comment says it "is no longer read here"), `support_base_pattern_spacing`; `tree-support` / `line_width`. **This is a ledger fact — re-derive it, do not quote it.**
  2. **Correct the two inverted register rows.** **G-16** claims `support_branch_merge_distance_mm` is "not declared in its manifest" — it *is* declared, in `modules/core-modules/tree-support-planner/tree-support-planner.toml`; the row is inverted. **G-22** claims "No module manifest declares `support_threshold_angle` or its legacy alias" — `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` declares it; that row is inverted too. Both must be rewritten to the declared-but-unread finding.
  3. **Dispose of the `tree-support` `line_width` / `support_line_width` mismatch — a live defect, not bookkeeping.** `modules/core-modules/tree-support/` declares `line_width` in its manifest while `modules/core-modules/tree-support/src/lib.rs` reads **`support_line_width`**, which that manifest does not declare. Because a module's config view is filtered to its declared schema, `support_line_width` always resolves to its `1.125 * nozzle_diameter` in-code default and the declared `line_width` is dead. Fix it (declare the key the module reads, or read the key the manifest declares) or record it explicitly with an owner.
  4. **Re-derive, never freeze.** Run this before acting on any of the above:

     ```bash
     for m in tree-support-planner traditional-support-planner tree-support traditional-support; do
       echo "== $m"
       sed -n 's/^\[config\.schema\.\([a-z0-9_]*\)\]/\1/p' "modules/core-modules/$m/$m.toml" | sort -u |
         while read -r k; do grep -rqF "\"$k\"" "modules/core-modules/$m/src/" || echo "  DECLARED-BUT-UNREAD: $k"; done
       grep -rhoE 'get(_[a-z_]+)?\("[a-z0-9_]+"' "modules/core-modules/$m/src/" | grep -oE '"[a-z0-9_]+"' | tr -d '"' | sort -u |
         while read -r k; do grep -qE "^\[config\.schema\.$k\]$" "modules/core-modules/$m/$m.toml" || echo "  READ-BUT-UNDECLARED: $k"; done
     done
     ```

     The `READ-BUT-UNDECLARED` half legitimately reports host-global keys (`layer_height`, `nozzle_diameter`, `support_family`, `support_type`) that no module manifest declares; those are not findings. `support_line_width` in `tree-support` **is** the finding in item 3.
  5. **Shared-file caution.** The register edits above are **224's work**, but `docs/specs/support-parity-gap-register.md` is **shared with `242-support-family-orca-closure`**. Re-read the file immediately before writing and coordinate; do not assume exclusive ownership.
- **Tree support-density diagnosis (DONE — root cause is RC-15).** The originally stated figures ("486.33 mm against Orca's 1538.36 mm (31.6%)") are **void**: they summed de-retraction prime `E`, which deposits nothing. On deposited material the tree deficit is **388.73 mm vs 683.96 mm = 56.8%, i.e. 1.76x**, decomposing into a **1.949x** support XY-path-length shortfall against a **1.107x** higher flow per mm. See `design.md` §Measured Baseline and `tree-density-diagnosis.md`.
- **Tree contact-point derivation (RC-15) — in scope for 224.** Tree contacts are currently mesh overhang-triangle centroids (one per triangle). Canonical `TreeSupport::generate_contact_points` samples the per-layer overhang `ExPolygon` (contour corners, arc walk at `tree_support_branch_distance`, rotated interior grid), deduped by a `base_radius` hash grid. Classified a **GAP**, but explicitly **not** routed to the gap register: it is the dominant cause of the coverage deficit and every other tree parity claim in this packet is unmeasurable without it.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` visual-debug taps with manifest-indexed PNG review. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- Structural-invariant parity gating plus a written human/LLM `/visual-debug` inspection checklist with side-by-side Orca renders at matched heights. **No test may read the Orca G-code**, and no Orca-derived constant may be hardcoded into a test.
- Final G-code support/interface role checks.
- Honest-test remediation: delete tests that assert nothing and dead helpers; replace them with invariants on tracked `resources/` models.
- ~~Regeneration of the benchy golden **last**, after every fix, renamed off `orca_parity` to a regression-tripwire name and carrying a provenance header stating it is a PnP self-capture and **not** parity evidence.~~ **RESOLVED BY SUPERSESSION (recorded 2026-09-03) — the golden was DELETED, not regenerated.** `resources/golden/` no longer exists; commit `ac9466c6` (reachable from HEAD) "replace the self-captured tripwire with algorithmic invariants" removed the self-capture in favour of invariants. That better serves this packet's own rule that self-captured goldens are not parity evidence — there is now no self-capture to mislabel — so **the regeneration and provenance-header obligations are dropped.** The **rename obligation survives in reduced form and still applies**: the surviving test file `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` still carries the `orca_parity` name while asserting algorithmic invariants rather than Orca parity, and must be renamed to a name that does not claim parity.
- Closure disposition for packet 213, `TASK-329`, and `TASK-163b-orca-ref` without claiming exact path parity.
- Decisive fixture: `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` (tracked in-repo, authoritative). Orca references `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode` are gitignored inspection aids. **`tmp/` copies of the model are not authoritative** and must not be referenced as the fixture path.

## Out of Scope
Routed to follow-on packets via `docs/specs/support-parity-gap-register.md`; none of these may be implemented here:
*Destinations corrected 2026-09-03. The previously named packets 225/226/227 are unrelated work (`225-dragon-curve-feasibility-gate`, `226-authored-coloring-carrier`, `227-dragon-curve-community-module`) and `docs/spec_packets/stubs/` does not exist. Re-derive each owner from the register's destination column.*

- Support **base pattern** and **interface pattern** generators, including `support_base_pattern` and `support_base_pattern_spacing` behaviour (`238a-support-pattern-config-keys`).
- `support_expansion` (`238a-support-pattern-config-keys`).
- `support_bottom_z_distance` (`238a-support-pattern-config-keys`).
- Raft geometry and all raft config keys (`240a-support-raft-substrate` for the substrate, `240b-support-raft-module` for the module).
- Independent support-layer Z (`239a-anchored-host-seams`, `239b-anchored-wit-contract`, `239c-support-layer-height-producer`, `239d-support-coarse-floating-planes`). **The original justification is void.** It read: "Orca's `normal` reference emits 205 distinct print Z for a 150-layer print; PnP emits 150." That divergence no longer exists in the evidence: the Orca references were **regenerated on 2026-08-18 with `independent_support_layer_height` disabled**, and both PnP and Orca now emit **150 distinct print Z** (see `design.md` §Measured Baseline). Do not quote the 205 figure. The routing to packet 225 stands on a different and correct basis: independent support-layer Z is a **canonical feature PnP does not have**, not a divergence measurable against the current references. It is out of scope here because implementing an absent feature is not closure work, and because it cannot be gated by the references this packet uses.
- The `SupportGridPattern` AGG rasterizer and any `support_area_algorithm` key (`241-support-agg-rasterizer`).
- Raft keys and `support_base_pattern` stay as-is (dead) in the four support modules; they are recorded in the gap register rather than removed or wired. Recording them is in scope (AC-7); wiring or removing them is not.
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

*Line-pinned citations removed 2026-09-03 per `CLAUDE.md` §OrcaSlicer Citation Style; cite by file + function only. Each function below was resolved against the local `OrcaSlicerDocumented/` checkout.*

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_contact_points` (tree contact-point derivation); `TreeSupport::get_collision` (collision-avoidance volume lookup); `TreeSupport::drop_nodes` (branch body propagation between layers); `TreeSupport::draw_circles` (builds the base, roof, and first-layer-roof **areas** — it is not a dedicated interface routine, so do not describe it as one); `TreeSupport::calc_branch_radius` (branch-radius taper). Behaviour used for matched-height review.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — the class is **`PrintObjectSupportMaterial`**, not `SupportMaterial`: `PrintObjectSupportMaterial::generate` (traditional orchestration); `PrintObjectSupportMaterial::top_contact_layers` (contacts); `PrintObjectSupportMaterial::generate_base_layers` (downward propagation); `PrintObjectSupportMaterial::trim_support_layers_by_object` (collision trimming). Behaviour used for traditional matched-height review.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `Slic3r::generate_interface_layers` (free function in namespace `Slic3r`); interface role and layer-count reference.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants plus differential evidence.** Behaviour is pinned with invariant/property tests and inspected matched-height views against the existing standalone Orca references; claims never include exact path identity.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary
- Positive: `AC-1` through `AC-7` (`AC-7`, config-key reconciliation, added 2026-09-03).
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
