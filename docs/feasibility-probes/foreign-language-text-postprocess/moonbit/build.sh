#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/.."
wit-bindgen moonbit wit/deps/config/config.wit wit/deps/types/types.wit wit/deps/ir-types/ir-types.wit wit/deps/common/common.wit wit/postpass-text-postprocess.wit --out-dir moonbit --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module
cd moonbit
cp -f main.mbt gen/main.mbt
sed -i '/config-types/d;/module-errors/d' gen/moon.pkg
sed -i '4i\  "slicer/postpass-text-postprocess/interface/slicer/config/config-types",\n  "slicer/postpass-text-postprocess/interface/slicer/common/module-errors",' gen/moon.pkg
MOON_BIN="$(command -v moon || true)"
if [ -z "$MOON_BIN" ] && [ -x "$HOME/.moon/bin/moon" ]; then
  MOON_BIN="$HOME/.moon/bin/moon"
fi
"$MOON_BIN" fmt
"$MOON_BIN" build --target wasm --release
wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit _build/wasm/release/build/gen/gen.wasm -o embedded.wasm
wasm-tools component new -o comp.wasm embedded.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
