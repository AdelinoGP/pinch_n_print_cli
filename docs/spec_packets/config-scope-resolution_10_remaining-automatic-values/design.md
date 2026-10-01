# Design: remaining-automatic-values

## Selected Approach

Use a registry-declared host runtime extension key with a typed accessor, not a fixed serialized field. The existing `ResolvedConfig.extensions` map transports the key after normal packet-05 validation, defaults and scope resolution. Add no second resolution pass and no `typed_field_keys` entry. RegionMapIR stays at its pre-change 3.0.0 struct layout; new map content is distinct from a layout addition. This is not a blanket rule for future host keys.

## Controlling Code Paths

- `HOST_RUNTIME_KEYS`, `HostRuntimeKey`, `host_key_denied_scopes` and `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`): add the Float declaration with `Some("0.0")`, `SCOPE_FILAMENT`, `meta.min = Some(0.0)`, and `TOOL_CAPABLE_SCOPES` denial. The new accessor `filament_max_volumetric_speed(&self) -> Result<f64, String>` accepts Float, Int, or a finite numeric String, either as a scalar or as the first element of a non-empty List; absent is zero, while wrong types, empty Lists, non-finite values, and negative values are rejected. This string parsing is accessor-specific, not a general config coercion rule. Resolution retains this scalar envelope; it is not a tool-index array, so the accessor must not index it by tool number.
- `RegionMapIR.configs`, `RegionPlan.paint_overrides`, and `CURRENT_REGION_MAP_IR_SCHEMA_VERSION` (`crates/slicer-ir/src/slice_ir.rs`): restore only packet-added fixed-field/migration changes; retain pre-existing 3.0.0 shape and F-19 chain. `postcard` remains dev-only in `crates/slicer-ir/Cargo.toml`.
- `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`): live move inputs select a private automatic base when role speed is exactly zero, active-tool config wins, absent tool config falls back globally. Shared `feedrate_from_base_mm_per_s` applies normal clamp/conversion; no absolute point-speed input or public emission seam is added.
- `PrepassContext` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`): carry already-resolved tool configs to the model consumer instead of discarding them. Normal runtime and visual emission share resolved sources, not a duplicate scope engine.
- `load_visual_debug_config` and model GCodeEmit construction (`crates/pnp-cli/src/visual_debug.rs`): feed request-derived `FeedrateConfig::from_raw_config` and resolved global/tool configs to the emitter. Inline tests assert emitted F with a default-config control and distinct tool maxima.
- `resolved_config_keys_match_default` (`crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`): retained `[resolved_config]` mirror reads its default through the typed extension accessor. The previously red numeric-field lookup must not be presented as a passed lock.

## Architecture Constraints

- Config-only percentages/mirrors remain packet-04-owned; packet 05 remains the only precedence/resolution source. Emission does not repeat merges or registry assembly.
- Role speed zero triggers automatic speed; filament limit zero means unavailable. Automatic geometry/limit/base and final rounded/narrowed F must be finite positive. Reject non-finite factors before clamping; retain normal finite-factor clamp semantics. Overflow or round-to-zero fails through `GCodeEmitError::Emit`.
- Explicit positive configured speeds remain intentionally uncapped. This packet does not claim an all-speed volumetric ceiling.
- Canonical `GCode.cpp::GCode::_extrude` independently confirms the zero-speed quotient and active `m_writer.filament()` selection, but also caps explicit positive speeds separately. This packet is scope-limited formula/tool parity, not full canonical emitter parity; the user explicitly chose to leave positive speeds uncapped. The read-only source check is not a test PASS.
- Existing extension comparison/hash already participates in interning; test distinct content, config/paint preservation and identity instead of adding handwritten fixed-field equality/hash arms.
- Q8 "Amend ADR" authorizes the genuine narrow architectural mechanism amendment in ADR-0072 (`docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md`), registered as `D-CSR10-ADR-0052-AMENDED` in `docs/DEVIATION_LOG.md`. ADR-0052 §Decision 1's unchanged-body, exact single-function placement and direct-production-call requirements are superseded, not merely clarified. Its factor-valued carriers, public role/factor signature, replacement/fallback, profile length and mutation/application constraints remain; original sections and 2026-08-05 amendment stay textually unchanged.
- Public context-free `resolve_feedrate` delegates role-only base selection and shared conversion; configured zero remains its zero placeholder. Production extrusion uses private `resolve_extrusion_feedrate` with live tool/geometry context for both point and entity factors. One private `feedrate_from_base_mm_per_s` applies `clamp(0.05, 5.0)` and conversion for automatic and explicit bases; its actual f64 multiply rounds to three decimals before f32 narrowing. Producers never send absolute mm/s. These are inspected mechanism facts, not test PASS or blanket byte-identity claims.
- WIT, stage packages, CLI JSON, visual schemas/manifest and persisted struct layout are unchanged.
- Coordinate unit is 100 nm; use established mm/unit adapters in fixtures.

## Artifact and Dependency Policy

Keep the independent pre-change fixture under `crates/slicer-ir/tests/fixtures/region_map_v3_0_0/` byte-for-byte, including provenance and expected values. Repurpose `region_map_versioned_decode_tdd.rs` for direct dev-Postcard deserialize and current extension round-trips across configs/paint; its historical filename does not promise a version dispatcher. Abandon the unshipped experimental 3.1.0 layout and decoder without deleting any existing experimental artifacts or promising compatibility. This is a non-deletion constraint, not a requirement to create an experimental fixture or an assertion that a 3.1.0 artifact exists. `review-remediation.md` identifies the extant files, baseline/end hashes and bounded search result; unidentified files are never described as proven present, absent or deleted. Do not create or regenerate fixtures, read large JSON fixtures directly, or rely on experimental artifacts as an oracle.

Retire the packet-added production Postcard dependency. Root/guest locks are regenerated only through normal Cargo operations; inspect resulting diffs rather than assuming previous dependency-promotion diffs remain needed. Do not hand-edit locks or discard unrelated churn. Guest freshness is mandatory: check exact exit 0/1/3, rebuild stale guests, and use gated sequential guest-touching tests. Ordinary `cargo build/test` does not rebuild guests.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` — declaration/accessor, remove only experimental fixed field/fallout, inline accessor/scope controls.
- `crates/slicer-ir/src/slice_ir.rs` — retire only packet-added decoder/legacy shapes and restore 3.0.0 layout/version chain.
- `crates/slicer-ir/Cargo.toml` — retire packet-added production Postcard dependency, retain dev dependency.
- `crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs` — preserved fixture plus current extension/paint identity and accessor cases.
- `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs` — extension accessor consumption, real emission and final-F failure controls.
- `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`, `crates/slicer-config/tests/scope_eligibility_tdd.rs`, `crates/slicer-config/tests/registry_census_tdd.rs` — census, legitimate registry-roster follow-through, real declared-extension default/precedence/validation, and exact-denial coverage; preserve source-derived discovery and assertions, and do not rewrite packet-05 resolution behavior.
- `crates/slicer-runtime/src/run.rs` — `PrepassContext` and resolved-tool return/handoff; approved runtime expansion, not read-only.
- `crates/pnp-cli/src/visual_debug.rs` — model config/tool handoff and inline actual-F tests only.
- `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/{visual-debug.json,visual-debug-config.json}` — request/config pair with a distinct tool-0 override to falsify a dropped runtime tool map; inline multiple-tool emission asserts separate literal feedrates.
- `crates/pnp-cli/tests/visual_debug_volumetric_auto_tdd.rs` — introduce a direct Cargo test target that compares actual model CLI captures with and without the tool-0 override; the shared `visual_debug_volumetric_auto` filter must run it, not just helper tests.
- `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — accessor-based retained mirror lock.
- `docs/config/host-keys.toml`, generated `docs/15_config_keys_reference.md` — mirror retained, regenerate normally.
- `CONTEXT.md`, `docs/02_ir_schemas.md`, `docs/adr/0052-per-point-speed-factor-contract.md` (packet append only), TASK-571 evidence in `docs/07_implementation_status.md`, and this packet's five core files — revised domain/contract/evidence only.
- `docs/spec_packets/config-scope-resolution_10_remaining-automatic-values/review-remediation.md` — coordinator-owned finding disposition, identified artifact inventory and fresh verification receipts.
- `docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md` — separate accepted narrow mechanism decision; `docs/DEVIATION_LOG.md` — only the `D-CSR10-ADR-0052-AMENDED` row. Q8 explicitly expands documentation scope to these paths; no other ADR or registry-row edits.
- Root `Cargo.lock` and guest Cargo-generated locks only when normal resolution changes them; audit each diff and validate convergence. These are generated outputs, never hand edits or a blanket entitlement to unrelated changes.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md`, packet 04/05 exports/status and all other packet directories.
- `docs/19_visual_debug.md`, `docs/22_test_quality.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/11_operational_governance_and_acceptance_gate.md` — relevant bounded sections.
- `crates/slicer-runtime/src/pipeline.rs::dump_prepass_ir_if_requested`, `crates/slicer-ir/src/feedrate.rs::FeedrateConfig`, `crates/slicer-gcode/src/error.rs::GCodeEmitError` — relevant symbols only.
- Independent pre-change fixture bytes/provenance/expected values and experimental on-disk artifacts — immutable inputs/history.

