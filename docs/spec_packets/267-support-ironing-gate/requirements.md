# Requirements: 267-support-ironing-gate

## Packet Metadata

- Grouped task IDs: none - queue packet; implementation is recorded against [22 - Author packet P15 - Support / Support ironing - support-surface-ironing](../specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md).
- Backlog source: `docs/specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md` (P15 in the wayfinder map "Close the OrcaSlicer FFF feature gap").
- Packet number: allocate one directory prefix from disk at authoring time using the procedure settled by [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md); never reserve a block.
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

P15 was scoped as two canonical Support / Support-ironing keys: `support_ironing` and `support_ironing_pattern`. Authoring re-derived both against canonical and against this tree, and the two keys land in different places.

`support_ironing` is a `coBool` (default `false`) that canonical assigns to `SupportParameters::ironing`, which gates the ironing branch of `generate_support_toolpaths`. This port has the behaviour but not the key: `SupportSurfaceIroning::from_config` gates on a PnP-invented `ironing_enabled` bool, declared identically — and independently — by both `support-surface-ironing` and `top-surface-ironing`. One consequence is a reachability bug, not just a naming one: an OrcaSlicer configuration setting `support_ironing = 1` cannot enable support ironing in this port at all, and a user who sets `ironing_enabled` to reach top-surface ironing silently enables support ironing too. The map's ticket-07 ruling (standardise to Orca's names) and the 2026-09-01 grilling ruling **Q10(b)** (`ironing_enabled` retires into `ironing_type` plus `support_ironing`; two gates for one decision is what Authoring rule 5 forbids) both point at the same change. This packet makes `support_ironing` the module's sole gate.

`support_ironing_pattern` is a `coEnum` over `InfillPattern` whose two values, `rectilinear` and `concentric`, select the filler instance canonical constructs with `Fill::new_from_type(support_params.ironing_pattern)`. That is cross-module algorithm selection in this port's architecture, so map Authoring rule 4 and grilling ruling **Q3(a)** apply verbatim: it is never declared as an input key, it is selected through claim holders, and this port ships neither a support-ironing claim seam nor a concentric filler. Under Authoring rule 1 the key is therefore **left out of this packet and returned to the queue as unimplemented**, not declared with a gap. P15 covers **one key, not two**.

## In Scope

- Replace `[config.schema.ironing_enabled]` with `[config.schema.support_ironing]` in `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`: `type = "bool"`, `default = false`, `display = "Ironing Support Interface"` (canonical label), `group = "Support"`. Canonical type and default are matched exactly; a bool carries no `min`/`max`.
- Change the single gate read in `SupportSurfaceIroning::from_config` (`modules/core-modules/support-surface-ironing/src/lib.rs`) from `config.get("ironing_enabled")` to `config.get("support_ironing")`, keeping the existing absent-means-`false` behaviour. The stored field name follows the key.
- Add a TOML-direct-parse schema guard at `modules/core-modules/support-surface-ironing/tests/support_ironing_config_schema_tdd.rs`, and add `toml = "0.8"` to the module's dev-dependencies if absent. The module declares no explicit `[[test]]` entries, so the file is an auto-discovered test binary. The filename is net-new and collides with no guard planned by packets 253, 260a/260b, 263, 264, 265, or 266.
- Migrate the module's own suites to the canonical key: `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` (the `enabled_config` helper and every explicit `("ironing_enabled", …)` entry) and `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs`. Add the AC-2 off/absent arms and the AC-N1 legacy-key regression.
- Migrate the support-owned integrated-parity contract test `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` to `support_ironing`, so AC-3 proves reachability through the real host config path.
- Regenerate `docs/15_config_keys_reference.md` through `cargo xtask gen-config-docs` and verify with `--check`.
- Rebuild the guest artifact for this module and re-verify with `cargo xtask build-guests --check` (exit code, never a `STALE:` grep). Guest WASMs embed config key names — proven by byte-search in ticket 101 — so a gate-key change is a guest input.

## Out of Scope

