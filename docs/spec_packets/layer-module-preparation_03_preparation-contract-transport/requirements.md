# Requirements: preparation-contract-transport

## Packet Metadata

- Task: `TASK-574`, partial queue-row-03 coverage; source `docs/07_implementation_status.md`.
- Status: **draft; no implementation, execution gates, activation or commit**.
- Approved anchor: `docs/specs/layer-module-preparation-plan.md`, eight-row queue, row 03, dependent on row 02.
- Aggregate/largest-step context cost: M/M.

## Problem Statement

The generated draft pilot deliberately keeps declaration handling and resource/storage backing in a contract harness. Passing authoring preflight does not supply production transport. Row 03 must normalize that exact capability into enforceable declaration/frozen-binding and owner-private storage APIs before typed inputs and runtime orchestration consume it. This is not an algorithm migration or another ingestion/selection model.

## In Scope

- Parse, store and validate optional `[preparation]` rather than tolerate it as unknown metadata. Require a qualified `interface` and `input_reads` string array; default absent `views` to empty. Reject unknown keys, wrong table/value shapes, empty or duplicate permission/view names, and unsupported interface identity. Preparation permission is independent of ordinary `[ir-access].reads`/runtime `ir_access`.
- External spelling is exactly `[preparation].input_reads`, exposed by Rust `PreparationDeclaration::input_reads()`, not an `input-reads` alias. Amend the canonical docs/04 naming map with an explicit snake_case exception limited to this new metadata table (`interface`, `input_reads`, `views`); legacy manifest/config keys and existing naming-map rows remain unchanged. Semantic parser tests cover both the new spelling and retained legacy ingestion.
- Preserve one manifest-ingestion authority. Store preparation in an additive companion, not a new `LoadedModule`, `ModuleDeclaration`, `LiveModuleBinding`, schema-envelope or native-request field. Row 05 connects the companion to selection/load orchestration.
- Validate declaration and compiled capability metadata in both directions. Reject missing, undeclared, incompatible or non-Layer preparation before invoking module code; native metadata is paired with the generated stage/preparation entry, WASM validation checks actual exports and canonical imported resource identities by typed instantiation.
- Freeze both calls from one native artifact pair or one WASM component. Do not independently rediscover a preparation provider. Provide explicit production invocation scopes; do not select/schedule modules here.
- Normalize SDK host callback/read facades and macro glue to the production boundary while preserving the pilot's exact trait, attribute, native pair and WIT signatures. Required-piece convenience returns fatal `ModuleError` code `57401`; module-owned decoder faults remain module errors.
- Production print-scoped owner storage: implicit owner identity; complete unique `put` once per name; nonempty exact case-sensitive UTF-8 keys (including whitespace/path-like/decomposed strings); immutable retained bytes; checked conversion, offset/range arithmetic and accounting; bounded cursorless owner-wide reads.
- Validate offset against length first, then checked length-minus-offset and min(request, remaining). A valid oversized request clips; it is not rejected because offset plus the unbounded request would overflow. Checked conversion/accounting failures still return ArithmeticOverflow. Range fixtures use fresh or explicitly nonoverflowing counters; overflow counters have their own negative witnesses.
- Invocation-local staging with irreversible poison on any failed put, atomic successful-completion marker, successful empty ready collection, no publication on returned error/trap/unwind/dropped builder/rejected validator. Preserve original `ModuleError.code`, `.message`, `.fatal`; required dependency failure is not weakened by `fatal == false`.
- Fresh call-local native and WASM read wrappers/table handles; no retained guest instance, module object, table index, algorithm object or arena. Owner/print isolation, concurrent independent ranges, unwind-safe native binding restoration.
- Safe disposal APIs that reject active-use disposal with `Busy`, clear stored name/payload bytes, and make old read handles fail `AccessDenied`; pipeline lifetime/release ordering is row 05.
- Checked metadata/accounting for payload bytes and UTF-8 name bytes separately, transfer counts/requested/returned bytes and preparation calls. Redacted metadata/debug output contains no automatic raw payload dump; totals do not represent allocator overhead or peak memory.
- A synchronous pre-ready validation hook over private staging, with no geometric meaning, allowing row 06 to extend staging and atomically publish validated diagnostic attachments later. Parse/store declared `views` now; do not validate actual projections here.
- NET-NEW controlled-only host feature `preparation-test-fixtures`, a small dual-target PathOptimization module-owned codec fixture, actual production typed resource/native adapter tests, and explicit registration in the existing contract aggregator. Fixture input availability is not a production input API and is not a replacement store.
- Preserve ordinary pre-existing guests, use the pilot's archived old artifact/provenance rather than replace it with a fresh build; cover every schema-backed Layer binding through compiled typed adapter coverage and the ordinary export authority.
- Same-packet canonical-doc updates listed in `packet.spec.md`; no plan/backlog/other-packet edit during authoring or implementation of this partial task.

