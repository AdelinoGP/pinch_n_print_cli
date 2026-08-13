# Requirements: 225-dragon-curve-feasibility-gate

## Packet Metadata

- Grouped task IDs: `TASK-336`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The Dragon Curve community module's Go and MoonBit feasibility probes were measured on `wit-bindgen` 0.57.1 and `wasmtime` 43.x, and both returned "not loadable-and-correct" (Go: WASI preview2 import blocker; MoonBit: UTF-16/UTF-8 string-encoding mismatch). Spec §6 makes those verdicts contingent on the tested toolchain: a newer `wasmtime` may add WASI preview2 to the host linker (unblocking Go), and a newer `wit-bindgen` may add a UTF-16 host string-encoding option (unblocking MoonBit). This packet is the mandatory Step 0 gate: it updates the workspace, re-measures the Go blocker against the real slicer-only linker, and records a crisp yes/no verdict plus the chosen authoring language for packet 227 to consume.

## In Scope

- Bump `wasmtime` 43.0.0 → 47.0.3 (keep the `call-hook` feature) and `wit-bindgen` 0.57.1 → 0.60.0 in the workspace root `Cargo.toml` (lines 61–62), and sweep every `crates/**/Cargo.toml` and `modules/**/Cargo.toml` for stale pins. Grounded today: the only `wasmtime` dependents are `slicer-wasm-host` (direct bindgen + engine/Store/Linker/ResourceLimiter surface) and `slicer-runtime` (engine reuse, `workspace = true`). `wit-bindgen` is pinned inline at `"0.57.1"` in 24 manifests — the 21 `crates/slicer-wasm-host/test-guests/*/Cargo.toml` (4 of which are workspace members; the rest are built standalone by `cargo xtask build-guests`) plus `modules/core-modules/{machine-gcode-emit,part-cooling,overhang-classifier-default}/wit-guest/Cargo.toml` — and referenced via `wit-bindgen.workspace = true` in the sdk/macros-served module crates (those inherit the root bump automatically, no per-crate edit).
- Absorb the API fallout from both bumps. Likely-touched surfaces (verify against the tree at activation): `slicer-wasm-host`'s `wasmtime::component::bindgen!` and `add_to_linker`/`HasSelf` wiring, `Store`/`Engine` construction in `instance.rs`, `call_hook`, `get_fuel`, and the `ResourceLimiter` trait signature; any renamed feature flag or config default in wasmtime 47; and wit-bindgen 0.60 generated-shape drift in `slicer-sdk`'s `generate!` blocks, `slicer-macros`, and every guest.
- Prove the bump is safe with `cargo check --workspace --all-targets`, `cargo xtask build-guests && cargo xtask build-guests --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the narrowest targeted tests listed per step.
- Re-run the Go feasibility probe (spec §6 step 3) with the toolchain present on this machine (Go 1.26.5, wasm-tools 1.250.0, wasmtime crate 47.0.3), reproducing pnp_cli's exact Layer::Infill linker (slicer interfaces only, zero WASI). Procedure: wit-bindgen-go binding generation, `GOOS=wasip1 GOARCH=wasm` build, `wasm-tools component embed/new`, then the slicer-only linker instantiation check that previously failed with `imports wasi:cli/environment@0.2.6, no implementation in the linker`.
- Record the Go re-check verdict in `docs/14_submodule_programming_languages.md` §Community-module context (Go paragraph, MoonBit paragraph marked not re-run, and the section-intro paragraph) and append a dated re-check section to `docs/feasibility-probes/go-wasm.md`.
- Record the MoonBit re-check as "not re-run (toolchain absent)" — the `moon` binary is absent on this machine. Treat the Go verdict as the gate-deciding evidence; do not fabricate a MoonBit result.
- Decide and record the gate outcome exactly: either **PASS — Go component loadable-and-correct; authoring language = Go**, or **FALLBACK CONFIRMED — no non-Rust component loadable-and-correct; authoring language = Rust (Go tiling source retained as labeled reference only)**. This is the single yes/no verdict packet 227 consumes.
- Mark packet 227 as depending on this packet (in packet 227's own files; this packet only records it unblocks 227).

## Out of Scope

- Authoring the Dragon Curve module (`modules/community-modules/dragon-curve/`) — packet 227.
- The authored-coloring mechanism (`tool-index`, `fill_authored_coloring`, linker guard, DEV-135) — packet 226.
- Adding WASI preview2 to `slicer-wasm-host`'s linker, or any host-side workaround that would make a non-Rust component loadable. The gate's job is to measure, not to change the linker's WASI posture.
- MoonBit re-run, MoonBit toolchain installation, or any MoonBit source/artifact changes.
- Editing `docs/specs/community-modules-dragon-curve-infill.md` (may be archived; verdicts live in docs/14).

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-plan.md` — 102 lines; direct range read. Binding symbol contract + grounding facts.
- `docs/specs/community-modules-dragon-curve-infill.md` §6 — direct read; the gate's normative Step 0.
- `docs/feasibility-probes/go-wasm.md` — 197 lines; direct read (the probe brief and original evidence).
- `docs/feasibility-probes/moonbit-wasm.md` — 222 lines; delegated SUMMARY of §2/§8 only; never re-run here.
- `docs/14_submodule_programming_languages.md` — §Community-module context (lines 96–171); direct range read.

