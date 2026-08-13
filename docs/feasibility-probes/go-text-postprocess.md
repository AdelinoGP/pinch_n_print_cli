WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module
TOOL_VERSIONS: go 1.26.5; wit-bindgen-go v0.7.0; wasm-tools 1.250.0; python3 3.14.4
GENERATION_COMMANDS: PATH="$(go env GOPATH)/bin:$PATH" wit-bindgen-go generate ../wit -w slicer:postpass-text-postprocess/text-postprocess-module
BUILD_COMMANDS: GOOS=wasip1 GOARCH=wasm go build -buildmode=c-shared -ldflags=-checklinkname=0 -o core.wasm .; wasm-tools component new --adapt "wasi_snapshot_preview1=$HOME/wasi-adapters/wasi_snapshot_preview1.reactor.wasm" -o comp.wasm core.wasm
COMPONENT_SHA256: 2fc5cc87af01ffecbf35a4c429833a2930e3abdba7556186166c85e41fde4510
HOST_COMMAND: PNP_FOREIGN_COMPONENT=F:/slicerProject/pinch_n_print_cli_2/docs/feasibility-probes/foreign-language-text-postprocess/go/comp.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact
HOST_OUTPUT: foreign component failed to instantiate: component imports instance `wasi:cli/environment@0.2.12`, but a matching implementation was not found in the linker
FAILURE_DETAIL: Go's WASI reactor adapter produces a component importing wasi:cli/environment@0.2.12; the slicer-only host linker does not provide that WASI instance, so instantiation fails before the oracle can execute.
RESULT: NOT_LOADABLE_OR_CORRECT
