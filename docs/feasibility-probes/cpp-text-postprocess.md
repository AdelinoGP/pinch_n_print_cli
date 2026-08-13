WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: wit-bindgen-cli 0.60.0; WASI SDK clang++ clang version 23.1.0git; wasm-tools 1.250.0; wasmtime host workspace
WIT_BINDGEN_SOURCE: released wit-bindgen-cli 0.60.0
GENERATION_COMMANDS: wit-bindgen cpp ../wit --out-dir bindings -w text-postprocess-module
BUILD_COMMANDS: C:/wasi-sdk/bin/clang++.exe -I bindings -fno-exceptions -std=c++23 -c main.cpp -o main.o; C:/wasi-sdk/bin/clang++.exe -I bindings -fno-exceptions -std=c++23 -mexec-model=reactor main.o bindings/text_postprocess_module.cpp bindings/text_postprocess_module_component_type.o -o core.wasm; wasm-tools component new --adapt wasi_snapshot_preview1=$HOME/wasi-adapters/wasi_snapshot_preview1.reactor.wasm -o comp.wasm core.wasm; wasm-tools component wit comp.wasm (imports wasi:io/error@0.2.12 and wasi:cli/* interfaces)
COMPONENT_SHA256: d7694886a6e2e20d2c88a4f8f99b849a117e330c05d7c5a2a3d89296cb125131
HOST_COMMAND: PNP_FOREIGN_COMPONENT=F:/slicerProject/pinch_n_print_cli_2/docs/feasibility-probes/foreign-language-text-postprocess/cpp/comp.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: test foreign_language_feasibility_tdd::foreign_language_text_postprocess_component ... FAILED; foreign component failed to instantiate: component imports instance `wasi:io/error@0.2.12`, but a matching implementation was not found in the linker
FAILURE_DETAIL: The preview1 adapter produced a real component, but the slicer-only host cannot instantiate its wasi:io/error@0.2.12 import (component wit also lists wasi:io/streams@0.2.12, wasi:cli/*, wasi:clocks/*, and wasi:filesystem/* imports).
RESULT: NOT_LOADABLE_OR_CORRECT

WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: wit-bindgen-cli 0.60.0; WASI SDK clang++ clang version 23.1.0git (LLVM commit 278c31bfb8ceb7ea17dbfd11a4fb21e6634af957); wasm-tools 1.250.0; wasmtime host workspace with default-deny WASI preview2
WIT_BINDGEN_SOURCE: released wit-bindgen-cli 0.60.0
GENERATION_COMMANDS: wit-bindgen cpp ../wit --out-dir bindings -w text-postprocess-module
BUILD_COMMANDS: rm -f bindings/* main.o core.wasm comp.wasm embedded.wasm; C:/wasi-sdk/bin/clang++.exe -I bindings -fno-exceptions -std=c++23 -c main.cpp -o main.o; C:/wasi-sdk/bin/clang++.exe -I bindings -fno-exceptions -std=c++23 -mexec-model=reactor main.o bindings/text_postprocess_module.cpp bindings/text_postprocess_module_component_type.o -o core.wasm; wasm-tools component new --adapt wasi_snapshot_preview1=$HOME/wasi-adapters/wasi_snapshot_preview1.reactor.wasm -o comp.wasm core.wasm
COMPONENT_SHA256: 2abdaac1638fa9ec2b740f7d8e1526d60571daf2794c84a51209f414b52bf483
HOST_COMMAND: PNP_FOREIGN_COMPONENT=F:/slicerProject/pinch_n_print_cli_2/docs/feasibility-probes/foreign-language-text-postprocess/cpp/comp.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: test foreign_language_feasibility_tdd::foreign_language_text_postprocess_component ... ok; test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out
FAILURE_DETAIL: none; the corrected component loaded and transformed the probe input to the expected output
RESULT: LOADABLE_AND_CORRECT