## Acceptance Summary

- Positive: `AC-1` through `AC-9`.
- Negative: `AC-N1`.
- Cross-packet impact: this packet's toolchain bump and guest rebuild are consumed by packet 226 (which must rebuild guests again for its WIT change) and packet 227 (which must not activate until this packet's verdict is recorded). Packet 227 lists this packet as a dependency; packet 226 does not hard-depend on this packet but must be sequenced after it to avoid a redundant full guest rebuild against the pre-bump toolchain.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -n 'wasmtime = \{ version = "47\.0\.3", features = \["call-hook"\] \}|wit-bindgen = "0\.60\.0"' Cargo.toml` | pin landed at root | FACT pass/fail |
| `rg -n 'wasmtime = "43\.0\.0"|wit-bindgen = "0\.57\.1"' crates modules --glob 'Cargo.toml'` | no stale pins | FACT pass (0 matches) / SNIPPETS on fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -30` | compile fallout absorbed | FACT exit 0 / SNIPPETS ≤30 on fail |
| `cargo xtask build-guests 2>&1 \| tail -20 && cargo xtask build-guests --check 2>&1 \| tail -20` | guest freshness | FACT exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -30` | lint cleanliness | FACT exit 0 |
| `cargo test -p slicer-wasm-host --test contract host_services_tdd 2>&1 \| tail -20` | host-service bindgen surface still live | FACT exit 0 |
| `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 \| tail -20` | WIT/guest drift gate green | FACT exit 0 |
| `python3 …` (AC-6/AC-9 verification) | verdict transcription | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 2 (pin bump) must land before Step 3 (sweep) and Step 4 (compile fallout), because the sweep verifies the bump and the compile gate verifies the fallout.
- The Go re-check (Step 5) must run after `cargo check --workspace --all-targets` proves the wasmtime 47 host compiles, so the slicer-only linker built in the probe matches the updated production host. The verdict transcription (Step 6) must happen in the same step as the re-check so the recorded evidence and the doc edits never diverge.
- The gate verdict in `docs/14` (Step 6) is the single cross-packet artifact; packet 227 reads it, so it must be the last step's exit condition.

## Context Discipline Notes

- `crates/slicer-wasm-host/src/host.rs` is >5,000 lines and is the single biggest bindgen/wasmtime-API surface. Do not read it in full during the fallout step; delegate a `LOCATIONS` grep for `wasmtime::component::bindgen!`, `add_to_linker`, `ResourceLimiter`, `call_hook`, `get_fuel`, `Store::new`, and `Engine` to bound the read to those ranges only.
- `crates/slicer-macros/src/lib.rs` is ~3,000 lines of generated-shape-sensitive macro code; delegate symbol lookups rather than browsing.
- The Go probe's exact commands in `docs/feasibility-probes/go-wasm.md` §8 are the authoritative reproduction recipe; never substitute an approximate `component new` invocation.
- Heavy-dispatch return limits: `cargo check`/`clippy` output must be tail-filtered (≤30 lines) in every verification command.
