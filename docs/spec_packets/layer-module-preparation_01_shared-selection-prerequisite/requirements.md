# Requirements: shared-selection-prerequisite

## Metadata and motivation

- `TASK-572`, approved eight-packet queue row 01; no prerequisites. Source: `docs/specs/layer-module-preparation-plan.md`, selection/configuration requirements and Packet Queue.
- Draft, aggregate context M. Authoring changes only this packet's five contract files. Implementation, activation, tests/gates, source-plan/backlog changes and commits are outside this authoring task; activation requires successful independent preflight and explicit approval.
- Ordinary delivery currently loses full variant identity in maps/claims, native config attachment and coloring grants. A reusable preparation projection must not canonize those losses or live separately from production consumption.
- The former non-region authority question is withdrawn: resolved filtered invocation config is the explicit authority. Missing mechanical channels do not require producer bindings, scope inheritance, WIT changes or user policy decisions.

## Full scope

Both internal families must pass; neither alone closes this packet.

### A — Full-identity delivery and coloring repair

- Preserve complete ordered object/region/variant identity; add current global layer for exact model-config lookup. Never use unordered map first-match, sibling borrowing, material-only identity, or invocation defaults as model authority.
- Ordinary native and WASM receive identical selected source/config/held-role values, filtered to declared reads. Preserve all held fill roles and extension filtering. Invocation-wide reads continue to use `bind_module_config_view`'s frozen view.
- Reject missing exact model/perimeter config before invoking a module, including absent map with actual model work. This reconciles and replaces the initial draft's fallback AC; it is not a global ordinary error-policy rewrite.
- Full-origin authored-color grants govern both native production commit and existing WASM conversion enforcement; preserve two-sided predicate, range checks and strip-to-`None` behavior.
- Test real runner delivery, independently expected output identities/widths/roles and independent slice/map permutations. Native recording is a transport observer, not geometry parity. Public accessor tests cover actual host traits/canonical-ID rejection; they do not claim an unimplemented WASM postprocess driver.
- Correct the painted-holder fixture's slice chain to match its map key; retain its three tests and assertions.

### B — Reusable targeting consumed in production

- Public concrete seam in `slicer-wasm-host::selection` is summarized in `packet.spec.md`. Derive stage applicability from schema authority, reuse existing perimeter source geometry and support-family/carrier rules, retain full merged source membership and multiple held roles.
- Runtime ordinary eligibility and dispatcher ordinary native/WASM routing consume the same projection; no unused preparation-only index, private per-leg resolver or separate chain-blind runtime family authority.
- Model-derived targets use exact full `RegionKey`; support carriers, explicit raft and actual anchored module stages carry tagged non-region contexts and invocation-effective config. Preserve support entry family/demand/body/anchor provenance. Never fabricate model keys or producer inheritance for these contexts.
- Preserve carrier selection by `SupportPlanEntry.global_layer_index == layer.index as i32`, not `anchor_layer_index`. AC-B2's `carrier_selection_delivery_fixture` selects exactly global 7/anchor 3 at layer 7 and rejects global 3/anchor 7, retaining wrong-family/declined exclusions and exact eligible cardinality. This proves selection/delivery/config, not nonempty rendering: the paint view and traditional-support renderer still filter by anchor. A separate execution of `support_rendering_positive_control` uses one global-7/anchor-7 carrier with nonempty support geometry and asserts actual nonempty native/WASM committed paths and points. Keep `raft_positive_control` as a third independent execution with nonempty paths on both legs. Do not add either positive control to the selection fixture. Anchor remains carrier provenance; no production predicate, export or scope change is introduced.
- Preserve print-wide raft plan access without entry anchor filtering; explicit `GlobalLayer.is_raft`, not layer-index inference. Preserve actual `Layer::AnchoredEvents` module config/eligibility separately from synthetic anchored host closure handling.
- Support optional absence must be observable as `Absent`, distinct from `Present` with zero selected work. Carrier-only positive fixture, wrong-family/declined negatives, missing plan, empty plan, nonempty eligible plan, no-slice transport and explicit empty invocation are required.
- Assert nonempty inputs/outputs in positive fixtures before negative checks. Register production-driver tests; no helper-only or zero-test false green discharges production criteria.
- Document the public routing seam/authority boundaries in the exact scheduler section named in Doc Impact. No other architecture doc/schema change is required because persisted contracts remain unchanged.

