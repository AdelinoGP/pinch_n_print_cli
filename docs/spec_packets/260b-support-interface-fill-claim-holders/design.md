# Design: support-interface-fill-claim-holders

## Tier Derivation

**Tier C.** Authoring rule 1 re-tiers a packet that builds a decision point to B or C. This one builds a decision point *and* a new claim seam — a new claim ID, a new selector, net-new geometry modules, and a scheduler validation variant. That is above B's ceiling on both surface count and contract reach, so C. The pre-rules packet 260's Tier A assignment for this key is superseded; the `04-asset-tier-assignment.md` row needs the correction (reported for ticket 18's closure, not applied here).

## Selected Approach

Support interface fill becomes a claim-held role, mirroring the fill-role claims packet 37 introduced for infill:

1. A new claim `claim:support-interface-fill` is registered in `docs/03_wit_and_manifest.md` § Known claim IDs and recognized by the scheduler alongside the existing support-family vocabulary. The renderer keeps `support-generator` and its `support-family:<id>`; the filler holds **only** the new claim.
2. Each shipped canonical pattern is one module holding that claim — `concentric` and `grid` at minimum. `rectilinear` is not a module: the support renderers' existing scan-line filler already is the rectilinear family (canonical's sparse-density default resolves to `ipSupportBase`, a `FillSupportBase : FillRectilinear` at `spacing/density`), so "no holder configured" *is* rectilinear, which is why AC-N2's default path must stay byte-identical.
3. Selection is by holder plus region override only. Per rule 4's holder-only ruling, `support_interface_pattern` is never declared as a key — an Orca 3MF setting naming an unshipped pattern is silently dropped, and a holder naming no loaded module fails validation (AC-N1) rather than yielding a hollow interface.
4. Canonical's `auto` is not a module: it is the branch order in `SupportParameters::SupportParameters`, evaluated where the holder is resolved (AC-3).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Claims held:** the two (or three) net-new filler modules each hold exactly `claim:support-interface-fill`. Nothing else in the tree holds it. The renderers' claims are unchanged.
- **Mechanism carrying the data:** the existing `SupportPlanIR` per-layer interface geometry (top/base/bottom) is already in the IR, so the filler reads what it needs without an IR change — with one unresolved exception, `[BLOCK-3]`. Output goes into the existing `SupportIR` the renderers already write; two modules writing `SupportIR` in the same layer stage is the seam's central scheduler question, `[BLOCK-2]`.
- **Two modules, one stage, one output.** The renderer must stop emitting interface fill exactly when a filler holds the claim for that region. The switch is claim resolution, not a config flag inside the renderer.

## Divergences Recorded (rule 4: improving on canonical is in scope)

1. **Per-region interface pattern (AC-5).** Canonical resolves `contact_fill_pattern` once per print object. The claim seam gives per-region selection for free. Taken deliberately, recorded here, and pinned by AC-5 rather than left as an accident.
2. **Unmatched holder is a hard failure (AC-N1).** Canonical cannot express the situation. The port's alternative — silently emitting no interface — is worse than an error, so the seam fails validation. This matches Authoring rule 4's holder-only ruling.
3. **`auto` evaluated at holder resolution, not inside a filler.** Canonical folds the `auto` branch into `SupportParameters`' constructor. Keeping it at the seam means every filler is a pure algorithm with no mode logic, which is what makes them replaceable by community modules.

## Code Change Surface

- `docs/03_wit_and_manifest.md` — § Known claim IDs row + § Support-family claims paragraph for `claim:support-interface-fill`.
- `crates/slicer-scheduler/src/**` — recognize the new claim; resolve its holder; suppress the renderer's own interface fill when a holder is resolved; add the unmatched-holder `SchedulerError` variant (AC-N1).
- **`[BLOCK-1]`** the selector — see §Open Questions. Whichever resolution lands, its surface is named there, not assumed here.
- `modules/core-modules/support-interface-concentric/**` — net-new module (manifest, `src/lib.rs`, guest wrapper, tests). Copy `lightning-infill`'s layout: single-claim module, `slicer_sdk::test_prelude` harness.
- `modules/core-modules/support-interface-grid/**` — net-new module, same shape.
- `modules/core-modules/{traditional-support,tree-support}/src/lib.rs` — the interface emission seam `260a` leaves: skip own interface fill when a filler holds the claim.
- `crates/slicer-scheduler/tests/integration/support_interface_fill_claim_resolution_tdd.rs` — net-new (AC-1, AC-N1), registered as a `mod` in `crates/slicer-scheduler/tests/integration/main.rs` (binary name `scheduler_integration`).
- `crates/slicer-runtime/tests/contract/support_interface_fill_claim_resolution_tdd.rs` — net-new (AC-2 … AC-5), registered as a `mod` in `crates/slicer-runtime/tests/contract/main.rs`.
- `crates/slicer-runtime/Cargo.toml` — dev-dependency on the new module crates only if the contract arms drive them natively; verify before adding.

## Files in Scope (read + edit)

The Code Change Surface list above is authoritative. No file outside it may be edited.

## Read-Only Context

- `modules/core-modules/lightning-infill/**` and `modules/core-modules/gyroid-infill/**` — the single-claim module shape to copy.
- `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR` interface geometry fields (ranged read).
- `crates/slicer-scheduler/src/validation.rs` — `resolve_held_claims`, the empty-return behaviour AC-N1 fixes.
- `crates/slicer-ir/src/resolved_config.rs` — the four existing `*_fill_holder` fields, as the shape `[BLOCK-1]` is measured against.
- `docs/01_system_architecture.md` § Claim System / § Claim Conflict Resolution; `docs/04_host_scheduler.md` § Claim Resolution.

