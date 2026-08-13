#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/.."
wit-bindgen moonbit wit/deps/config.wit wit/deps/types.wit wit/deps/ir-types.wit wit/deps/common.wit wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit --out-dir moonbit/out --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module
cd moonbit
moon fmt
moon build --target wasm --release
wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit target/wasm/release/build/foreign-language-text-postprocess.wasm -o embedded.wasm
wasm-tools component new -o comp.wasm embedded.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
