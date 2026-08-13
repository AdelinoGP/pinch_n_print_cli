# Requirements: 225-dragon-curve-feasibility-gate

## Packet Metadata

- Grouped task IDs: `TASK-336`
- Backlog source: `docs/specs/community-modules-dragon-curve-plan.md` (the `docs/07_implementation_status.md` rows are intentionally created later by packet 228)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The existing gate considered only historical Go and MoonBit probes. That is insufficient after discovery of an experimental AssemblyScript backend in `D:\wit-bindgen`, and it leaves C++'s documented component support unmeasured. Candidate comparisons also used different worlds and host harnesses. This packet creates one fair, deterministic feasibility oracle: every candidate must satisfy the existing `text-postprocess-module` component contract, instantiate in the production slicer-only linker, and return one exact string. Toolchain absence blocks measurement and requires user action; it is not evidence against a language.

## In Scope

- Bump root pins to wasmtime 47.0.3 and wit-bindgen 0.60.0, retain `call-hook`, sweep stale inline pins, absorb compile fallout, and rebuild/check guest WASM.
- Reconcile current-toolchain version statements in `docs/00_project_overview.md`, `docs/05_module_sdk.md`, and `docs/14_submodule_programming_languages.md` with the landed pins; preserve historical probe version records as historical evidence.
- Add a shared probe fixture under `docs/feasibility-probes/foreign-language-text-postprocess/` containing the canonical WIT dependency tree copied from `crates/slicer-schema/wit`, a README with exact per-language prerequisites/build commands, executable `check-prerequisites.sh` and `check-fork-readiness.sh` gates (including deterministic simulation modes for negative tests), and minimal handwritten MoonBit, AssemblyScript, C++, and Go implementations.
- Add `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs` and register it in `tests/integration/main.rs`. The ignored test reads `PNP_FOREIGN_COMPONENT`, uses the existing production `TextPostprocessModule` linker/dispatch seam, calls `run-text-postprocess` with `; probe input\n`, and requires exactly `;; foreign-language-probe\n; probe input\n`.
- Use the same checked-in WIT snapshot and ignored integration driver for every candidate. A component is successful only when generation, compilation, componentization, slicer-only instantiation, invocation, and exact output assertion all pass.
- Re-run MoonBit rather than inheriting its old verdict. MoonBit is highest priority because it emits a bare core Wasm module without WASI. Its absent toolchain is a blocker requiring a user install request.
- Generate AssemblyScript bindings using the read-only latest committed HEAD of `D:\wit-bindgen` on `feat/assemblyscript-backend`. Resolve the HEAD only after the user confirms the concurrent async-support work is committed and `git status --porcelain` is empty; record that SHA and clean status immediately before generation. Use UTF-16 embedding unless the latest backend's own docs/tests prove a different required encoding.
- Generate C++-17+ bindings from that same captured clean HEAD, compile for `wasm32-wasip1` with WASI SDK clang++, componentize, and test without adding WASI to the host. Any unresolved `wasi:*` import is an honest candidate failure.
- Re-run Go last against the same text-postprocess world and oracle, replacing the prior infill-world-only comparison for selection purposes.
- Add four evidence records: `moonbit-text-postprocess.md`, `assemblyscript-text-postprocess.md`, `cpp-text-postprocess.md`, and `go-text-postprocess.md`. Each records commands, versions, fork provenance when applicable, component SHA-256, load/invocation output, and exactly one terminal `RESULT:`.
- Select the first `LOADABLE_AND_CORRECT` candidate in this fixed order: MoonBit, AssemblyScript, C++, Go. Select Rust only when all four records say `NOT_LOADABLE_OR_CORRECT`.
- Before each candidate step, check all required commands. If one is missing, stop, tell the user exactly which tool is missing, provide installation and version-verification instructions from the shared README, and wait. Do not install tools, skip the candidate, write a result, or select a lower-priority language.
- Before AssemblyScript generation, require explicit user confirmation that the concurrent fork work has been committed, branch `feat/assemblyscript-backend`, and an empty `git status --porcelain`. If any condition fails, stop and ask; never stash, clean, switch, pull, reset, or use uncommitted fork changes. Capture the resulting HEAD once and use it for both AssemblyScript and C++ probes.

