# Host WASI accommodation for foreign-language guests

The 225 feasibility gate measured candidates against the slicer-only linker (no WASI), which confounded the measurement: Go and C++ failed because their toolchains always link WASI, not because the languages cannot produce working components. The gate's purpose — and PNP's goal — is to allow modules to be authored in other languages, so the host will be extended with WASI preview2 (wasmtime-wasi, default-deny capabilities) in a follow-up packet, and the gate re-measures candidates (Go, C++, a MoonBit retry, AssemblyScript) against the accommodating host. Packet 225 stays open with its records as production-fit evidence; the final verdict is deferred to the follow-up packet.

**Status:** accepted

**Considered Options:**

- Keep the slicer-only linker and treat WASI-importing guests as honest candidate failures (packet 225's original design). Rejected: it measures the host's constraints rather than the languages, and contradicts the goal of allowing foreign-language modules.
- Amend packet 225 to add WASI support mid-run. Rejected: a scope change on a run whose toolchain-bump work is already complete and independent; the accommodation is a coherent unit of its own.

**Consequences:**

- `slicer-wasm-host` gains wasmtime-wasi; the "slicer-only" sandbox becomes "slicer interfaces + WASI with default-deny capabilities" (no preopens, no env, no args, no network) — the security boundary is preserved in substance.
- Packet 225's AC-7 verdict is deferred; `docs/14` receives exactly one verdict line, from the follow-up packet.
- The production-fit records (packet 225's, plus historical `go-wasm.md`/`moonbit-wasm.md`) remain as measured evidence of production-fit; the follow-up's re-measurements supersede them for the gate's selection only.
