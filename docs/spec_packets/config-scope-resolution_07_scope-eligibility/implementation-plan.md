# Implementation Plan: scope-eligibility

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-568`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every cargo command is delegated and tee'd to `target/test-output.log`.

## Steps

### Step 1: Reconcile resolver and inventory declaration blast radii

- Task IDs: `TASK-568`
- Objective: confirm packet 05's landed shapes and enumerate all host declaration literals, speed consumers, roster declarers, legacy fields, and 48 manifest section headers.
- Precondition: packet 05 is landed.
- Postcondition: bounded inventories cover every edit owner and confirm the AC-1 roster/declarers.
- Files allowed to read, with ranges when over 300 lines: predecessor authority files and exact symbols/manifests named in design.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds: code edits, guest source, WIT, fixture contents, generated code.
- Blast-radius discipline: dispatch `LOCATIONS` for every `HostConfigKey` literal and every test hard-coding its field shape before Step 2.
- Expected sub-agent dispatches: exports `FACT`; struct/declarer/legacy locations in ≤20-entry batches.
- Context cost: `S`
- Authoritative docs: plan Eligibility/Resolution and ADR-0069.
- OrcaSlicer refs: none.
- Verification: inventories account for all four declaration channels, five nozzle declarers, 24 manifests, and both headers per manifest.
- Exit condition: any prerequisite shape or declaration owner remains unresolved.

### Step 2: Extend host declarations and author the host roster

- Task IDs: `TASK-568`
- Objective: add static denials to `HostConfigKey`/DSL and the aligned speed table, then author all host/runtime roster entries.
- Precondition: Step 1 blast-radius inventory is complete.
- Postcondition: host and speed declarations produce exactly AC-1's policies and all affected literals compile.
- Files allowed to read, with ranges when over 300 lines: named carrier/macro/row sections and focused host-row tests only.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/resolved_config.rs`; `crates/slicer-ir/src/feedrate.rs`; `crates/slicer-config/tests/scope_eligibility_tdd.rs`.
- Files explicitly out of bounds: manifests, resolver, scheduler legacy fields, WIT.
- Blast-radius discipline: every Step-1 `HostConfigKey` literal/test assertion is owned here; if more than three files are required, split into adjacent ≤3-file substeps before activation.
- Expected sub-agent dispatches: focused compile/test `FACT`.
- Context cost: `M`
- Authoritative docs: ADR-0069 and plan's initial-denials paragraph.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-config --all-targets --test scope_eligibility_tdd authored_denial_roster_is_exact_across_all_declarers -- --exact` (host half may remain red only for not-yet-authored module declarers until Step 3; a dedicated host assertion must be green).
- Exit condition: speed/meta/denial tables drift in length, a host roster key is absent, or denial order differs.

### Step 3: Hand-author module schema denials

- Task IDs: `TASK-568`
- Objective: add exact AC-1 denials to every module declarer, including all five `nozzle_diameter` declarations.
- Precondition: Step 2 host declarations compile and Step 1 declarer inventory is complete.
- Postcondition: every declarer is explicit and aggregate registry union equals the roster.
- Files allowed to read, with ranges when over 300 lines: only inventoried `[config.schema.<key>]` tables.
- Files allowed to edit (at most 3): process module TOML files in deterministic alphabetical ≤3-file substeps; each substep lists its exact files before editing.
- Files explicitly out of bounds: guest Rust, unrelated schema entries, legacy section deletion (Step 6).
- Blast-radius discipline: not applicable; no Rust struct field added.
- Expected sub-agent dispatches: per-batch manifest parse result `FACT`.
- Context cost: `M`
- Authoritative docs: ADR-0069 union/authorship policy and AC-1 exact roster.
- OrcaSlicer refs: none.
- Verification: AC-1 exact test after final batch.
- Exit condition: aggregate union passes while any individual declarer lacks its authored policy.

### Step 4: Add independent mechanical derivation and drift control

- Task IDs: `TASK-568`
- Objective: derive unreachable sub-print pairs mechanically and compare them to declarations without generating runtime policy.
- Precondition: Steps 2–3 author the full roster.
- Postcondition: AC-2 passes and a negative control proves one removed authored denial creates a mismatch.
- Files allowed to read, with ranges when over 300 lines: registry census helpers, host channel metadata, manifest lifecycle/granularity fields, packet-05 delivery matrix.
- Files allowed to edit (at most 3): `crates/slicer-config/tests/scope_eligibility_tdd.rs`; optional test helper module; its test-target manifest only if required.
- Files explicitly out of bounds: production declaration generation, source-grep assertions, guest implementation.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: reachability fact checks `FACT` ≤5 lines each; focused test `FACT`.
- Context cost: `M`
- Authoritative docs: plan mechanical-derivation direction and `docs/22_test_quality.md` false-green rules.
- OrcaSlicer refs: none.
- Verification: AC-2 exact test plus its mismatch negative control.
- Exit condition: expected output is copied from the authored roster, runtime policy is generated, or the negative control stays green.

