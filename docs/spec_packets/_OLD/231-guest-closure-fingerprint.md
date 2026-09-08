---
status: implemented
packet: 231-guest-closure-fingerprint
task_ids:
  - TASK-342
---

# 231-guest-closure-fingerprint

## Goal

Replace the hardcoded shared-crate fingerprint set in `xtask/src/build_guests.rs` with a per-guest dependency-closure walk over manifest path deps, delete the now-unreachable `compute_shared_freshness` and `stage_wit_snapshot`, make `xtask/src/test.rs`'s pnp_cli freshness an unconditional `cargo build --bin pnp_cli`, and re-align `crates/pnp-cli-locator`'s documented `is_stale` mirror with the model that replaces it.

## Problem Statement

`cargo xtask build-guests --check` decides guest staleness from a fingerprint whose "shared" half is a hardcoded list, `let shared_crates = ["slicer-macros","slicer-sdk","slicer-ir","slicer-schema","slicer-core"]` inside `shared_input_paths` (`xtask/src/build_guests.rs`), plus a depth-2 WIT walk yielding `crates/slicer-schema/wit/root.wit` and the flat `crates/slicer-schema/wit/deps/*.wit`. Every discovered guest is charged that whole set regardless of what it actually links: 11 of the 21 discovered test guests declare **no path dependency at all** (measured 2026-08-19) and link nothing from those five crates, yet a one-byte edit anywhere in them marks every guest stale. `stage_wit_snapshot` adds a second charge on the same axis — per-stage WIT directories, or conservatively *all* of them when `GuestSpec.stage_id` is `None`.

Packets 229 and 230 move WIT staleness to artifact verification, which makes both charges wrong rather than merely coarse: the fingerprint should now cover **code inputs only**, and it should cover each guest's real code inputs rather than a union. The list is also under-inclusive in a way the union hides — `has_parent_path_dep`, the existing manifest reader, inspects only `tab["dependencies"]`, so a walk modelled on it would silently drop `crates/slicer-sdk/Cargo.toml`'s `[target.'cfg(not(target_arch = "wasm32"))'.dependencies] slicer-core` and `modules/core-modules/classic-perimeters/Cargo.toml`'s `[target.'cfg(target_arch = "wasm32")'.dependencies] wit-bindgen` (Round 5 finding R5-6).

Two dependents of the deleted model must move with it. `xtask/src/test.rs`'s `ensure_pnp_cli_fresh_with` is the sole consumer of `compute_shared_freshness` outside `build_guests.rs`; it uses it as the cutoff of a hand-rolled mtime gate that decides whether to invoke the rebuild closure at all. That gate missed per-stage WIT, host crates, optional deps and `include_str!` assets (locked decision C7), and it disappears with the model it reads. And `crates/pnp-cli-locator/src/lib.rs`'s `staleness_reason` carries a rustdoc that ADR-0054 Decision rule 5 makes normative: it is a documented mirror of `is_stale`, and it currently describes `is_stale`'s omitted third disjunct as a hash over "shared crates, the guest's own inputs, and its per-stage WIT package". Both halves of that sentence stop being true here (R5-9).

This is one coherent slice because all four edits are consequences of a single model change — what set of files a guest's fingerprint covers — and leaving any of them behind leaves a false statement or dead code in the tree.

## Architecture Constraints

