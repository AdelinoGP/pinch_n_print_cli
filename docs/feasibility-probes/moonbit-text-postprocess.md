WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: moon 0.1.20260807; moonc v0.10.7; wit-bindgen 0.60.0; wasm-tools 1.250.0; cargo test workspace toolchain
GENERATION_COMMANDS: wit-bindgen moonbit wit/deps/config.wit wit/deps/types.wit wit/deps/ir-types.wit wit/deps/common.wit wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit --out-dir moonbit --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module
BUILD_COMMANDS: moon fmt; moon build --target wasm --release; wasm-tools component embed --encoding utf16 -w slicer:postpass-text-postprocess/text-postprocess-module; wasm-tools component new
COMPONENT_SHA256: 46e1583f29b9d0dc99ca9dab4151e028f3b81ad6eb632e377bbe8f595b529fd5
HOST_COMMAND: PNP_FOREIGN_COMPONENT=<component.wasm> cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: FAILED; foreign component failed to run: error while executing at wasm backtrace (gen wasm functions 36 and 30)
FAILURE_DETAIL: Host oracle exited 101 with a guest execution failure before exact output comparison.
RESULT: NOT_LOADABLE_OR_CORRECT
WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: wit-bindgen-cli 0.60.0; moon 0.1.20260807 (4da23f8); moonc v0.10.7; wasm-tools 1.250.0
GENERATION_COMMANDS: wit-bindgen moonbit wit/deps/config/config.wit wit/deps/types/types.wit wit/deps/ir-types/ir-types.wit wit/deps/common/common.wit wit/postpass-text-postprocess.wit --out-dir moonbit --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module; then cp main.mbt gen/interface/slicer/postpass-text-postprocess/text-postprocess/main.mbt (the world export is forward-declared as `declare pub fn run` in that interface package and its definition must live in the same package; the previous run copied main.mbt into gen/, leaving the declare unsatisfied so moonc emitted a trapping stub -- that was the earlier "gen wasm function 36/28" trap)
BUILD_COMMANDS: moon fmt; moon build --target wasm --release; wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit _build/wasm/release/build/gen/gen.wasm -o embedded.wasm; wasm-tools component new -o comp.wasm embedded.wasm (all via moonbit/build.sh; note moon fmt auto-converts moon.mod.json to moon.mod under feature rr_moon_mod)
COMPONENT_SHA256: 51b06b5f47445059c4c27ca55cac36de30c3773e4691548c5b48ef5620341512 (build is not bit-reproducible; a second clean build hashed differently but this is the hash of the component the oracle verified)
HOST_COMMAND: PNP_FOREIGN_COMPONENT=docs/feasibility-probes/foreign-language-text-postprocess/moonbit/comp.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out
FAILURE_DETAIL: none; verified twice, including once from a fully cleaned scratch tree (rm -rf _build gen interface world) via moonbit/build.sh
RESULT: LOADABLE_AND_CORRECT