## Out of scope

Preparation capabilities/declarations/plans/input views/storage/SDK/WIT/lifecycle, lightning migration, diagnostic captures and later queue rows; producer config bindings/inheritance; config precedence registry rewrites; general nonfatal error-policy repair; schema/version bumps; geometry algorithms/Orca porting; new/modified guest sources; optional native integrated-module feature dependencies; baselines/goldens; broad workspace tests; plan/backlog/other-packet edits during generation or implementation steps.

## Governing authority

- `docs/02_ir_schemas.md`, IR 4 / Config Interner Contract: exact key and config interning.
- `docs/03_wit_and_manifest.md`, source of truth / declared reads: canonical WIT and module key filtering.
- `docs/04_host_scheduler.md`, RegionMapping / Error Handling Policy: delivery contract and retained global policy limitation.
- `docs/adr/0056-integrated-modules-native-dispatch.md`, Decision 1/3: provenance-independent selection, dispatch-only native/WASM split.
- `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §§1–2: literal and falsifiability discipline.
- `docs/specs/layer-module-preparation-plan.md`, shared-targeting semantics / Selection and configuration must agree with consumption / Tests and closure evidence / Packet Queue row 01: source/context/config/optional-product requirements and forward consumption.
- `docs/00_project_overview.md`, documentation map, provides the authority map only.

No OrcaSlicer behavior is translated; Orca/coordinate snippets are absent. No guest-source edit feeds artifacts, so the guest-source snippet is absent; artifact validation remains mandatory for real guest tests.

## Acceptance summary and traceability

| Family / IDs | Implementation steps | Proof |
| --- | --- | --- |
| A: AC-1/2/3/N1/N2 | 1, 2, 4, 5 | Five registered native/WASM runner tests; exact widths, roles, permutations, missing-authority rejection, unknown-holder suppression |
| A: AC-4/N3 | 4, 5, 6 | Two registered public accessor/native perimeter tests; exact identities, declared filtering, canonical decimal rejection |
| B: AC-B1/B2/B3 | 2, 3, 5, 7 | Three registered projector plus real native/WASM production tests; merged sources, carriers/raft, anchored module config |
| B: AC-B4/BN1 | 2, 3, 8, 9 | Two registered runtime production-driver tests; schema applicability, eligibility, absence/empty/positive controls |
| B: AC-BN2 | 2, 3, 5, 10 | One new test in existing coloring module, native production plus WASM conversion boundary; preserve all existing grant/strip tests |
| Doc impact | 11 | Exact section presence plus semantic review; no global policy rewrite |

Public export acceptance unblocks #04 and subsequently #05. This packet does not close later obligations of TASK-572 or turn a future preparation no-work state into a present API.

## Verification matrix

Future implementation commands only; NONE ran during generation. Run from workspace root, create `target/` before tee. Every test captures combined output to `target/test-output.log`; save bounded findings before the next test overwrites it. Return FACT verdict, count and log path; inspect the saved file, never rerun for truncation. Commands below filter successful output and fail on zero execution. Each AC owns its repeated exact-test command in `packet.spec.md`.

Command-verified `cargo metadata --format-version=1 --no-deps`: `slicer-wasm-host.features = {}`, `contract.required-features = []`; runtime features are `default = ["report"]`, `report = []`, `perimeter-spatial-test-support = ["slicer-core/perimeter-spatial-test-support"]`, and `executor.required-features = []`. Both aggregators were read and have no feature cfg gate. No additional feature is needed for the named binaries; new modules must not introduce one. Existing sliced/native fixture gates must still compile the asserted cases.

| Command | Purpose / return |
| --- | --- |
| `cargo xtask build-guests --check` | Before any guest/dispatch failure attribution: FACT exit 0 fresh, 1 stale (rebuild with `cargo xtask build-guests`, recheck), 3 infrastructure blocked. Never infer freshness from text absence. In-tree mtime stale despite clean shared artifacts requires `--force` and recheck. |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract variant_identity_delivery_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 5 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 5 executed')"` | A runner delivery suite |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract variant_identity_accessors_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 2 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 2 executed')"` | A accessor/native perimeter suite |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract slice_region_view_contract_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 2 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 2 existing resource tests executed')"` | Explicit origin config fixture migration; retain metadata and real arachne perimeter assertions |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract shared_selection_dispatch_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 3 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 3 executed')"` | B projection plus actual native/WASM production consumers |
| `set -o pipefail; cargo test -p slicer-runtime --test executor shared_selection_runtime_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 2 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 2 executed')"` | B real executor eligibility/absence/empty controls |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s and 'variant_grants_cover_native_and_wasm_boundaries ... ok' in s; print('FACT PASS: coloring witnesses executed')"` | Existing predicate/range/strip tests plus new full-origin production/boundary witness; inspect test results in saved log |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract infill_holder_resolution_painted_region_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 3 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 3 executed')"` | Corrected matching-chain fixture; preserve all assertions |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract view_seam_identity_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: existing identity tests executed')"` | Legacy direct native builder signature caller |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract region_eligibility_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: existing eligibility tests executed')"` | Legacy direct builder caller/adjacent eligibility |
| `set -o pipefail; cargo test -p slicer-wasm-host --test contract anchored_events_both_legs_tdd:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: existing anchored boundary tests executed')"` | Existing direct native commit wrapper |
| `set -o pipefail; cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd 2>&1 \| tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: raft witnesses executed')"` | Existing real raft access through both dispatch legs |
| `cargo check --workspace --all-targets > target/shared-selection-check.log 2>&1; result=$?; echo "FACT check exit=$result"; exit "$result"` | Compile all production/test/bench/example callers; FACT exit |
| `cargo clippy --workspace --all-targets -- -D warnings > target/shared-selection-clippy.log 2>&1; result=$?; echo "FACT clippy exit=$result"; exit "$result"` | All-target lint closure gate; FACT exit |
| `cargo xtask check-literals > target/shared-selection-literals.log 2>&1; result=$?; echo "FACT literals exit=$result"; exit "$result"` | Struct-literal gate; FACT exit |
| `cargo xtask check-test-quality --report > target/shared-selection-quality.log 2>&1; result=$?; echo "FACT quality-report exit=$result"; exit "$result"` | Report plus explicit fix/waiver of touched-file findings; report exit alone is not acceptance |
| `rg -q '^### Ordinary full-identity delivery \(Normative — TASK-572 row 1\)$' docs/04_host_scheduler.md` | Required doc-impact section presence; FACT exit plus delegated semantic review |

No `cargo test --workspace` acceptance ceremony is required or permitted by this packet. Narrow single-crate runs use plain Cargo and specified test binaries; whole-suite/multi-crate runs, if later explicitly authorized, must use `cargo xtask test` with freshness preflight.

## Cross-step obligations

Capture behavioral red before repair, not missing artifact/unregistered test/compile-error red. Registered future interface tests may initially fail to compile until the seam exists; that is scaffolding, not defect evidence. Keep all expected identities/roles/values and input/output nonempty guards. Never alter canonical geometry to satisfy a fixture. Stop/reconcile unlisted required field/WIT changes or a fourth edit in one step. Both families and production integration are closure requirements; no export-only completion or inherited blocker remains.

Long files use symbol searches and bounded windows; delegate heavy commands and normative fact-checks with compact escaped results outside thinking. Schema/producer/generated/lockfile inspection is not an excuse to broaden this packet.
