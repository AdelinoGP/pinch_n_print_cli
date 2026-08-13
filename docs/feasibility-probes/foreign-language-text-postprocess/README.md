# Foreign-Language Text Postprocess Probe

This fixture measures whether each candidate can load and return the same WIT component under the wasmtime 47.0.3 / wit-bindgen 0.60.0 workspace toolchain. The copied WIT snapshot in `wit/` is the complete dependency closure and must remain unchanged. Generated bindings and binaries are scratch output only.

## Contract

- World: `slicer:postpass-text-postprocess/text-postprocess-module` (package `slicer:postpass-text-postprocess@1.0.0`, world `text-postprocess-module`, export `run`)
- Input: `; probe input\n`
- Expected output: `;; foreign-language-probe\n; probe input\n`

Oracle text values (shown as actual line breaks):

    ; probe input

    ;; foreign-language-probe
    ; probe input

;; foreign-language-probe
; probe input

The displayed `\n` sequences are literal backslash-n characters. The oracle checks the exact output. Candidates report `LOADABLE_AND_CORRECT` or `NOT_LOADABLE_OR_CORRECT`. Missing prerequisites are transient: `BLOCKED: TOOLCHAIN <candidate> <command>`.

## Prerequisites and Installation

Run `check-prerequisites.sh [candidate]` before building. A missing tool stops the probe: ask the user to install it, quote the matching INSTALL and VERIFY instructions below, and do not record a candidate failure.

MoonBit: INSTALL `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser; irm https://cli.moonbitlang.com/install/powershell.ps1 | iex`; VERIFY `moon version`. Official source: https://moonbitlang.com/download/

AssemblyScript: install Node/npm/npx from the MSI at https://nodejs.org/en/download and VERIFY `node --version`; INSTALL `npm install -g assemblyscript`; VERIFY `asc --version`. Official source: https://www.assemblyscript.org/

C++: download `wasi-sdk-33.0-x86_64-windows.tar.gz` from https://github.com/WebAssembly/wasi-sdk/releases, extract it, and use `bin/clang++.exe` (default target wasm32-wasip1). VERIFY `bin/clang++ --version`. `wit-bindgen cpp` needs `--wasi-sdk-path` or `WASI_SDK_PATH`.

Go: install the MSI from https://go.dev/dl/; VERIFY `go version`; INSTALL `go install go.bytecodealliance.org/cmd/wit-bindgen-go@latest`; VERIFY `wit-bindgen-go --version`. Official source: https://github.com/bytecodealliance/go-modules

Shared: INSTALL `cargo install --locked wasm-tools`; VERIFY `wasm-tools --version`; install Python from https://www.python.org/downloads/ and VERIFY `python --version`.

The gate checks PATH first, then `$HOME/.moon/bin/moon` and `$WASI_SDK_PATH/bin/clang++` / `C:/wasi-sdk/bin/clang++.exe`.

## Fork Gate

Before AssemblyScript generation, confirm that `D:/wit-bindgen` is the intended fork, on branch `feat/assemblyscript-backend`, and clean. Run `check-fork-readiness.sh --confirmed` (or set `FORK_CONFIRMED=1`). Otherwise it prints `BLOCKED: FORK_NOT_READY <reason>` and stops. It writes `.generation-started` only after all checks pass; the marker captures HEAD and time. `--simulate-dirty` exercises the dirty stop path.

## Scratch Builds and Oracle

Run each `build.sh` from its language directory after the prerequisite gate. Each script generates bindings, compiles, embeds, componentizes, then invokes the shared host oracle with `PNP_FOREIGN_COMPONENT=comp.wasm`. The oracle returns `LOADABLE_AND_CORRECT` only when instantiation succeeds, imports resolve, and output equals the contract; otherwise `NOT_LOADABLE_OR_CORRECT`.

MoonBit (`moonbit/build.sh`): `wit-bindgen moonbit wit --out-dir out --derive-show --derive-eq --derive-error`; `moon fmt`; `moon build --target wasm --release`; `wasm-tools component embed --encoding utf16`; then `wasm-tools component new`. The exact MoonBit generator binary is resolved at probe time and assumed to support this world.

AssemblyScript (`assemblyscript/build.sh`): after the fork gate, `wit-bindgen assemblyscript ./wit --out-dir bindings`; implement generated exports using `index.ts`; `asc bindings.ts --target release --outFile core.wasm --runtime incremental`; `wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit core.wasm -o embedded.wasm`; then `wasm-tools component new -o comp.wasm embedded.wasm`. UTF-16 is required. The script reads `.generation-started` or `WIT_BINDGEN_HEAD`.

C++ (`cpp/build.sh`): `wit-bindgen cpp ./wit --out-dir bindings -w text-postprocess-module`; compile with WASI `clang++` using `-I bindings -fno-exceptions -std=c++20 -c`; link with `-mexec-model=reactor`; then `wasm-tools component new`.

Go (`go/build.sh`): run `wit-bindgen-go generate`; set `GOOS=wasip1 GOARCH=wasm` and run `go build -buildmode=c-shared -ldflags=-checklinkname=0`; embed the world and componentize with the `wasi_snapshot_preview1.reactor.wasm` adapter. Any unresolved `wasi:*` import is an honest candidate failure.

No generated output is committed. Result vocabulary is exactly `LOADABLE_AND_CORRECT`, `NOT_LOADABLE_OR_CORRECT`, or transient `BLOCKED: TOOLCHAIN <candidate> <command>` / `BLOCKED: FORK_NOT_READY`.