## Out-of-Bounds Files

Approved plan and packet dirs other than this one; WIT packages, UI/renderers, visual schema/manifest vocabulary; packet-04 expansion and packet-05 resolution implementations; unrelated source/docs/fixtures; prior ADR-0052 sections and all other ADRs except the exact new ADR-0072 path above. Deviation registry edits beyond the one named row are out of bounds. Never load canonical Orca directly, large fixture bodies, target/generated code or vendored dependencies. Experimental artifact deletion, fixture regeneration, branch/stash/reset/revert and commits are prohibited.

## Rejected Alternatives

- Phase-B zero-speed expansion or geometric `FeedrateConfig` adapter: missing live move context.
- Fixed serialized field plus 3.1.0 migration: existing extension transport suffices; unshipped experiment need not become a compatibility commitment.
- Public absolute-mm/s point-speed seam: contradicts factor contract; private automatic base does not need it.
- Conforming to ADR-0052's unchanged body/direct production call by simulating move context in configuration or cloning emitters: hides geometry-owned context; abandoning automatic geometry speed drops the chosen feature. Q8 instead accepts the bounded private delegation mechanism in ADR-0072.
- Re-resolving visual tools or taking a default-only map: duplicates precedence or loses tool-specific behavior.
- Manifest-only visual proof or a complete hand-authored census roster: false-green coverage.

