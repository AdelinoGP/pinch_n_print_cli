# Requirements: preparation-capability-pilot

## Packet Metadata

- TASK-573; backlog crosswalk belongs to `task-map.md`.
- Approved source: `docs/specs/layer-module-preparation-plan.md`, queue row 02.
- Status draft; aggregate context M; independent preflight must pass before activation. No implementation or test execution.

## Problem Statement

An encode/decode probe cannot prove same-artifact typed resource identity, macro/native authoring, or fresh-store consumption. This packet must produce executable evidence before row 03 finalizes production bindings. Controlled fixtures keep the proof independent of routing repair and lightning fidelity.

## In Scope — authoritative full scope

- Add separately versioned capability WIT, imported real host input/output/read resources, combined worlds and shared canonical type mappings. Ordinary packages/signatures remain unchanged.
- Required explicit SDK preparation trait, opt-in macro export composition and same-source generated native adapter; no default successful preparation. Ordinary `from_config` runs per Layer call.
- Test-local private staging/ready state, immutable serialized pieces, unique one-call complete puts, owner-private reads, fresh wrappers, store destruction and atomic success/poison semantics. This pilot implements the public transport witness, not normalized production persistence.
- Fixture-only input resource with checked bounded reads and explicit access denial. No production whole-print projection or fabricated RegionKeys.
- Real infill and non-infill PathOptimization adapters with different input families and independent expected builder outputs; module-owned codecs rather than host interpretations of piece names.
- Schema-derived compile coverage for every module-backed Layer row in prepared/plain forms, real native and guest compilation/linking, missing-required-method negative compile witness.
- Real pre-existing ordinary classic-perimeters artifact typed load unchanged; combined linker imports, exact compatibility failure for requested missing/incompatible preparation exports.
- Named witnesses for owner/print isolation, ready-empty/missing, optional/required pieces, duplicate/name poisoning, failed/trapped preparation, bounded ranges, checked conversions/retention arithmetic and malformed codec rejection.
- Measured UTF-8 name and payload retention, transfer byte/count records, representative prep/read durations; metadata-only normal diagnostics. No claimed measurements before execution.
- Test-guests discovery, same shared target and artifact staging, capability-aware artifact verification. Preserve singular ordinary stage resolution, full capability export validation and ordinary unrelated-package compatibility.
- New actual typed driver in `slicer-wasm-host` contract tests and explicit registration; SDK/native guest crates as host dev-dependencies. Compile-matrix scratch components derive from schema authority, not a permanent manually maintained module roster.
- Implementation-only documentation sections named by packet ACs and existing task completion through backlog worker at actual closure.

## Out of Scope