## Out of Scope

- Dragon Curve or Hilbert generation, geometry output, performance comparison, and OrcaSlicer behavioral parity.
- Adding WASI interfaces or adapters to the production slicer linker.
- Modifying, stashing, cleaning, switching, pulling, resetting, committing, vendoring, or path-linking `D:\wit-bindgen` into the workspace.
- Treating a missing toolchain as `NOT_LOADABLE_OR_CORRECT`.
- Shipping foreign-language SDK abstractions, changing WIT/schema versions, or implementing packet 226/227.
- Editing historical `go-wasm.md` and `moonbit-wasm.md`; the new same-world records supersede them only for this gate's selection.

## Authoritative References

- `docs/specs/community-modules-dragon-curve-plan.md`
- `docs/specs/community-modules-dragon-curve-infill.md` section 6
- `docs/14_submodule_programming_languages.md`
- `crates/slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit:7-19`
- `crates/slicer-wasm-host/src/host.rs:1048-1070`
- `crates/slicer-wasm-host/src/dispatch.rs:1691-1802`
- `crates/slicer-wasm-host/tests/integration/wasm_instance_tdd.rs:181`
- `D:\wit-bindgen\README.md:267-439`
- `D:\wit-bindgen\src\bin\wit-bindgen.rs:45-53,77-82,155-164`
- `D:\wit-bindgen\crates\test\src\assemblyscript.rs:90-149`
- `D:\wit-bindgen\crates\test\src\cpp.rs:29-195`

No OrcaSlicer reference applies. The closest documented C++ Hilbert implementation is deliberately excluded because this packet measures language/component feasibility, not infill parity.

## Acceptance Summary

- Positive: `AC-1` through `AC-9` in `packet.spec.md`.
- Negative: `AC-N1` rejects wrong output through the real driver; `AC-N2` enforces stop-and-request-install behavior; `AC-N3` rejects an unready fork before generation.
- Cross-step expectation: no final selection is legal until all four candidate records contain terminal results.

## Verification Matrix

| Surface | Narrow verification | Expected observation |
| --- | --- | --- |
| Root and inline pins | AC-1, AC-2 | exact new pins; zero stale pins |
| Workspace/guests | AC-3 | compile, clippy, rebuild, freshness all exit zero |
| Shared host oracle | AC-4, AC-N1 | production linker invokes exact string contract; wrong component fails |
| Probe completeness | AC-5 | four terminal results, versions, SHA-256; no tooling blocker remains |
| AssemblyScript provenance | AC-6 | runtime-resolved HEAD, branch, clean status, encoding, world |
| Selection | AC-7 | fixed first-success priority applied exactly |
| Fixture contract | AC-8 | one world/input/output shared by four candidates |
| Version documentation | AC-9 | three current-toolchain docs match landed pins |
| Missing tools | AC-N2 | exit 42, request user install, include install/verification instructions |
| Unready fork | AC-N3 | exit 43 before generation or evidence writes |

## Step Completion Expectations

- Toolchain bump and green guest freshness precede probe execution.
- The shared host oracle and fixture contract land before any candidate is measured.
- Candidate execution is serialized in priority order. A failed candidate may proceed to the next; a missing tool may not.
- Each candidate's evidence record is written immediately after its run and before the next candidate starts.
- Final docs selection is computed from the four records, never chosen from preference or historical evidence.

## Context Discipline Notes

- Delegate all cargo/build/test execution and return only exit status plus at most 20 relevant lines.
- Never read `crates/slicer-wasm-host/src/host.rs` or `dispatch.rs` in full; use the grounded ranges above.
- Treat `D:\wit-bindgen` as external read-only authority. After user confirmation, require clean branch state, capture HEAD immediately before AssemblyScript generation, and reuse the same SHA for C++.
- Generated bindings and component binaries are scratch outputs and are never committed.
