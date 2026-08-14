# AssemblyScript — Foreign-Language Text Postprocess Probe

## Packet 225a — accommodating host, fork-gated generation (2026-08-13)

Fork gate passed immediately before generation (`check-fork-readiness.sh --confirmed`):

WIT_BINDGEN_HEAD: 7942297b75fe36e5bfaa09b82f697ae421d951fc
WIT_BINDGEN_BRANCH: feat/assemblyscript-no-async
WIT_BINDGEN_STATUS: clean

WIT_WORLD: slicer:postpass-text-postprocess/text-postprocess-module (package `slicer:postpass-text-postprocess@1.0.0`, export `run`)

TOOL_VERSIONS: wit-bindgen-cli 0.60.0 fork (`D:\wit-bindgen`, branch feat/assemblyscript-no-async, HEAD 7942297b75fe36e5bfaa09b82f697ae421d951fc; version string reports base rev b6c9ec127 2026-08-11; rebuilt from HEAD sources this session with `cargo build -p wit-bindgen-cli`); asc 0.28.20; node v26.2.0; wasm-tools 1.250.0; Python 3.14.4; host workspace wasmtime 47.0.3 / wit-bindgen 0.60.0.

GENERATION_COMMANDS: `cargo build -p wit-bindgen-cli` (in `D:\wit-bindgen`); `bash check-fork-readiness.sh --confirmed` (wrote `.generation-started`: `7942297b75fe36e5bfaa09b82f697ae421d951fc 2026-08-14T00:57:13Z`); `wit-bindgen assemblyscript ./../wit --out-dir bindings` (fork CLI, run by `assemblyscript/build.sh`).

BUILD_COMMANDS: via `docs/feasibility-probes/foreign-language-text-postprocess/assemblyscript/build.sh` (fork CLI on PATH): splice probe `run` implementation into the generated export stub and drop `exportStart` from the generated asconfig (Python inline steps in build.sh); `asc bindings.ts --target release` (inside `bindings/`, picks up generated asconfig.json); post-compile wasm export rename per `wit_bindgen_exports.json` (`wasm-tools print` → text rename `__exp_6_run` → `slicer:postpass-text-postprocess/text-postprocess@1.0.0#run` → `wasm-tools parse -o core.wasm`); `wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit core.wasm -o embedded.wasm` (UTF-16 string encoding, required — AssemblyScript strings are native UTF-16); `wasm-tools component new -o comp.wasm embedded.wasm`.

COMPONENT_SHA256: 1f3dc321e1b5d6dacd830000f4d11f0489bf9c9a5cac8f36b501c226a2340953

HOST_COMMAND: `PNP_FOREIGN_COMPONENT=F:\slicerProject\pinch_n_print_cli_2\docs\feasibility-probes\foreign-language-text-postprocess\assemblyscript\comp.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact` (from repo root; output tee'd to `target/test-output-as.log`)

HOST_OUTPUT: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.04s` — full lines:
```
test foreign_language_feasibility_tdd::foreign_language_text_postprocess_component ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.04s
```

FAILURE_DETAIL: none for the final component. Two fixture defects were found and fixed in `assemblyscript/build.sh` during the run (documented for reproducibility): (1) `asc` was invoked as `asc bindings.ts` from the fixture root where the file does not exist, and regeneration clobbered any hand-edited implementation — build.sh now splices the implementation post-generation and compiles inside `bindings/`; (2) the generated asconfig exports runtime init as `_start`, which nothing calls in a reactor component, so the itcms GC trapped via `ffi/abort` inside `~lib/rt/itcms/visitRoots` on the first allocation (`String.UTF16.decodeUnsafe` in `__exp_6_run`) — build.sh drops `exportStart` so asc emits a core-module start section that runs at instantiation. After both fixes the oracle passed on the first run.

RESULT: LOADABLE_AND_CORRECT
