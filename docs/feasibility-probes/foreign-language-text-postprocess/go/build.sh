#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"
PATH="$(go env GOPATH)/bin:$PATH" wit-bindgen-go generate ../wit -w slicer:postpass-text-postprocess/text-postprocess-module
GOOS=wasip1 GOARCH=wasm go build -buildmode=c-shared -ldflags=-checklinkname=0 -o core.wasm .
WASI_ADAPTER="${WASI_ADAPTER:-$HOME/wasi-adapters/wasi_snapshot_preview1.reactor.wasm}"
[ -f "$WASI_ADAPTER" ] || { echo "BLOCKED: TOOLCHAIN go wasi_snapshot_preview1.reactor.wasm (missing; download from wasmtime v47.0.3 release assets)"; exit 42; }
wasm-tools component new --adapt "wasi_snapshot_preview1=$WASI_ADAPTER" -o comp.wasm core.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
# Any unresolved wasi:* import is an honest candidate failure.
