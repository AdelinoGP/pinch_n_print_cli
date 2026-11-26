# Requirements: support-interface-fill-claim-holders

## Packet Metadata

- Packet: `260b-support-interface-fill-claim-holders`
- Status: `draft` — **not activatable**; four open `[BLOCK]`s in `design.md` §Open Questions.
- Tier: **C** (the packet builds a decision point *and* a new claim seam)
- Depends on: `260a-support-interface-spacing-and-loops`
- Backlog source: wayfinder ticket 18, map `docs/specs/orca-feature-gap/map.md`
- Split: this packet and `260a` replace the pre-rules packet `260-support-interface-keys`.

## Problem Statement

`support_interface_pattern` selects among *different fill algorithms* for support interface surfaces. Authoring rule 4 is explicit about this shape: an Orca enum whose values are different algorithms is not an enum to declare on one module and mark with-gap — it is a set of `claim:*` holders, one per shipped value, resolved through the claim seam. Rule 4's holder-only ruling goes further: the Orca enum is never declared as an input key at all, not even as a host-side alias mapping its string onto a holder name.

The pre-rules packet 260 declared the key with-gap. Under rule 1 that disposition is prohibited, so the key was removed from `260a` and returned to the queue; this packet is the queue row that makes it real.

The obstacle is that the seam rule 4 points at does not exist for support interface fill, and four parts of it are unresolved at once — recorded as `[BLOCK]`s rather than papered over.

## In Scope

- A new claim ID `claim:support-interface-fill`, registered in `docs/03_wit_and_manifest.md` § Known claim IDs and recognized by the scheduler, held by interface-filler modules and by nothing else.
- At least two net-new filler modules — concentric and grid — each holding only that claim, running at `Layer::Support`, reading `SupportPlanIR` interface geometry and writing `SupportIR`.
- Canonical's `auto` resolution as the branch order from `SupportParameters::SupportParameters` (AC-3), and canonical's per-pattern interface angles from `support_interface_angle()` (AC-4).
- Per-region selection through the existing region-override mechanism (AC-5) — a capability canonical does not have, recorded as a divergence.
- Startup validation failure for a holder naming no loaded module (AC-N1).
- Default-path byte-identity when no holder is configured (AC-N2).

## Out of Scope