- The `wasm-staleness` snippet is **deliberately omitted**. Its applies-to list is `crates/slicer-schema/wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, `modules/core-modules/*/src|Cargo.toml|wit-guest`, and `crates/slicer-wasm-host/test-guests/*/src|Cargo.toml`. This packet's entire change surface is `xtask/src/build_guests.rs`, `xtask/src/test.rs`, `crates/pnp-cli-locator/src/lib.rs` and `docs/07_implementation_status.md`. None of those feeds a guest `.wasm`: `xtask` is host-only tooling, and `crates/pnp-cli-locator` is a std-only dev-dependency that ADR-0054 Decision rule 3 forbids from ever compiling into guest WASM. No guest needs rebuilding for this packet's edits to take effect, so quoting the snippet would assert an obligation that does not exist here. (The implementer will still *run* `cargo xtask build-guests --check` as an end-to-end smoke of the new input set — see `requirements.md` §Verification Commands — but that is verification of the tool, not a rebuild obligation on the change surface.)
- The `coord-system` snippet does not apply: no geometry, no mm/unit conversion.
- Closure over-approximation is intentional and must not be "optimized": optional path deps are included (locked decision C8), and a crate reached only under a `cfg` that never matches this build is still charged. The convergence argument is that the fingerprint is a content hash rewritten after each successful build, so a spurious rebuild converges to fresh; a *missing* input never converges and is a silently-stale guest.
- No public schema/version constant is bumped here. `FINGERPRINT_VERSION` is set to `"v2"` by packet 230; this packet changes the input set behind that version, not the version itself, and must not bump it again.
- Struct-literal churn gate: `cargo xtask check-literals` watches `pub` structs with >=5 named fields defined under `crates/*/src`. `GuestSpec` is defined in `xtask/src/build_guests.rs`, so the one test-code `GuestSpec` literal (in `missing_fingerprint_metadata_is_stale`) is out of the watchlist and needs no `..` rest or waiver. Do not add one.

## Data and Contract Notes

- IR/manifest contracts: none changed. The walk *reads* Cargo manifests; it writes none, and it does not touch module manifests' `[stage]`, `[config]` or `[claims]` sections. Config-key naming is not in play.
- WIT boundary: none crossed. This packet removes `.wit` files from the fingerprint input set; it does not read, parse or compare WIT. That responsibility is entirely packets 229 and 230.
- Determinism/scheduler constraints: `guest_closure_input_paths` must be order-deterministic (sort then dedupe) because its output feeds `fingerprint_entries`, which sorts by `(path, bytes)` but whose input path strings are recorded relative to `ws_root`. Canonicalization must not leak absolute machine paths into the hash: keep `snapshot_from_paths`' existing `ws_root`-relative rendering, and canonicalize only for cache keying and cycle detection.

## Locked Assumptions and Invariants

- **ADR-0054 is conformed to, not amended, and packet 231 owns that decision.** Rule 5 requires `staleness_reason`'s rustdoc to pin `is_stale` by crate-qualified path and symbol name "so the two stay legible as siblings when either changes". `is_stale` changes here, so the rustdoc must be updated — that update *is* the conformance. None of the five Decision rules is contradicted: the crate stays std-only (no dependency added), dev-dependency only, host-side only, and owns exactly the same four functions with unchanged signatures. Therefore no `D-231-ADR-0054-AMENDED` deviation is filed and no superseding ADR is authored. Packet 232 must not amend ADR-0054 either; it owns ADR-0014 and ADR-0045 only. Exactly one packet touches ADR-0054's subject matter, and it is this one.
- **ADR-0014 is amended, but by packet 232, not silently.** Its `## Amendments` section records packet 185's rule that `slicer-core` is tracked "in `xtask/src/build_guests.rs::shared_crates`", and its Consequences claim "Touching `slicer-core` does not trigger a guest rebuild storm". This packet deletes `shared_crates` and changes when that claim is true. ADR-0014's *normative decision* — guest discovery by validated filesystem walk rather than `cargo_metadata`, and no heavy `xtask` dependency — is conformed to exactly: `discover_guests` and `has_parent_path_dep` are untouched and no dependency is added. The stale amendment and consequence text is repaired by packet 232's AC-12, which names this packet. That cross-reference is what makes the change an explicit amendment rather than a silent ADR rewrite; do not also edit ADR-0014 here.
- Dev-dependencies are permanently excluded from the closure. This is a correctness rule, not a performance choice: a dev-dep does not compile into the guest artifact, and including one makes unrelated test-tree edits mark guests stale — the behaviour this packet exists to end.
- Optional path deps are permanently included (C8).
- The fingerprint covers code inputs only. Re-adding any `.wit` path to the fingerprint would restore double-counting on top of packet 230's artifact verification.
- `GuestSpec.stage_id` and `parse_stage_id_from_module_manifest` survive (R5-4). They are packet 230's independent stage expectation; deleting them re-opens the self-referential-check regression that `module_stage_wit_dir`'s own doc comment records from packet 164.

## Risks and Tradeoffs

- **Closure under-approximation is silent.** If the walk misses a table form, the affected guest reports fresh while running old code — the same failure class as the 2026-07-25 missing-`slicer-core` incident, which `--check` reported clean. Mitigated by AC-2 (three table forms), AC-3 (a real chain), AC-N4 (errors instead of truncation), and by the `cargo xtask build-guests --check` exit-code smoke.
- **Over-approximation costs rebuilds.** A guest whose closure includes an optional dep it never compiles rebuilds when that dep changes. Accepted per C8; it converges.
- **Unconditional `cargo build --bin pnp_cli` adds a fixed cost to every `cargo xtask test`.** The cost is a no-op Cargo fingerprint check when nothing changed. **Unmeasured on this machine at authoring time**; the implementer should time `cargo xtask test -- --help` before and after if a figure is wanted, and must not quote one otherwise.
- **`ClosureCache` threading touches packet 230's freshly-landed `CheckContext`.** Sequencing risk, not correctness risk: if 230 has not landed, Step 2 cannot compile. Guarded by the activation blocker in `packet.spec.md`.
- **The rustdoc can drift again.** ADR-0054 rule 5 is a documentation obligation with no compiler enforcement. AC-12's grep is the only automated guard, and it is a text grep; it will not catch a rustdoc that is merely stale in some other clause.
