#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"
test -n "${WIT_BINDGEN_HEAD:-$(awk '{print $1}' ../.generation-started 2>/dev/null || true)}" || { echo 'BLOCKED: FORK_NOT_READY generation-marker'; exit 43; }
wit-bindgen assemblyscript ./../wit --out-dir bindings
asc bindings.ts --target release --outFile core.wasm --runtime incremental
wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit core.wasm -o embedded.wasm
wasm-tools component new -o comp.wasm embedded.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