- Every interface *spacing* key and the contact-loop pass — `260a`.
- Declaring `support_interface_pattern` as a config key anywhere; AC-N3 lints the whole module tree against it.
- `ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin (Authoring rule 2).
- Reworking the four existing `*_fill_holder` selectors or the fill-role claims (`claim:top-fill` … `claim:sparse-fill`); this packet adds a sibling, it does not refactor them.

## Returned to Queue — unimplemented, needs a canonical read to size

- **`rectilinear_interlaced`** may not ship in this packet. Canonical `support_interface_angle()` alternates its angle by layer parity, which needs the layer index and the support base angle at the filler — see `[BLOCK-3]`. If the implementer's canonical dispatch confirms the filler can derive both from data it already receives, the value ships here; otherwise it is returned to the queue as *unimplemented, needs layer-parity angle metadata at the interface filler*, and no module claims it. **It is never declared as a value the port recognises but does not implement** (rule 4, holder-only: unshipped values are unimplemented, not declared).

## Ruled Dead-in-Canonical

None. `support_interface_pattern` has read sites inside `libslic3r/` (`Support/SupportParameters.hpp` `SupportParameters::SupportParameters` and `support_interface_angle()`; `Support/SupportCommon.cpp` `generate_support_toolpaths`), so Authoring rule 3 does not rule it out.

## Per-Key Canonical Evidence

| Key | Canonical type | Canonical default | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- |
| `support_interface_pattern` | coEnum `SupportMaterialInterfacePattern` (`auto`, `rectilinear`, `concentric`, `rectilinear_interlaced`, `grid`) | `auto` | **none — holder-only** (rule 4: the enum is never an input key; selection is by claim holder / region override) | `SupportParameters::SupportParameters` — `contact_fill_pattern` branch order: `smipGrid`→`ipGrid`; `smipRectilinearInterlaced`→`ipRectilinear`; (`smipAuto` ∧ zero interface gap) ∨ `smipConcentric`→`ipConcentric`; contact interface density > 0.95→`ipRectilinear`; else `ipSupportBase`. Plus `support_interface_angle()` for the per-pattern angle, and `SupportCommon.cpp` `generate_support_toolpaths` for filler construction (`Fill::new_from_type`) | **(b) built by this packet** — as `claim:support-interface-fill` holders. AC-2 asserts the behaviour change at a non-default holder selection; AC-3/AC-4 pin the branch order and angles |

Rule 6b check: the packet's single key is asserted at a non-default value by AC-2 (a non-default holder selection producing different interface geometry), and no AC's only evidence is default-path identity — AC-N2's default-path identity arm is *additional*, never the sole evidence.

## In-Tree Grounding (verified at authoring, 2026-09-01)

1. **No `claim:support-interface-fill` exists.** `docs/03_wit_and_manifest.md` § Known claim IDs lists no interface-fill claim; the support-family vocabulary is `support-generator`, `support-planner`, and `support-family:<id>`.
2. **No interface-fill holder selector exists.** The four fill holders (`top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`, `sparse_fill_holder`) are fields on the host `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`), each with a CLI binding and a default of `"rectilinear-infill"`. A support-interface holder of the same shape is a **new host `ResolvedConfig` field**.
3. **`module_overrides` is documented but not implemented.** It appears in `docs/01_system_architecture.md` § Claim System as the per-region override form, and in packet 263's design as the per-region selector, but a tree-wide search finds **zero Rust occurrences** of the identifier. Per-region selection (AC-5) therefore depends on that mechanism being built.
4. **No concentric or grid generator exists as a module.** `modules/core-modules/` ships arachne/classic perimeters, gyroid, lightning, rectilinear, and the wave/infill-linker modules; there is no concentric or grid filler, and the SDK exposes no fill primitive (`crates/slicer-sdk/src/host.rs` offers clipping, offsetting, simplification, medial axis, and Arachne walls — no filler).
5. **Interface geometry is already in IR.** `SupportPlanIR`'s per-layer entries carry top-, base-, and bottom-interface geometry (`crates/slicer-ir/src/slice_ir.rs`), so a filler module can reach the islands without an IR change. What is *not* obviously present is the support base angle and the layer index in a form the filler can use for canonical's angle rules — `[BLOCK-3]`.
6. **Unmatched holders fail silently today.** Authoring rule 4's holder-only ruling records that `resolve_held_claims` (`crates/slicer-scheduler/src/validation.rs`) returns empty for every module and no `SchedulerError` variant covers a holder naming no module. AC-N1 requires that to be fixed here or inherited from whichever packet fixes it first.

## Acceptance Summary

| AC | Assertion | Non-default value asserted |
| --- | --- | --- |
| AC-1 | Filler manifests hold only `claim:support-interface-fill`, at `Layer::Support`, writing `SupportIR` | — (claim contract) |
| AC-2 | Concentric / grid holder selection changes the emitted interface geometry | non-default holder selection |
| AC-3 | Canonical `auto` branch order reproduced | zero-gap and >0.95-density arms |
| AC-4 | Canonical per-pattern interface angles reproduced | grid = base angle; snug rectilinear = interface angle − 45° |
| AC-5 | Region override changes only that region | per-region holder differing from the print holder |
| AC-N1 | Unmatched holder fails startup validation | bogus holder name |
| AC-N2 | No holder configured → default output unchanged | — (additional identity arm) |
| AC-N3 | No manifest declares `support_interface_pattern` | — (holder-only lint) |

## Verification Commands

| Command | Covers | Return format |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-3, AC-4, AC-5 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration support_interface_fill_claim_resolution_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1, AC-N1 | FACT pass/fail |
| `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2 | FACT pass/fail |
| `rg -l 'support_interface_pattern' modules/core-modules/*/[a-z-]*.toml; test $? -ne 0; echo "exit=$?"` | AC-N3 | FACT exit code |
| `rg -q 'claim:support-interface-fill' docs/03_wit_and_manifest.md; echo "exit=$?"` | doc impact | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (net-new modules) | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo xtask check-literals` | gates | FACT pass/fail |

## Step Completion Expectations

- No step may ship a filler module without the holder that selects it and a test that selects it — a module nothing can reach is a declaration-only key in module form.
- The step that adds the new claim ID owns the `docs/03_wit_and_manifest.md` registration in the same step; an unregistered claim is not a claim.
- If `[BLOCK-3]` resolves against shipping `rectilinear_interlaced`, the step that would have shipped it instead moves the value to §Returned to Queue and ships nothing for it.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters` (`contact_fill_pattern` branch order) and `support_interface_angle()` (per-pattern angles).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (filler construction and the parameters the filler receives).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `SupportMaterialInterfacePattern` members.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillConcentric.cpp`, `Fill/FillRectilinear.cpp` — the concentric and grid generators.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

## Context Discipline Notes

Read budget 120k. Delegate every cargo run and every `OrcaSlicerDocumented/` read. Do not open the two support renderer `src/lib.rs` files in full — the only part this packet needs is where `260a` left the interface emission seam.
