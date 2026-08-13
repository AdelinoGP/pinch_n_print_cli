WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: moon 0.1.20260807; moonc v0.10.7; wit-bindgen 0.60.0; wasm-tools 1.250.0; cargo test workspace toolchain
GENERATION_COMMANDS: wit-bindgen moonbit wit/deps/config.wit wit/deps/types.wit wit/deps/ir-types.wit wit/deps/common.wit wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit --out-dir moonbit --derive-show --derive-eq --derive-error -w slicer:postpass-text-postprocess/text-postprocess-module
BUILD_COMMANDS: moon fmt; moon build --target wasm --release; wasm-tools component embed --encoding utf16 -w slicer:postpass-text-postprocess/text-postprocess-module; wasm-tools component new
COMPONENT_SHA256: 46e1583f29b9d0dc99ca9dab4151e028f3b81ad6eb632e377bbe8f595b529fd5
HOST_COMMAND: PNP_FOREIGN_COMPONENT=<component.wasm> cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: FAILED; foreign component failed to run: error while executing at wasm backtrace (gen wasm functions 36 and 30)
FAILURE_DETAIL: Host oracle exited 101 with a guest execution failure before exact output comparison.
RESULT: NOT_LOADABLE_OR_CORRECT
