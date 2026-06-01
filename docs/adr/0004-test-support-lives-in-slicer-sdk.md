# ADR-0004: Test support lives in slicer-sdk behind a `test` feature

## Status

Accepted (packet 77, 2026-05-31)

## Context

Before packet 77 the project had two unconnected test-support surfaces. The
`slicer-test` crate exposed a `MockHost` with its own state, while
`slicer_sdk::host::test_support` carried thread-locals that the SDK's host
service wrappers read during `cfg(test)` builds. Neither knew about the other:
clearing one did nothing to the other, and module authors could not reach the
SDK thread-locals from outside the `slicer-sdk` crate at all (no public re-export,
no feature gate).

On top of that, the `#[slicer_module]` macro's expansion referenced helper
identifiers (e.g. `test_support::reset_global_state`) that no crate actually
defined at the path the expansion expected, and `docs/05_module_sdk.md` documented
a `slicer_sdk::test_prelude` that did not exist. Module-author unit tests against
the published surface either could not compile or silently drifted from the host's
real behaviour. Packet 77 exists to close that seam.

## Decision

Test-support APIs are owned by `slicer-sdk` and exposed as
`slicer_sdk::test_support` (this packet, 77) and — in packet 78 — re-exported
through a curated `slicer_sdk::test_prelude`. The module is gated behind a Cargo
feature named `test` (also auto-enabled under `cfg(test)`), so production guest
WASM builds pay no cost.

The fold direction is deliberate: **test support lives inside slicer-sdk** so
that module authors get test-support APIs from the same crate they use to author
modules, the `#[slicer_module]` macro can emit a single fully-qualified path
(`::slicer_sdk::test_support::…`) that always resolves, and the documented public
surface becomes honest.

## Consequences

Positive:

- One source of truth for test seams (thread-locals, fixture builders, mock host
  state) — no more "which crate's MockHost did I reset?".
- The macro's emitted code can name `::slicer_sdk::test_support::*` directly,
  removing the dangling-identifier class of bug.
- `docs/05_module_sdk.md`'s `test_prelude` description becomes implementable
  (packet 78) against a real module path.

Negative:

- `slicer-sdk` now carries a Cargo feature (`test`), and guest WASM builds
  **must not** enable it — `cargo xtask build-guests` and every
  `modules/core-modules/*/Cargo.toml` must continue to depend on `slicer-sdk`
  with `default-features = false` and without `features = ["test"]`. A future
  change that flips this default would bloat every guest `.wasm`.
- `slicer-test` is not deleted in this packet; the redundancy is closed
  incrementally (see Alternatives).

## Alternatives Considered

1. **rename slicer-test to slicer-sdk-test** — rejected because it preserves the
   two-crate split and the duplicated thread-local/MockHost state; the rename is
   cosmetic and does not let the macro emit a single path.
2. **delete slicer-test outright** — rejected for this packet because it would
   orphan in-flight test-fixture helpers and unrelated downstream callers; the
   deletion is deferred to packet 78 once `test_prelude` re-exports cover the
   surface `slicer-test` provides today.
3. **keep slicer-test as a separate crate but wire the macro to it** — rejected
   because every crate that uses `#[slicer_module]` already depends on
   `slicer-sdk`; routing the macro through `slicer-test` would force a
   `slicer-sdk → slicer-test → slicer-sdk` dep chain (or make `slicer-test` a
   mandatory dep of every module crate), neither of which is acceptable.
