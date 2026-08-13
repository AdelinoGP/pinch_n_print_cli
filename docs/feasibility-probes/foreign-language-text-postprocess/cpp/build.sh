#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"
WASI_SDK_PATH=${WASI_SDK_PATH:-C:/wasi-sdk}
CLANGXX_BIN=${CLANGXX_BIN:-$WASI_SDK_PATH/bin/clang++}
WASI_ADAPTER=${WASI_ADAPTER:-$HOME/wasi-adapters/wasi_snapshot_preview1.reactor.wasm}
wit-bindgen cpp ../wit --out-dir bindings -w text-postprocess-module
"$CLANGXX_BIN" -I bindings -fno-exceptions -std=c++23 -c main.cpp -o main.o
"$CLANGXX_BIN" -I bindings -fno-exceptions -std=c++23 -mexec-model=reactor main.o bindings/text_postprocess_module.cpp bindings/text_postprocess_module_component_type.o -o core.wasm
wasm-tools component new --adapt wasi_snapshot_preview1="$WASI_ADAPTER" -o comp.wasm core.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
