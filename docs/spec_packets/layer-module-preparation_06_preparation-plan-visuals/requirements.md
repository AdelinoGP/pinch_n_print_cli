# Requirements: preparation-plan-visuals

## Packet Metadata

Status draft; aggregate context M. This file owns scope and the verification matrix; task-map owns the TASK-575 backlog crosswalk. Creation is authorized by approved queue row06; no implementation is authorized by generation.

## Problem Statement

Metadata-only preparation captures cannot show module-private planning geometry. Interpreting plan bytes on the host would violate private codec ownership; a separate drawing export would violate committed diagnostic capture. One optional typed snapshot published in the existing transaction solves both while preserving ordinary output.

## In Scope

- Gate A: named declared views, requested sink presence, generic Point2 points/open polylines/filled or outline polygons-with-holes, diagnostic classes/labels/ids and explicit whole-print scene completion, including empty completion.
- Typed native/SDK/WIT transport with canonical imported geometry identities, checked accounting, owner/print/call-local enforcement and permanent write-fault poison. Validate all attachments before the existing atomic Ready transition. Original module/trap/cancellation precedence remains the producer's authority.
- No request means no framework allocation/retention of diagnostic geometry. Both legs expose the same sink and errors; a non-geometric module may declare no views.
- Gate B: structured full module-id/view tap selector under new request schema 1.4.0; preserve all prior schemas. Model/XY only. Validate syntax/source/dimension/declaration before invoking modules; schedule layer and eligible-owner resolution after final committed products and before preparation.
- Share #05's validated startup and final PrePass/preparation prefix. Preparation-only captures create no LayerArena; mixed taps run the required selected-layer closure with the actual prepared runner and retain plans until it completes. Postpass comparisons retain their existing whole-print closure, not a fabricated minimal path.
- Immutable shared post-commit snapshots, generic CapturedIr variant, Projector rendering, full-plan/model framing and deterministic class assignment independent of rendered subset. Store each scene once, not once per layer.
- Existing PNG/manifest bundle and overwrite rejection behavior; typed numeric mirror, real layer/Z, owner/version/artifact, consumer stage, capture phase, projection schema, class legend and empty status; actual execution evidence kept distinct from rendered layers.
- Real same-source native/component fixture and production-adapter contract driver; runtime e2e registration and CLI auto-discovered test target. Independent literal geometry/output/pixel oracles, positive nonempty controls and negative mutations. Loud fixture/artifact absence, never silent skips.
- Four normative doc sections identified in AC-8; literal/test-quality/freshness/build/check/clippy gates.

## Out of Scope

Lightning-specific projection, any codec/plan parser, ordinary IR plan accessors, additional scheduled stages, drawing callbacks after preparation, retained guest instances/arenas, configurable diagnostic quota/cache/spill, front/side/3D plan views, standalone G-code reconstruction, extrusion volume/width claims, ProgressEvent schema changes and canonical geometry/parity claims. Other packets, source plan, backlog and ADRs remain untouched during generation; implementation does not edit producer packet documents.

## Authoritative Docs

Approved plan projection contract and visual witnesses; ADR-0066 consequences; docs/19 request compatibility, Projector/framing and tap closure; docs/17 instrumentation versus static declaration; docs/08 conversion; docs/21 literals; docs/22 independent oracle/negative control. Author reads were bounded symbol/section windows; producer designs and exact #05 public exports were reconciled. No Orca delegation snippet applies.

## Acceptance Summary

Gate A: AC-1/2 and AC-N1. Neutrality: AC-3. Gate B: AC-4–8 and AC-N2. Both gates are required for completion. #08 may consume the generic SDK scene/sink and bundle contract only after implementation acceptance; it owns the useful lightning view.

## Verification Commands — authoritative matrix

Each command runs from workspace root. Every test tees combined output and checks nonzero tests; exact individual commands are attached to their ACs. Inspect the saved log before any next run overwrites it. Delegate commands with FACT pass/fail, executed-test count and at most 20 failure lines outside thinking.

| Command | Purpose / authority |
| --- | --- |
| `cargo xtask build-guests --check` | Check exit status: 0 fresh, 1 stale (rebuild then recheck), 3 infrastructure blocked. Rebuild via `cargo xtask build-guests`; staged in-tree mtime mismatch requires `--force`, not source touches. |
| `cargo build --tests` | Required after canonical preparation WIT/typed binding changes; no ordinary world widening. |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test layer_module_tdd preparation_projection -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Facade/sink None, DTO conversion, native callback parity; extend existing actual SDK target. |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --features preparation-test-fixtures --test contract preparation_projection_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Gate A production store/frozen adapters, real native/component calls. Required feature is FORWARD #03. |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e preparation_visual_capture_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Shared-prefix production execution, cancellation/error cleanup and semantic neutrality. Runtime enables host-algos; e2e is ungated. |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test visual_debug_render_tap_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Exhaustive CapturedIr arm, renderer and Step10 public-scene-only serialization regression; traces AC-5/6 through the existing tagged envelope, shared Arc and explicit empty scene, with exact-key/literal checks against payload leakage. |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test visual_debug_render_tap_tdd preparation_projection_serializes_only_scene -- --exact 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Bounded decisive serialization regression in the existing auto-discovered binary; actual production-committed native fixture snapshots, not private-type construction. |
| `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Gate B real public run_visual_debug/CLI bundle request, render, compatibility and failure atomicity. Auto-discovered ungated integration binary. |
| `set -euo pipefail; mkdir -p target; : > target/test-output.log; for test_target in visual_debug_request_bundle_tdd visual_debug_validation_tdd visual_debug_typed_tap_capture_tdd visual_debug_agent_determinism_tdd visual_debug_silhouette_bundle_tdd visual_debug_overlays_tdd layer_range_scope_visual_debug_tdd; do cargo test -p pnp-cli --test "$test_target" 2>&1 \| tee -a target/test-output.log \| python -c 'import re,sys; lines=[line.strip() for line in sys.stdin if line.startswith("test result:")]; assert len(lines)==1 and re.match(r"^test result: ok\. [1-9][0-9]* passed; 0 failed;", lines[0]), (sys.argv[1],lines); print(sys.argv[1]+": "+lines[0])' "$test_target"; done` | Preserve every legacy target. Each individual invocation must produce exactly one nonzero passing result; missing/zero/failing results abort even if another target passed. Pipefail also preserves Cargo/tee failures. Initialize full log once, append every target's complete combined output, and read that log for detail; never rerun to recover truncated output. No new feature gate on these homes. |
| `cargo xtask check-literals` | Full literal churn enforcement; newly watched DTO test literals use FRU or justified exhaustive waiver. |
| `cargo xtask check-test-quality --report` | Fix/waive touched-code findings; report mode is not a claimed enforce pass. |
| `cargo check --workspace --all-targets` | Compile every target, including enum-match fallout. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate. |

No workspace test ceremony is required by this packet. Narrow single-crate runs remain plain cargo test after the guest freshness gate; any later authorized broad/multi-crate run must use `cargo xtask test --summary`.

## Step Completion Expectations

All requested views complete before Ready; all required owners Ready before any consumer. Capture never opens opaque pieces. Snapshot copies contain only typed geometry and metadata and may outlive disposed private storage. Accounting gauges before disposal and cleanup zero are separate; no allocator highwater or timing is invented. The visual path and full slice use the same prefix, eligibility and frozen artifact selection.

## Context Discipline Notes

Large CLI/executor/render source files require named-symbol windows. Read neither generated bindings nor target fixture bytes as source. Independent preflight belongs to the orchestrator; draft self-review is not a preflight PASS or acceptance result.