### Step 5: Enforce registry-derived admission in the resolver

- Task IDs: `TASK-568`
- Objective: expose admission sets and reject denied deltas atomically before packet-05 merge/expansion.
- Precondition: roster and drift tests pass; packet-05 final module is reconciled.
- Postcondition: allowed values resolve and denied pairs return exact `ScopeDenied` errors without partial output.
- Files allowed to read, with ranges when over 300 lines: registry entry/accessor and packet-05 resolver/error definitions.
- Files allowed to edit (at most 3): `crates/slicer-config/src/lib.rs`; packet-05's landed resolution source file; `crates/slicer-config/tests/scope_eligibility_tdd.rs`.
- Files explicitly out of bounds: scheduler duplicate admission rosters, layer-range ingestion, modifier typing.
- Blast-radius discipline: inventory all exhaustive `ResolutionError` matches before adding `ScopeDenied`; include them here or split bounded substeps.
- Expected sub-agent dispatches: error match locations `LOCATIONS` ≤20; focused tests `FACT`.
- Context cost: `M`
- Authoritative docs: plan Resolution and ADR-0069.
- OrcaSlicer refs: none.
- Verification: AC-3, AC-4, AC-N1, and AC-N2 exact tests.
- Exit condition: validation occurs after merge, any caller uses a separate object key roster, or a denied input yields partial state.

### Step 6: Retire both legacy manifest sections

- Task IDs: `TASK-568`
- Objective: remove the model/builder/parser/test surface and both sections from all 24 manifests.
- Precondition: registry-derived admission is operational.
- Postcondition: no production/test/TOML occurrence remains and manifest ingestion succeeds without either section.
- Files allowed to read, with ranges when over 300 lines: inventoried `LoadedModule`, builder, ingestion, parse-test symbols and section blocks only.
- Files allowed to edit (at most 3): first substep `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`; subsequent alphabetical manifest batches of at most three files.
- Files explicitly out of bounds: per-field denials, unrelated manifest content, module Rust.
- Blast-radius discipline: all builder struct/method call sites inventoried in Step 1 are included in bounded substeps; do not retain deprecated no-op methods.
- Expected sub-agent dispatches: after each batch, static occurrence count `FACT`; final scheduler test `FACT`.
- Context cost: `M`
- Authoritative docs: ADR-0069 replacement contract and plan RC-9.
- OrcaSlicer refs: none.
- Verification: AC-5 command; `cargo test -p slicer-scheduler --all-targets --test scheduler_integration manifest_ingestion_tdd`.
- Exit condition: either old name remains, parser merely makes it optional, or the populated wave-overhangs lists survive as authority.

### Step 7: Update docs and close gates

- Task IDs: `TASK-568`
- Objective: document sole per-key eligibility/admission and run closure gates.
- Precondition: Steps 1–6 pass.
- Postcondition: AC-6 and all packet gates pass.
- Files allowed to read, with ranges when over 300 lines: relevant manifest/config-resolution sections only.
- Files allowed to edit (at most 3): `docs/03_wit_and_manifest.md`; `docs/04_host_scheduler.md`.
- Files explicitly out of bounds: plan, ADR decision text, other packets, unrelated docs.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: doc and cargo gates each return `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs: ADR-0069 and approved plan.
- OrcaSlicer refs: none.
- Verification: AC-6; all-target check/clippy; literals; test-quality report; guest freshness.
- Exit condition: docs retain dual-authority language, any gate fails, or a version bump appears.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | inventories |
| 2 | M | host carrier blast radius |
| 3 | M | manifest roster batches |
| 4 | M | independent derivation |
| 5 | M | resolver enforcement |
| 6 | M | 24-manifest retirement batches |
| 7 | S | docs/gates |

## Packet Completion Gate

- All steps and exits complete and every AC command passes.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `cargo xtask build-guests --check` exits 0; all-target check/clippy and quality gates pass.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record any consciously rejected mechanical proposals and their rationale.
- Confirm context stayed within the standard band or record the required swarm escalation/lesson.