## Out-of-Bounds Files

- `docs/spec_packets/260a-support-interface-spacing-and-loops/**` and every other packet directory.
- `docs/specs/orca-feature-gap/map.md` and `issues/**`.
- `crates/slicer-gcode/src/serialize.rs` — no padding or CONFIG_BLOCK edit is in this packet (Authoring rule 2).
- `crates/slicer-schema/wit/**` — if the design turns out to need a WIT change, that is a new `[BLOCK]`, not an edit.
- `OrcaSlicerDocumented/**` — delegated reads only.

## Expected Sub-Agent Dispatches

- `SUMMARY` (≤200 words): canonical `contact_fill_pattern` branch order and `support_interface_angle()` per-pattern rules, precise enough to implement AC-3 and AC-4.
- `SUMMARY` (≤200 words): `FillConcentric` and the grid variant of `FillRectilinear` — what each emits and with what spacing/angle parameterization.
- `FACT` (≤5 lines): does any existing scheduler mechanism let a second module contribute to `SupportIR` for a region in `Layer::Support` without displacing the `support-generator` holder? Feeds `[BLOCK-2]`.
- `FACT` (≤5 lines): can a `Layer::Support` module obtain the support base angle and the layer index from data it already receives? Feeds `[BLOCK-3]`.

## Locked Assumptions and Invariants

1. `SupportPlanIR` carries per-layer top/base/bottom interface geometry — verified in `crates/slicer-ir/src/slice_ir.rs`.
2. No module or doc currently declares `claim:support-interface-fill` or a `support_interface_pattern` config key — verified at authoring; AC-N3 keeps the second true.
3. The default path (no holder) must remain byte-identical to `260a`'s output — AC-N2.
4. A module may hold exactly one claim: verified by `lightning-infill`'s sparse-only precedent.

## Risks and Tradeoffs

- **The seam is the expensive part, not the fillers.** Two geometry generators are ordinary work; a new claim with a new selector and a new stage interaction touches host contracts. That asymmetry is why this packet is C and why it is split away from `260a` — none of `260a`'s value should wait on this.
- **Silent-drop semantics.** Rule 4's holder-only ruling accepts that an Orca 3MF naming `support_interface_pattern = concentric` is dropped unless the user also sets the holder. Users importing Orca profiles will see rectilinear interface fill and no diagnostic. Accepted by ruling; flagged here because it is the most likely support ticket this packet generates.
- **Renderer/filler double-emission.** If claim suppression is wrong in one direction the interface is filled twice; in the other, not at all. AC-2 and AC-N2 bracket both.

## Context Cost Estimate

L at packet level (new claim + selector + two modules + two test suites). `implementation-plan.md` decomposes it so no single step is L; if the `[BLOCK-1]` resolution makes the selector step L, the packet splits again before activation.

## Open Questions

- **`[BLOCK-1]` — the holder selector does not exist and the obvious shape is out of bounds.** The four existing fill holders are fields on the host `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`), each with a CLI binding. A `support_interface_fill_holder` of that shape is a **new host `ResolvedConfig` field**, which this authoring session is explicitly barred from committing a packet to. The alternative named by rule 4 — `module_overrides` — is documented in `docs/01_system_architecture.md` § Claim System but has **zero Rust occurrences** in the tree, so it is a mechanism to be built, not used. **Ruling needed:** add the `ResolvedConfig` field (and accept the host-config growth), or build `module_overrides` first as its own packet and select through it alone.
- **`[BLOCK-2]` — two modules writing `SupportIR` in one layer stage.** The renderer holds `support-generator` for the region and writes `SupportIR`; a filler holding `claim:support-interface-fill` would write into the same output in the same stage. Whether the scheduler admits that today, and how the renderer learns to skip its own interface fill, is unresolved. **Ruling needed:** does this reduce to existing claim-conflict resolution (`docs/01_system_architecture.md` § Claim Conflict Resolution), or does it need a new scheduler rule — and if the latter, does the rule reach the WIT surface?
- **`[BLOCK-3]` — per-pattern angles may need plan metadata that is not there.** Canonical `support_interface_angle()` needs the support base angle (grid) and the layer parity (interlaced). A `Layer::Support` filler may not be able to derive both from what it receives. If it cannot, carrying them is an **IR schema bump on `SupportPlanIR`**, which this session is barred from committing to. **Ruling needed:** confirm what the filler can already see; if the base angle is not reachable, `grid` needs the bump too, not just `rectilinear_interlaced`, and the packet shrinks to `concentric` alone until the bump is authorized.
- **`[BLOCK-4]` — ADR-0059 conformance is unchecked.** `docs/adr/0059-support-families-and-anchored-entities.md` is the decision record governing the support-family claim vocabulary this packet extends. Adding a claim that lets a non-`support-generator` module write `SupportIR` inside `Layer::Support` plausibly touches its normative content, and Authoring/preflight rule S8 forbids silently contradicting an ADR. **Ruling needed:** confirm the packet conforms to ADR-0059, or author a `D-260b-ADR-0059-AMENDED` deviation quoting the contested clause plus a superseding ADR. This must be settled in Step 0 alongside `[BLOCK-2]`, which is the same question seen from the scheduler side.
- `[FWD]` Should the four existing `*_fill_holder` fields and this one converge on a single holder map rather than one field per role? Out of scope here; recorded because `[BLOCK-1]`'s ruling effectively decides it.