## Out of Scope

- Row 04 final typed whole-print PrePass/target/config input projections and their vocabulary/validation; `read-fixture` remains controlled-only.
- Row 05 eligibility, autoselection/search precedence integration, late-PrePass placement, cancellation outcome orchestration, dispatch scheduling, anchored lifetime/release and runtime execution-event wiring.
- Row 06 visual-debug request/schema/capture/rendering, actual typed diagnostic projection validation and geometry storage. The publication seam and declared names are the only preparation for it here.
- Algorithm codecs in host, lightning migration/retirement, canonical geometry ports, changed coordinate conventions, quotas, spill-to-disk, caching, compression or payload-schema negotiation.
- Changes to ordinary WIT worlds/signatures, existing public struct fields, scheduled `STAGES`, live IR version constants, reserved future IR versions, ADR/deviation slots, or an extra preparation scheduled stage.
- Speculative workspace test suite; no `cargo test --workspace` requirement.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: approved sections/queue stated in packet contract, bounded reads only.
- `docs/03_wit_and_manifest.md`: typed identity/manifest/access-enforcement sections; `docs/01_system_architecture.md`: data ownership; `docs/05_module_sdk.md`: entry point/state lifecycle; `docs/04_host_scheduler.md`: frozen plan/ingestion and “Manifest ↔ Runtime Naming Map (Normative)” sections. The latter's blanket kebab-case rule receives only the explicit new preparation-table exception. Direct bounded sections plus delegated authority summary, not full long-doc reads.
- `docs/adr/0066-private-layer-preparation-capability.md`: “Consequences”/“Relationship to existing decisions.” Conform to `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` and `docs/adr/0056-integrated-modules-native-dispatch.md`, both “Decision.” No parity port or Orca attribution work applies.
- `docs/21_data_defaults_and_fixtures.md` §1/§4 and `docs/22_test_quality.md` §4/§5: new test literals use FRU or a reasoned exhaustive waiver, independently pinned expected literals, nonempty populations, loud fixture absence.

## Acceptance Summary and Trace

| Criteria in packet.spec.md | Steps | Protected contract |
| --- | --- | --- |
| All host-feature ACs | 0, 8a, 9 | Explicit feature, fixture dependency and registered real driver |
| AC-1, AC-N1 | 1–3, 5–7, 10 | Declaration storage, compiled agreement, pre-execution rejection |
| AC-2 | 3, 5–9, 11 | Real serialized native/WASM fresh-call transport |
| AC-3, AC-4, AC-N5 | 4, 7, 9, 11 | Exact keys, checked bounds/conversion/accounting |
| AC-9 | 2, 13a | Semantic new-table spelling/legacy ingestion and scoped canonical authority amendment |
| AC-5 | 3–4, 8–9, 11 | Ready-empty versus unavailable; optional versus fatal decoder |
| AC-6 | 4–7, 9, 13 | Owner/print/cursor/TLS isolation and disposal |
| AC-7 | 4, 7, 9, 13 | Metadata only, separate name/payload/transfer accounting |
| AC-8 | 6, 9–10, 14 | Ordinary old artifact and schema-derived typed stage coverage |
| AC-N2 | 2–3, 7, 9–10 | Deny-by-default separate preparation permission |
| AC-N3, AC-N4 | 4–9, 12 | Poison/error/trap/unwind and extensible atomic publication |

