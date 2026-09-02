---
status: draft
packet: 267-support-ironing-gate
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: S
---

# Packet Contract: 267-support-ironing-gate

## Goal

Replace `support-surface-ironing`'s non-canonical `ironing_enabled` gate with canonical `support_ironing`, so support-interface ironing has its own switch, is reachable from an OrcaSlicer configuration, and stops sharing one bool with top-surface ironing.

## Scope Boundaries

This packet changes the `support-surface-ironing` manifest, its `from_config` gate, its own tests, the support-owned integrated-parity contract test, and the generated config reference. It does **not** touch `modules/core-modules/top-surface-ironing/**` — the top module's gate becomes `ironing_type` under [21 - Author packet P14 - Quality / Ironing - top-surface-ironing](../specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md) / packet `266-top-surface-ironing-keys`, and the two manifests are independently filtered `ConfigView`s with no shared declaration. It does **not** touch `ORCA_CONFIG_PADDING` or any CONFIG_BLOCK twin (map Authoring rule 2). It ships **no** `support_ironing_pattern` declaration: that key is an algorithm-selecting enum and is returned to the queue as unimplemented under map Authoring rule 4 (see `requirements.md` section "Returned to Queue"). No IR, WIT, or host `ResolvedConfig` change.

## Prerequisites and Blockers

- Depends on [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md), [07 - Document the Orca to Pinch alias map and retire the hand-maintained column](../specs/orca-feature-gap/issues/07-alias-map-and-column-retirement.md), and [106 - Rename ironing keys to Orca names](../specs/orca-feature-gap/issues/106-rename-ironing-keys.md); all are resolved map decisions.
- Sequencing with packet `266-top-surface-ironing-keys`: disjoint file sets, no merge conflict, either order is safe. Only after **both** land does `ironing_enabled` disappear from the tree, so neither packet may assert its tree-wide absence on its own.
- Activation blockers: none for the draft packet; activation remains a separate explicit `/swarm` decision.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`, **when** its `[config.schema]` is parsed directly with `toml`, **then** it contains `[config.schema.support_ironing]` with `type = "bool"` and `default = false` (canonical `coBool`, default `false`), carrying a `display` and `group = "Support"`; it contains **no** `[config.schema.ironing_enabled]` table; and the four sibling tables (`ironing_speed`, `support_ironing_flow`, `support_ironing_spacing`, `line_width`) are unchanged field for field. | `cargo test -p support-surface-ironing --test support_ironing_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** a `SliceRegionView` fixture with a square region, **when** `SupportSurfaceIroning::from_config` receives `support_ironing = true` and `run_support_postprocess` runs, **then** at least one `ExtrusionPath3D` with `ExtrusionRole::Ironing` is pushed; **when** it receives `support_ironing = false`, and separately when the key is absent entirely, **then** zero paths are pushed. The `true` arm is the non-default value and its output differs from the default arm. | `cargo test -p support-surface-ironing --test ironing_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** the real `support-surface-ironing` manifest and guest artifact, **when** the integrated-parity harness runs the `Layer::SupportPostProcess` stage with a host config map carrying `support_ironing = true`, **then** the native and wasm legs agree and both emit ironing paths — proving the key reaches the module through the real host config path, not only through a hand-built `ConfigView`. | `cargo test -p slicer-runtime --test contract integrated_parity_support_surface_ironing 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** the manifests are final, **when** `cargo xtask gen-config-docs` regenerates `docs/15_config_keys_reference.md`, **then** the generated module table carries a `support_ironing` row owned by `support-surface-ironing`, carries no `ironing_enabled` row owned by `support-surface-ironing`, and the deviation section gains no row (canonical default `false` equals the declared default, compared under the boolean comparison ticket 100 enabled). | `cargo xtask gen-config-docs --check`

## Negative Test Cases

- **AC-N1. Given** a support module config containing legacy `ironing_enabled = true` but no `support_ironing`, **when** `SupportSurfaceIroning::from_config` and `run_support_postprocess` are invoked, **then** the module stays off and emits no path — the legacy bool is not a fallback and is not a second gate. | `cargo test -p support-surface-ironing --test ironing_tdd legacy_ironing_enabled_is_not_a_support_gate 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** the source tree, **when** `support_ironing_pattern` is searched for in `.rs`, `.toml`, and `.wit` sources, **then** it does not appear. This packet declares no pattern key and its absence is deliberate (map Authoring rule 4 — returned to the queue as unimplemented, never declared with a gap). | `rg --files-with-matches --glob '!target' --glob '*.rs' --glob '*.toml' --glob '*.wit' support_ironing_pattern . ; test $? -eq 1`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p support-surface-ironing 2>&1 | tee target/test-output.log | grep -E '^test result'` and `cargo xtask build-guests --check; echo "exit=$?"`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` - delegated summary of the `[config.schema]` bool form and per-module `ConfigView` filtering.
- `docs/15_config_keys_reference.md` - generated output; regenerated and checked, never hand-edited.
- `docs/ORCASLICER_ATTRIBUTION.md` - standard porting header, required only if a new Rust file carrying translated canonical logic is added (this packet adds none).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` generated module-key table - regenerated by `cargo xtask gen-config-docs` and checked with `--check`; the AC-4 row assertions are made inside the schema/doc verification step against the regenerated file. No hand edit is allowed. `docs/ORCA_CONFIG_REFERENCE.md` is the hand-maintained upstream snapshot and is untouched.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, <= 20 entries) or `SUMMARY` (<= 200 words, no code unless asked). Code snippets in returns are capped at 30 lines. The checkout is the sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, not in-tree.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` - the `support_ironing` declaration (`coBool`, default `false`) and its `PrintObjectConfig` membership.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - the `SupportParameters` constructor's `this->ironing = object_config.support_ironing` assignment.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - `generate_support_toolpaths`: the `support_params.ironing && !top_contact_layer.empty()` gate that captures the polygons to iron, and the later block that fills them at `ExtrusionRole::erIroning`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list - those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
