# Handoff: 225a-host-wasi-accomodation (session paused)

Session date: 2026-08-13. Run paused by user request. Resume with `/swarm implement docs/spec_packets/225a-host-wasi-accomodation/`.

## Run state

- Packet status: `draft` (implement mode, explicit user request; status flip to `implemented` deferred to closure).
- Steps 1-2 (host WASI + oracle): **DONE and committed**.
- Step 3 (MoonBit): **recorded NOT_LOADABLE_OR_CORRECT** (trap in generated wrapper) — but see the MoonBit re-investigation directive below; the user wants a mid-coder to check for anything being missed.
- Step 4 (C++): **LOADABLE_AND_CORRECT** after a fixture fix (dangling-pointer string return in main.cpp).
- Step 5 (Go): **BLOCKED** — generator/toolchain pairing incompatibility (details below). The record's 225a block is a fixture-defect artifact and must be replaced by a real measurement.
- Step 6 (AssemblyScript): **NOT RUN** — requires user confirmation + fork gate (`D:\wit-bindgen` at `feat/assemblyscript-backend`, clean) immediately before generation.
- Step 7 (verdict in docs/14): **NOT DONE** — needs all four terminal records.
- Acceptance ceremony: not run. AC-1..AC-5, AC-6 (rg parts), AC-N1 verified green during steps. AC-6's sh part = AC-N1 (passed). AC-N2 not yet run.

## Commits made this session (branch feature/community-modules)

| Commit | Content |
| --- | --- |
| `8c5d4372` | wit-bindgen 0.60.0 / wasmtime 47.0.3 bump across guests + docs version refs (root Cargo.toml/lock bump included here) |
| `b7fc2efc` | glossary terms (slicer-only linker, accommodating host, production-fit, language-feasibility, foreign-language guest) |
| `99a89fb7` | ADR-0060 host WASI accommodation |
| `a8fd0260` | ignored foreign-language oracle test |
| `e7a98b06` | probe fixture, gates, WIT (43 files) |
| `12565e30` | MoonBit / C++ / Go packet-225 probe records |
| `55a52262` | 225a spec packet |
| `c6126d3b` | default-deny WASI preview2 accommodation (wasmtime-wasi 47.0.3, WasiCtx in HostExecutionContext + HostState, WasiView impls, `add_wasi_to_linker` at all 15 production linker sites) |
| `de455912` | oracle test registers default-deny WASI preview2 |
| `674b7668` | **packet fix**: AC-6 verification regex `\|` → `|` (ripgrep treats `\|` as literal pipe; the check could never match) |
| `37de5e6c` | **fixture fix**: MoonBit build.sh WIT paths + out-dir, moon.mod name, main.mbt type paths |
| `d8027ec6` | MoonBit re-measurement record (NOT_LOADABLE_OR_CORRECT, trap in gen wrapper) |
| `6bc1d066` | **fixture fix**: C++ main.cpp returns owned string storage (was a dangling stack pointer) |
| `9510c99f` | C++ re-measurement record (LOADABLE_AND_CORRECT, SHA-256 2abdaac1...) |

## Dirty working tree (do not lose)

- `docs/feasibility-probes/foreign-language-text-postprocess/go/build.sh` — WIT embed step added (fixture repair, mid-flight)
- `docs/feasibility-probes/foreign-language-text-postprocess/go/go.mod` — `go.bytecodealliance.org/cm v0.3.0` added by `go mod tidy`
- `docs/feasibility-probes/foreign-language-text-postprocess/go/go.sum` — untracked, created by `go mod tidy`
- `docs/feasibility-probes/foreign-language-text-postprocess/go/main.go` — export wiring fix (imports generated textpostprocess package, assigns `textpostprocess.Exports.Run` returning `cm.OK(";; foreign-language-probe\n; probe input\n")`)
- `docs/feasibility-probes/go-text-postprocess.md` — 225a block currently says NOT_LOADABLE_OR_CORRECT "no exported instance"; that block is a **fixture-defect artifact** (main.go was never wired) and must be replaced by the real measurement once Go unblocks

## MoonBit — user directive (priority for next session)

**User's direction: dispatch a mid-coder on MoonBit to check if anything is being missed. MoonBit is the best candidate for "easy to be understood by a junior, fully functional as a wasm module", behind AssemblyScript, which is too novel to be recommended.**

Current MoonBit evidence (record `docs/feasibility-probes/moonbit-text-postprocess.md`, block 2):
- Component builds and componentizes with wit-bindgen-cli 0.60.0 + moon 0.1.20260807 (moonc v0.10.7), UTF-16 embed.
- Oracle traps at runtime in the generated gen wrapper: `0xb35 - slicer/postpass-text-postprocess/gen!<wasm function 36>`, `0xa6e - ... <wasm function 28>`.
- SHA-256 23d9207b1b0b5bc30fa3b3f5dfc1e02a983350a889226aa2295eb0af566bc182.

Investigated and RULED OUT:
1. **wit-bindgen/MoonBit ABI mismatch** (README `#moonbit-abi-compatibility`, 8-byte data-pointer offset change): wit-bindgen 0.60.0 already emits the NEW ABI (no offset); moonc 0.10.7 expects the new ABI. Pairing is correct.
2. **`--encoding utf16` embed flag**: trap reproduces identically with default UTF-8 embed. Not the cause.

