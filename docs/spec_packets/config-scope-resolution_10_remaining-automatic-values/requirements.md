# Requirements: remaining-automatic-values

## Packet Metadata

- Task: `TASK-571`; backlog: `docs/07_implementation_status.md`.
- Status: `implemented` (remediation closed after fresh full review `APPROVED WITH NOTES` and final checks); aggregate context cost: `M`.
- Re-scope authority: current user conversation 2026-09-30, recorded in `packet.spec.md`; previous approval/gate claims are superseded/unverified.

## Problem Statement

Config-only resolution cannot determine a zero extrusion-role speed from live width, effective layer height, flow and emitting tool. Phase C owns that derivation. The filament maximum can travel in the existing declared-extension map; adding a fixed field and migration is unnecessary for this selected approach. Model visual-debug must receive the same resolved tool inputs as normal emission.

## In Scope

- Declare `filament_max_volumetric_speed` in `HOST_RUNTIME_KEYS` (`crates/slicer-ir/src/resolved_config.rs`) as Float, `default = Some("0.0")`, `SCOPE_FILAMENT`, `meta.min = Some(0.0)`, with `TOOL_CAPABLE_SCOPES` denial policy. Carry it in existing `extensions` after normal registry validation/defaulting/scope resolution; do not add a macro field or `typed_field_keys` entry.
- Add `ResolvedConfig::filament_max_volumetric_speed(&self) -> Result<f64, String>` in that file: absent returns numeric zero; Float, Int, or a finite numeric String is accepted as a scalar or as the first element of a non-empty List; wrong types, empty Lists, non-finite values, and negative values are rejected. Numeric-string parsing is specific to this accessor, not a general config coercion rule. Resolution retains the original scalar List envelope, not a tool-index array. Zero means unavailable, not a request to derive the limit.
- Select already-resolved active-tool config, or global only if the tool entry is absent. No duplicate merge, precedence or automatic-value resolver.
- At the real move site in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`), role base exactly zero selects `limit / (width × height_delta × flow_factor)` in mm/s. Validate finite positive geometry, volume per mm and limit; private automatic base feeds shared normal factor clamp and F conversion.
- Automatic branch rejects non-finite factors, arithmetic/conversion/narrowing overflow, and final rounded/narrowed non-positive or non-finite F. Explicit positive configured speeds intentionally remain uncapped. Preserve finite-factor behavior under ADR-0052 as narrowly amended by ADR-0072: public context-free role/factor resolver keeps its signature and zero placeholder; production uses private move-context selection and one shared host clamp/conversion policy for point/entity and auto/explicit bases. Producers retain factors, never absolute speeds.
- Derive sentinel candidates from joined live host/module declarations, scalar/array defaults and lower bounds. Test an array-negative control, metadata minimum, and unknown-owner failure; retain packet-04 ownership of both known mirrors.
- Restore the pre-change persisted RegionMapIR 3.0.0 shape and existing version chain. Retire only packet-added fixed-field fallout, legacy structs, public version dispatcher and production Postcard promotion. Keep independent pre-change fixture bytes/provenance/expected values unchanged; direct dev-Postcard tests assert preserved contents and current extension config/paint round-trip and identity.
- Preserve any existing experimental artifact files on disk: no deletion or regeneration is authorized. Abandoned unshipped 3.1.0 artifacts have no promised reader compatibility and are not authoritative regression inputs. This non-deletion rule does not assert that a 3.1.0 artifact was produced; record identified paths and hashes in `review-remediation.md`, not unsupported presence or absence claims.
- Carry resolved tool configs through `PrepassContext` in `crates/slicer-runtime/src/run.rs`; consume them with request-derived feedrate config in the model path of `crates/pnp-cli/src/visual_debug.rs`. Test actual feedrates with differing tools plus the default-config control.
- Retain visual request/config fixtures and validate their bundle. Retain `[resolved_config]` host-key mirror; update `resolved_config_keys_match_default` in `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` to use the typed accessor rather than a fixed-field arm.
- Update packet docs, glossary, canonical config/IR descriptions, bounded ADR-0052 append, generated doc 15 and TASK-571 evidence. Normal Cargo regeneration only; prior production-Postcard lockfile consequences are not automatically justified after its retirement. Validate convergence/freshness rather than hand-editing locks.
- Q8 authorizes separate accepted `docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md` and one `D-CSR10-ADR-0052-AMENDED` registry row in `docs/DEVIATION_LOG.md`, quoting ADR-0052's contested clauses. Only unchanged-body, exact single-function placement and direct-production-call mechanism requirements are superseded. ADR-0052 packet append points to that record; original Decision/Consequences/reconciliation/2026-08-05 stay unchanged and all other carrier/replacement/fallback/profile-length/mutation constraints remain.

## Out of Scope

- Packet 04 overhang percentages and both config-only negative mirrors; packet 05 resolution/precedence.
- Volumetric capping of explicit positive speeds, invented undeclared Orca sentinels, blanket extensions policy for future host keys.
- WIT, ConfigView contracts, CLI/manifest schemas, visual UI or renderer changes, persisted shape migration and unsupported-version public API guarantees.
- Approved plan or other packet edits; fixture regeneration, deletion of experimental artifacts, commits or full-workspace tests.
- Other ADR edits except the exact new ADR-0072 record and ADR-0052's packet append; deviation rows other than `D-CSR10-ADR-0052-AMENDED`. No public factor-interface or absolute point-speed change.

## Authoritative Docs

Read `docs/02_ir_schemas.md` resolved config/interner/RegionMapIR/versioning; `docs/adr/0052-per-point-speed-factor-contract.md` Decision and append; `docs/19_visual_debug.md` model/tap/bundle; `docs/22_test_quality.md` falsifiable assertions; `docs/21_data_defaults_and_fixtures.md` FRU. Read the approved plan's Phase C and compatibility checklist without editing it.

Read `docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md` for Q8's narrow mechanism amendment and `docs/DEVIATION_LOG.md` for the registered `D-CSR10-ADR-0052-AMENDED`. The original preflight returned `PREFLIGHT BLOCKED` on S8; after the amendment the repeated preflight returned `PREFLIGHT PASS` (2026-09-30). Q8 is resolved; closure authorization remains with a completed full review.

## OrcaSlicer Reference Obligations

Delegate `GCode.cpp::GCode::_extrude` canonical formula/tool/guards. Return SUMMARY at most 200 words outside thinking; no line-number citations or direct canonical reads by implementer.

The independent read-only source check confirms the zero quotient and active `m_writer.filament()` as well as a separate explicit-positive cap. This packet claims only scope-limited formula/tool parity: the user intentionally retained uncapped explicit positive speeds. Source inspection is not test or closure PASS.

## Acceptance Summary

AC-1/2/7 prove literal emission, differing tools, uncapped explicit speed and missing-tool fallback. AC-2's explicit-speed control must be above ceiling: exercise 30 mm/s against a positive `filament_max_volumetric_speed` limit of 1.0 mm³/s with geometry that puts the ceiling below 30 mm/s, and independently assert literal `F1800` rather than inferring uncapped behavior from automatic-speed outputs. AC-3 proves derived census and nonnegative declaration metadata. AC-4 proves resolved-tool visual emission and valid bundle. AC-5 proves generator and accessor-based doc lock, including absent zero versus stored 8. AC-6 proves unchanged-layout fixture deserialization and extension/paint identity. AC-8 exercises real `resolve_scope_stack`: numeric default zero, global 20, tool 1 List first value 12, missing tool 20, invalid values and denied object scope. AC-N1/N2 cover bad automatic inputs and final-F safety. Full-file commands avoid promising remaining new test names before implementation; review must inspect the actual assertions, not accept non-empty tests alone as complete coverage.

## Verification Commands

| Command | Purpose | Return |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd 2>&1 \| tee target/test-output.log'` | Real-path literals, fallback and final-F failures | FACT; non-empty passing file, bounded failure excerpt |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --all-targets --test region_map_versioned_decode_tdd 2>&1 \| tee target/test-output.log'` | Preserved 3.0.0 oracle, typed extension access and config/paint identity | FACT; non-empty passing file |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test automatic_value_expansion_tdd registry_negative_sentinel_census_has_no_unowned_phase_c_candidate -- --exact 2>&1 \| tee target/test-output.log'` | Derived census | FACT; exact passed marker |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test registry_census_tdd 2>&1 \| tee target/test-output.log'` | Full registry-census target, including roster follow-through assertions | FACT; non-empty passing file |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd 2>&1 \| tee target/test-output.log'` | Tool-capable denial regression | FACT; non-empty passing file |
| `cargo xtask test --summary -p pnp-cli --all-targets --features report visual_debug_volumetric_auto` | Target-neutral prefix, request and resolved-tool emitted output | FACT; matching passed marker and inspect both controls |
| `cargo xtask test --summary -p slicer-runtime --test unit host_keys_doc_lock_tdd` | Mirror default through accessor | FACT; `resolved_config_keys_match_default` passed marker |
| `cargo check --workspace --all-targets` | Required all-target compilation gate, including test and benchmark targets | FACT |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required all-target lint gate, including test and benchmark targets | FACT |
| `cargo xtask check-literals` | Struct-literal discipline | FACT |
| `cargo xtask check-test-quality --report` | Review touched-test findings | FACT findings with disposition |
| `cargo xtask gen-config-docs --check` | Generated reference drift | FACT |
| `cargo xtask build-guests --check` | Freshness and lock convergence | FACT exact exit 0/1/3 |

## Run and Completion Discipline

The workspace check/clippy gates require `--all-targets` so test and benchmark targets are compiled/linted; this does not ask to run Criterion benchmarks. The focused runtime command (`cargo xtask test --summary -p slicer-runtime --test unit host_keys_doc_lock_tdd`) selects the `unit` test target and named test filter, not Criterion benches; preserve its assertion and the registry-census target's assertions. Every plain cargo test command above runs with pipefail and combined-output tee to `target/test-output.log`; xtask test already writes that log. Guest-touching tests use the gated xtask entry point sequentially. Inspect and archive each fresh log under `target/packet10-remediation/` before overwrite; never rerun for truncated stdout or reuse a historical receipt. AC-6 requires the current named decode, roundtrip and interning markers in the retained receipt. Rebuild stale guests; infrastructure exit 3 is not clean. Closure needs fresh preflight, all ACs and gates, then full review before status changes. No unrun check is PASS, no full-workspace test, no commit.