Cross-packet consumers use only the forward summary, not private test helper names. All consumers must retain FORWARD-DEP labels until executable dependencies actually pass.

## Verification Commands — authoritative full matrix

Commands are future implementation gates, **not executed authoring evidence**. Delegate with `FACT pass/fail`, outside thinking, escaped, plus at most 20 failure lines. Every Cargo test uses combined `tee target/test-output.log`; every test run must assert a nonzero executed-test count, using the same Python count check in the ACs (exact one for exact filters, `n > 0` for a whole named binary/prefix). Inspect the saved log rather than rerun for truncated output.

| Command | Purpose / feature discipline |
| --- | --- |
| Every complete pipe-suffixed AC command | Exact driver/resource behavior, counted execution and documentation section check |
| `cargo metadata --format-version=1 --no-deps` summarized to named package features/targets | Recheck registration after row 02 lands; no lock/full metadata dump |
| `cargo xtask build-guests --check >target/preparation-freshness.log 2>&1` | Exit 0 fresh; 1 stale requires rebuild without `--check`; 3 infrastructure error blocks. Never infer freshness from absence of `STALE:` |
| `cargo xtask build-guests` followed by `cargo xtask build-guests --check` | Rebuild changed guests including the new discovered fixture; ordinary mode only. Do not introduce accelerated artifact claims |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-scheduler --lib preparation::tests:: 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | NET-NEW inline parser tests, no feature gate; no aggregator edit needed |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test layer_module_tdd preparation_transport_ 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | Existing actual target requires `test`; additive facade/TLS/required-piece tests |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-macros --test slicer_module_tdd preparation_transport_ 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | Existing target, no custom features; opt-in/native sidecar regression witnesses |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-schema --lib package_lookups_match_per_stage_table 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | Preserve existing stage version/package assertions, no schema feature |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-macros --test binding_surface_tdd 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | Existing export/schema/JSON/determinism assertions remain unchanged |
| `set -o pipefail; mkdir -p target; cargo test -p slicer-macros --test postpass_text_glue_tdd 2>&1 \| tee target/test-output.log >/dev/null && python -c "import re,pathlib; n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',pathlib.Path('target/test-output.log').read_text()))); assert n>0; print('PASS executed='+str(n))"` | Existing text package/version assertion regression guard |
| `cargo build --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` | Build every target, including controlled production-adapter tests |
| `cargo check --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` | All-target acceptance gate, not a feature-blind library-only check |
| `cargo clippy --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures -- -D warnings` | All-target lint gate |
| `cargo check -p slicer-wasm-host --all-targets` | Default production mode compiles without controlled fixture feature |
| `cargo xtask check-literals` | Enforce struct-literal gate; no narrowing to obtain a pass |
| `cargo xtask check-test-quality --report` | Report gate; fix/justify touched-code findings, never weaken assertions |
| Every doc-impact grep in packet.spec.md | Same-packet docs, including SDK lifecycle clarification and scoped scheduler naming exception |

Metadata was actually summarized during authoring: `slicer-wasm-host` has no existing features, `contract` resolves to `tests/contract/main.rs`; SDK `test` exists and `layer_module_tdd` requires it; scheduler/schema/macros have no custom features. The new fixture feature is explicitly NET-NEW. `slicer-wasm-host` already has production dependencies on SDK/scheduler/schema. No new crate or production serde dependency is needed.

## Step Completion Expectations

No implementation step begins until row 02's executable pilot and independent preflight permit activation. The production store/resources must exist before transport tests are adopted; the test driver may construct fixtures/calls but must never provide an equivalent staging store, native facade or imported Host implementation. Rows 04–06 extend only published seams. Closure of row 03 does not close the shared backlog TASK-574.

## Context Discipline Notes

Use symbol windows for long `host.rs`, `manifest.rs`, schema/macro sources and authoritative docs. The pilot harness is read-only precedent, not a transport to retain. Do not browse target outputs as source; tests may load/hash their explicit artifact paths and provenance. No full implementation-status read, whole repository grep dump, generated binding expansion or Orca source read is needed.