Remaining lead (from the utf16 test): the generated wrapper's string handling — `mbt_ffi_ptr2str` copies raw canonical bytes via `moonbit.init_array16` with NO UTF-8 decoding, and `wasmExportRun` stores `payload.length()` (UTF-16 code units) as the returned string length. The canonical ABI boundary is UTF-8. This looks like a genuine wit-bindgen-MoonBit wrapper defect, but the user wants a mid-coder to double-check for anything missed (e.g., a fixture-side workaround, a `--ignore-stub` / stub.mbt angle, a different world wiring, or a MoonBit-side string conversion the fixture should call).

Fixture wrinkle to verify: `moonbit/build.sh` post-processes the generated `gen/moon.pkg` with `sed` (removes/re-adds config-types and module-errors deps with the full package prefix) and copies `main.mbt` into `gen/main.mbt`. One temp experiment reported wit-bindgen generating `gen/moon.pkg.json` while the script edits `gen/moon.pkg` — verify the script still works as-is in the workspace (it did in the w3b run).

If the mid-coder finds a fixture-side fix that makes the oracle pass, re-run the probe and rewrite record block 2 (keep packet-225 block 1). If the trap is confirmed genuine, the record stands.

## Go — blocker details

- Fixture: wit-bindgen-go v0.7.0 (only released version; `go list -m -versions` shows nothing newer; upstream tags stop at v0.7.0), go 1.26.5 (GOTOOLCHAIN=auto), `go.bytecodealliance.org/cm v0.3.0`.
- With the export wired, the generated import files compile and fail:
  `slicer/config/config-types/config-types.wasm.go:33:6: go:wasmimport: unsupported parameter type *cm.Option[string]`
  `slicer/config/config-types/config-types.wasm.go:37:6: go:wasmimport: unsupported parameter type *cm.List[string]`
- Tested: Go 1.26.5, 1.25.0, 1.24.0 all reject; Go 1.23 cannot do `-buildmode=c-shared` on wasip1. No shape aliases (OptionShape/ListShape) exist in cm v0.2.x/v0.3.0. No newer generator exists.
- Verdict from diagnosis: genuine generator-output/toolchain incompatibility, not a dependency mismatch. Packet-225's Go build "worked" only because main.go never imported the generated packages (dead code).
- Per the packet, this is a tooling blocker, NOT a language verdict: do not record Go as NOT_LOADABLE_OR_CORRECT on this basis. Options for the next session: (a) ask the user how to proceed (e.g., accept Go as blocked, patch the generated wasmimport signatures in the fixture, or vendor a workaround); (b) if a workaround is found (e.g., editing the generated config-types.wasm.go to use shape types), re-run the probe and record truthfully.

## AssemblyScript (Step 6) — pending, needs user interaction

- NOT started. Preconditions: Steps 1-2 green (yes); `check-prerequisites.sh` ready (verify); **user confirmation** of clean `D:\wit-bindgen` at `feat/assemblyscript-backend` immediately before generation; `check-fork-readiness.sh` must pass (AC-N2: `--simulate-dirty` exits 43, writes `BLOCKED: FORK_NOT_READY dirty`, no `GENERATION_COMMANDS:`).
- Record must include: 40-hex `WIT_BINDGEN_HEAD`, `WIT_BINDGEN_BRANCH: feat/assemblyscript-backend`, `WIT_BINDGEN_STATUS: clean`, UTF-16 embedding, world `slicer:postpass-text-postprocess/text-postprocess-module` (AC-8).
- Fixture: `assemblyscript/` (asconfig.json, build.sh, index.ts). The fork gate writes `.generation-started` capturing HEAD + time.

## Verdict logic (Step 7, AC-9)

- Winner = first `LOADABLE_AND_CORRECT` in fixed order **MoonBit, AssemblyScript, C++, Go**; else Rust.
- Currently C++ is the only LOADABLE_AND_CORRECT. If the MoonBit re-investigation flips MoonBit to LOADABLE_AND_CORRECT, MoonBit wins (user's expectation: MoonBit is the best junior-friendly candidate; AssemblyScript too novel to recommend).
- docs/14 must end with exactly one `**Dragon Curve authoring language: ...**` line.

## Packet-authoring notes

- AC-6 string fixed in packet.spec.md (escaped alternation) — committed in `674b7668`.
- The AC-7 python one-liner in packet.spec.md is valid; the per-candidate simplification used in some worker prompts had a syntax bug (`[assert ...]` in a comprehension) — use the packet's exact AC-7 command in the ceremony.
- docs/07 delta: none (packet 228 creates the TASK-336 row).

## Next-session dispatch order

1. Mid-coder on MoonBit (user directive) — investigate the wrapper string handling / anything missed; re-run probe if a fix is found.
2. Decide Go path with the user (blocked pairing).
3. Step 6 AssemblyScript (user confirmation + fork gate).
4. Step 7 verdict in docs/14.
5. Acceptance ceremony: AC-1..AC-9, AC-N1, AC-N2, packet-level gates (cargo check --workspace --all-targets, clippy -D warnings, host_services_tdd, wit_drift_detection_tdd), full spec-review, then status flip to `implemented` (user asked to implement; ceremony must be green).