- Production `[preparation]` parsing/validation and normalized host storage (#03); declaration-gated production PrePass IR/config/target projections (#04); scheduling/selection/override/cancellation lifecycle activation (#05); diagnostic geometry and visual-debug (#06).
- Routing TASK-572, any source plan/queue edit during this authoring task, other packets, unrelated backlog tasks.
- Lightning input/kernel changes, host-path retirement, canonical parity and Orca references; transport is not algorithm evidence.
- New retained quota, disk/cache/cross-print reuse, public payload metadata export, append writers, shared plan catalog, retained guest/store/index/unfinished arena or native algorithm object.
- Broad ordinary-stage error policy repair; no workspace test default.

## Authoritative Docs

Direct reads grounded the normative map in `docs/00_project_overview.md`; source plan declaration/private lifetime/pilot witnesses; ADR-0066 in full; ADR-0045 decision and resource compatibility; ADR-0056 one-model native dispatch; `docs/01_system_architecture.md` Tier 2 and Data Ownership Rules; `docs/03_wit_and_manifest.md` Host-Boundary Access Enforcement, Concurrency & Instance Isolation, WIT Package compatibility; `docs/05_module_sdk.md` Single-Stage-Per-Impl Constraint and Module State Lifecycle; `docs/08_coordinate_system.md` Rule; `docs/21_data_defaults_and_fixtures.md` literal discipline; `docs/22_test_quality.md` falsifiability and compile-witness separation. Long documents are ranged, not loaded whole.

## Acceptance Summary

- Positive: AC-1 fresh guest; AC-2 native equivalence; AC-3 schema-derived compile and non-infill dispatch; AC-4 plain compatibility; AC-5 measured accounting.
- Negative: AC-N1 privacy/readiness; AC-N2 atomic failures; AC-N3 bounds/codec; AC-N4 typed export validation.
- Cross-packet: planned forward seams in `packet.spec.md`; only an actual passing pilot permits row 03 binding finalization. Rows 04–06 must not treat controlled fixtures as production input/activation/visual coverage.

## Verification Commands — authoritative full matrix

Run after implementation only. Every test command below captures combined output, fails on pipeline failure, rejects zero tests, and emits compact `PASS executed=...`; on failure read the existing log rather than rerun for output. Test names and fixture files are NET-NEW. Existing `slicer-wasm-host` has no package features or target required-features and contract sources have no feature gates (verified metadata/manifests/source); use no invented `wasm` feature. SDK test support uses explicit `--features test`. `slicer-core` is not a direct test target here.

| Command | Purpose / return |
| --- | --- |
| `set -o pipefail; mkdir -p target; cargo xtask build-guests --check && cargo test -p slicer-wasm-host --test contract preparation_pilot_ -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"` | AC-1–5/N1–N4; FACT pass/fail plus executed count; failure SNIPPETS at most 20 lines. |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-macros --test slicer_module_tdd 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"` | Ordinary macro regression plus new opt-in parser tests, no required features. |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test layer_module_tdd 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"` | Ordinary SDK contracts, explicitly enabled test support. |
| `set -o pipefail; mkdir -p target; cargo test -p xtask --bin xtask preparation_pilot_ 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"` | NET-NEW capability resolution/comparison unit tests in existing bin, no features. |
| `set -o pipefail; mkdir -p target; cargo test -p xtask --bin xtask wit_verify::tests 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"` | Existing version/export/world checks and macro include/build-watch literal fallout. |
| `cargo xtask build-guests --list` | Both new cdylib guests discovered with staged paths; FACT matching entries; no build in this authoring session. |
| `cargo xtask build-guests` then `cargo xtask build-guests --check` | Rebuild stale artifacts; require exit 0. Exit 1 is stale (including lock divergence), exit 3 inability to judge. Use `--sync-locks` only for actual divergence; `--force` only for verified in-tree copy freshness disagreement. |
| `cargo build --workspace --tests` | WIT test-target build gate. |
| `cargo check --workspace --all-targets` | All test/bench/example compile coverage. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate. |
| `cargo xtask check-literals` | Enforced test literal gate. |
| `cargo xtask check-test-quality --report` | Fix/justify touched findings; report mode is not a quality pass by itself. |
| `rg -q '^## Preparation capability pilot composition' docs/03_wit_and_manifest.md && rg -q '^### Preparation capability pilot authoring' docs/05_module_sdk.md && rg -q '^## Executable pilot evidence' docs/adr/0066-private-layer-preparation-capability.md` | Exact implementation doc sections; prose must distinguish executed pilot and future production integration. |

No workspace tests are required. The schema-surface contract test owns a real compile subprocess driver: capture subprocess combined logs without claiming build success from source strings, use fail-fast status handling, summarize each plain/prepared stage/configuration and assert a nonempty authority-derived set. Freshness is required before attribution of any component failure. Guest-affecting broad/multi-crate test runs, if separately authorized, use `cargo xtask test --summary`; they are not implicit gates here.

## Step Completion Expectations

Keep ordinary public structs and existing version literals unchanged; sidecar capability metadata avoids unrelated field-literal churn. No later step may manufacture a ready plan to claim preparation coverage. Range and staging unit probes supplement, never replace, actual guest/native calls. Measure after correctness; timings are evidence without thresholds. Dependent implementation stops on actual feasibility failure instead of adjusting settled semantics.

## Context Discipline Notes

Reopen macro and host large files by symbol windows only. Do not read generated guest expansions, lockfiles or full Cargo logs. Return failing test name/assertion and at most 20 log lines. All quantities in future evidence must be read from actual execution, not forecast.