- `modules/core-modules/top-surface-ironing/**`, including its own `ironing_enabled` declaration and read. That gate becomes `ironing_type` under P14 / packet `266-top-surface-ironing-keys`. The two modules receive separately filtered `ConfigView`s, so there is no shared declaration and no ordering dependency between the packets.
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`, and `resources/test_config/benchy_combined_feature_evidence.json`. Verified this session: every `ironing_enabled` occurrence in those four files is exercising **top-surface** ironing (the e2e asserts `;TYPE:Ironing` over a staircase's top steps). They are packet 266's migration surface, not this packet's.
- `support_ironing_pattern` — see "Returned to Queue" below.
- `ironing_speed` in the support manifest. The grilling ruling **Q11(a)** renames it to `support_ironing_speed` (keeping `30.0`) and flags a membership decision for the feedrate key table — which is `SPEED_KEYS` in `crates/slicer-ir/src/feedrate.rs`, not the `FEEDRATE_KEYS` the ruling names; that is a PnP naming fix with no canonical counterpart key, it is not a P15 queue key, and it needs its own adjudication. Left untouched here and filed as a follow-up on the map.
- `support_ironing_flow` and `support_ironing_spacing`. Already canonical-named and live as of ticket 106; unchanged field for field, and AC-1 pins that.
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) and every CONFIG_BLOCK twin, in both directions: not read, not edited, not asserted. Map Authoring rule 2 — the padding table is not parity evidence and is never a deliverable. Re-derived this session: neither `support_ironing` nor `ironing_enabled` appears in that table today, so there is nothing there to be tempted by.
- New IR/WIT fields, an IR schema bump, or a host `ResolvedConfig` field. The key is module-owned and reaches the module through the manifest-filtered `ConfigView`.
- Changing **which** surfaces support ironing covers. That divergence is real and is recorded (`DIV-267-A` in `design.md`), not silently absorbed and not wired here.
- Hand edits to `docs/15_config_keys_reference.md` or `docs/ORCA_CONFIG_REFERENCE.md`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` - delegated SUMMARY of manifest config-schema types and per-module view filtering.
- `docs/15_config_keys_reference.md` - generated; targeted checks only, never a source file.
- `docs/ORCASLICER_ATTRIBUTION.md` - porting-header contract; no new translated file is added by this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, <= 20 entries) or `SUMMARY` (<= 200 words, no code unless asked). Code snippets in returns are capped at 30 lines. The checkout is the sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, not in-tree.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` / `PrintConfig.hpp` - the `support_ironing` and `support_ironing_pattern` declarations, types, defaults, and enum value lists.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - the `SupportParameters` constructor's assignment of `ironing`, `ironing_flow`, `ironing_spacing`, and `ironing_pattern`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - `generate_support_toolpaths`: the ironing capture gate and the ironing fill block.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests. Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence

| Key | Canonical type | Canonical default | Bounds / values | Canonical consumer | Current PnP state | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `support_ironing` | `coBool` | `false` | none | `SupportParameters`' constructor (`Support/SupportParameters.hpp`) assigns it to `SupportParameters::ironing`; `generate_support_toolpaths` (`Support/SupportCommon.cpp`) gates the capture of the top contact layer's polygons on `support_params.ironing && !top_contact_layer.empty()`, and a later block fills those polygons at `ExtrusionRole::erIroning` with `ironing_flow` / `ironing_spacing` / `ironing_pattern` | zero occurrences in the tree; the module gates on the PnP `ironing_enabled` bool instead, which `top-surface-ironing` also declares | **Wired.** Replace the gate key in the manifest and in `from_config`; behaviour at the default (`false`) is byte-identical, and `true` enables the existing ironing emission |
| `support_ironing_pattern` | `coEnum` over `InfillPattern` | `ipRectilinear` | `rectilinear`, `concentric` | `SupportParameters`' constructor assigns it to `ironing_pattern`; `generate_support_toolpaths` constructs the filler with `Fill::new_from_type(support_params.ironing_pattern)` | zero occurrences in the tree | **Returned to the queue as unimplemented** — algorithm-selecting enum, holder-only under map Authoring rule 4 / grilling ruling Q3(a); see below |

Canonical gating nuance, recorded not wired: canonical reaches the ironing branch only inside the `top_interfaces` arm of `generate_support_toolpaths` — i.e. only when top interface layers were requested — and irons exactly the support **top contact layer**'s polygons. This port's module runs at `Layer::SupportPostProcess` and receives only `&[SliceRegionView]`, so it has no handle on support interface geometry at all. That is `DIV-267-A` in `design.md`: bounded, named, and unchanged by this packet, which moves the gate key without moving the gate's subject.

## Returned to Queue — unimplemented

- **`support_ironing_pattern`.** Canonical selects a filler implementation with `Fill::new_from_type(support_params.ironing_pattern)` over the two values `rectilinear` and `concentric`. Under map Authoring rule 4 (and the grilling's Q3(a) ruling, which lists the sibling `ironing_pattern` explicitly) an Orca enum whose values are different algorithms is never declared as an input key: the values become `claim:*` holders resolved through the holder keys and `module_overrides`. This port has no support-ironing claim (`[claims] holds = []` in `support-surface-ironing.toml`), no holder key for it, and no concentric filler — standing that seam up is new-module work at a new seam (Tier C), not the Tier-A plumbing P15 was scoped as. The key is therefore **not** declared here, not declared with a gap, and not counted as covered. It returns to the queue as unimplemented, with the missing feature named as "support-ironing filler selection through a claim seam, shipping at least the canonical default `rectilinear` as a holder". AC-N2 pins its honest absence from the tree.

## Ruled Dead-in-Canonical

**None.** Both keys are read inside canonical's slicing pipeline (`Support/SupportParameters.hpp` and `Support/SupportCommon.cpp`, not `ConfigManipulation.cpp`, not a GUI tooltip, not an `IGNORE`/legacy-alias set). `support_ironing_pattern` leaves this packet under rule 4, which is a mechanism ruling, not a dead-key ruling — it stays in the queue's scope count.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` exact manifest schema and the removal of the legacy table; `AC-2` gate behaviour at `true` / `false` / absent; `AC-3` reachability through the real host config path under integrated parity; `AC-4` generated documentation and no new deviation row.
- **Map gate (a) coverage.** The disposition table lists **zero** declaration-only keys. One key is wired; the other is returned to the queue, not declared.
- **Map gate (b) coverage.** `support_ironing` has an AC asserting a behaviour change at a non-default value: AC-2's `true` arm emits ironing paths where the default `false` arm and the absent-key arm emit none. Its evidence is not default-path identity, and no AC asserts a CONFIG_BLOCK line.
- Negative: `AC-N1` the legacy bool is not a fallback gate; `AC-N2` no `support_ironing_pattern` declaration exists anywhere.
- Cross-packet impact: packet `266-top-surface-ironing-keys` owns the top module's `ironing_enabled` retirement and its four fixtures; this packet owns the support module's. Only after both land is `ironing_enabled` gone from the tree, and neither may assert that alone. No other queued packet claims either P15 key.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p support-surface-ironing --test support_ironing_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 manifest schema guard | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p support-surface-ironing --test ironing_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2 gate behaviour, AC-N1 legacy regression | FACT pass/fail |
| `cargo test -p support-surface-ironing --test ironing_scanline_parity_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | scan-line parity suite survives the key migration | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract integrated_parity_support_surface_ironing 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-3 host-path reachability, native/wasm parity | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after manifest/source edits | FACT exit code; 0 fresh, 1 stale, 3 wasm-tools missing |
| `cargo xtask gen-config-docs` | regenerate the generated reference | FACT exit code |
| `cargo xtask gen-config-docs --check` | AC-4 generated-reference check and deviation gate | FACT exit code |
| `rg -n 'support_ironing\b' docs/15_config_keys_reference.md` | AC-4 row presence under the support owner | LOCATIONS <=10 |
| `rg -n 'ironing_enabled' modules/core-modules/support-surface-ironing crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` | AC-1/AC-N1 support-side absence (top-side occurrences are packet 266's) | LOCATIONS; expect none |
| `cargo xtask check-literals` | struct-literal churn gate | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation; every test run writes the required `target/test-output.log`.

## Step Completion Expectations

- The schema guard and the manifest land together; the guard asserts both the new table and the removal of the legacy one.
- The module source and its own suites land together, so the gate has an invariant at `true`, `false`, and absent before the contract test is migrated.
- The support-owned contract test is migrated only after the module gate is live, and it is the packet's proof that the key crosses the real host boundary.
- Guests are rebuilt after the final manifest/source edit and re-checked by exit code; the generated reference is regenerated last.
- No step touches a `top-surface-ironing` file, a top-owned fixture, or `crates/slicer-gcode/src/serialize.rs`.

## Context Discipline Notes

- `modules/core-modules/support-surface-ironing/src/lib.rs` is short; only `from_config`, the struct fields, and `run_support_postprocess`'s gate are needed. The scan-line generator is not changed by this packet.
- `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` is read by targeted search for the config-map construction only.
- `docs/15_config_keys_reference.md` is read by targeted `rg` only; it is generated.
- Canonical files remain delegated and all cargo commands remain delegated; retain only FACT/LOCATIONS/SNIPPETS returns.
