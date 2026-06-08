# Packet 92 Closure Log

P91_BASELINE_SHA=e60fca7fb4ea67fd54a402c2d352ae7719f82389a4ac4669b497922c9301f674
P92_POST_SHA=e60fca7fb4ea67fd54a402c2d352ae7719f82389a4ac4669b497922c9301f674
AC_11_CHECK=clean@6c872f3  # cargo xtask build-guests --check exited 0 with no STALE entries after both commits landed (post-Commit B HEAD). Audit corroborated W6's transcript; rebuild required first because intermediate cargo test runs (audit pass) had invalidated guest fingerprints.

## Per-validator test names (Step 4)
- region_split_manifest_basic (AC-1)
- region_split_duplicate_semantic_rejected (AC-3)
- region_split_scalar_rejected (AC-4)
- region_split_community_priority_floor (AC-5)
- region_split_core_priority_mismatch (AC-6)
- region_split_priority_type_mismatch (AC-N3)

## Aggregation test names (Step 5)
- region_split_aggregation_canonical_order (AC-8)
- region_split_tied_priority_warn (AC-7)
- region_split_aggregation_empty_default (AC-N2)

## LoadErrorKind variants (Step 3)
- Added: DuplicateRegionSplitSemantic, ScalarValueTypeNotAllowedInRegionSplit, CommunityPriorityBelowFloor, CorePriorityMismatch
- Reused (no new variant): Schema (missing-required-field), TomlParse (malformed type per AC-N3)

## Documented in-packet deviations (spec-review GREEN, all LOW severity)
- D-92-1 Fixture directory `aggregation/` (W4) instead of `tied_priorities/` (design.md). Test intent fully covered.
- D-92-2 `module_invocation_allowed_on_layer(declared: &HashSet<String>, slice: Option<&SliceIR>) -> bool` instead of design.md's iterator-typed signature. W5 chose the cached HashSet path; SliceIR Option supports the conservative-allow case when no IR is present. AC-9 satisfied; reasoned improvement.
- D-92-3 Filter call site at layer_executor.rs:362 (before `on_module_start`) rather than the design.md-cited line 385. Skipped modules now absent from instrumentation and audit log — deliberate improvement; comment in code records the intent.
- D-92-4 `#![allow(clippy::result_large_err)]` added to crates/slicer-scheduler/src/manifest.rs (W6). The 4 new String-bearing LoadErrorKind variants pushed LoadError past clippy's 128-byte threshold; the `allow` is documented with rationale and avoids breaking the public Result<_, LoadError> surface. Boxing was the alternative and was rejected as out-of-scope for this packet.
- D-92-5 CompiledModuleStatic + CompiledModuleBuilder in crates/slicer-scheduler/src/execution_plan.rs extended with `region_split_semantics: HashSet<String>` field + accessor + builder setter (W5). Implied by design.md's "the runtime's LoadedModule descriptor in scope at line 385 carries region_split_semantics" — the propagation surface is the execution_plan side.

## Workspace gate (Step 9)
- cargo clippy --workspace --all-targets -- -D warnings: PASS
- cargo test -p slicer-scheduler: 145/145
- cargo test -p slicer-runtime --test integration: 157/157 (zero regressions vs pre-packet baseline 153; +4 new tests)
- cargo xtask build-guests: 33 guests built
- cargo xtask build-guests --check: 0 STALE entries

## TASK row
docs/07_implementation_status.md:213 — `- [x] TASK-242 — Manifest [[region_split]] schema + priority registry + per-layer host-filtered dispatch (packet 92). Closed 2026-06-08 — packet 92.`
