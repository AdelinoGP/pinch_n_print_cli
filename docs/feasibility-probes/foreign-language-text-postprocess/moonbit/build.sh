#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/.."
rm -rf moonbit/gen moonbit/interface moonbit/world
wit-bindgen moonbit wit/deps/config/config.wit wit/deps/types/types.wit wit/deps/ir-types/ir-types.wit wit/deps/common/common.wit wit/postpass-text-postprocess.wit --out-dir moonbit --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module
cd moonbit
# `run` is forward-declared (`declare pub fn run`) in the exported interface
# package; the definition must live in that same package, NOT in gen/.
cp -f main.mbt gen/interface/slicer/postpass-text-postprocess/text-postprocess/main.mbt
MOON_BIN="$(command -v moon || true)"
if [ -z "$MOON_BIN" ] && [ -x "$HOME/.moon/bin/moon" ]; then
  MOON_BIN="$HOME/.moon/bin/moon"
fi
"$MOON_BIN" fmt
"$MOON_BIN" build --target wasm --release
wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit _build/wasm/release/build/gen/gen.wasm -o embedded.wasm
wasm-tools component new -o comp.wasm embedded.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