## Expected Sub-Agent Dispatches

1. Reconcile dependencies and derived census; exact exports/status and source-derived owners; FACT or bounded LOCATIONS with symbol/path.
2. Verify canonical `GCode.cpp::GCode::_extrude` formula/tool/guards; SUMMARY at most 200 words.
3. Bounded workers by implementation step, each at most three edited files; fresh test/gate FACT with non-empty markers and failure snippets only. Return results outside thinking blocks.
4. Fresh preflight S0–S8 before activation and full review after all gates; no previous approval reused.

## Risks and Open Facts

Direct 3.0.0 fixture decode must prove actual shape restoration; extension round-trip alone is insufficient. Valid division can still produce unsafe final F, so test non-finite factors, overflow and round-to-zero independently. Visual tools must be the already-resolved map, not raw config reconstruction. The explicit-speed regression must use a positive volumetric ceiling below its configured speed. Current named receipts must cover fixture decode, roundtrip and interning; an older two-test receipt is not evidence for the current test file. Aggregate/largest context cost M; no L step.

Original preflight was `PREFLIGHT BLOCKED` on S8. Q8 remains resolved by ADR-0072/`D-CSR10-ADR-0052-AMENDED`; the later cold review reopened the packet with `CHANGES REQUESTED`. Remediation closure is supported by the fresh validation and independent full review recorded in `review-remediation.md`, not inferred from architectural acceptance.
