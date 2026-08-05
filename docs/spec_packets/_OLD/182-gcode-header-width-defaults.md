---
status: implemented
packet: 182-gcode-header-width-defaults
task_ids:
  - TASK-295
---

# 182-gcode-header-width-defaults

## Goal

Correct the `DefaultGCodeSerializer` header-comment wall-width defaults from the non-governing `0.42`/`0.45` to the governing `0.4`/`0.4` fallback, delete the dangling `config_schema.rs` citation, and re-bless the one golden fixture that captured the old values — so the emitted `; outer_wall_line_width` / `; inner_wall_line_width` header lines report the value the pipeline actually falls back to (`resolve_line_width_mm` → `0.4`). Closes deviation G-code header width-default gap.

## Problem Statement

Deviation **G-code header width-default gap** (Open, filed 2026-07-16 during the D-160 session). `DefaultGCodeSerializer` (`crates/slicer-gcode/src/serialize.rs`) hard-codes `outer_wall_line_width: 0.42` and `inner_wall_line_width: 0.45` in its `with_extrusion_mode` constructor, and `serialize_width_comments` prints those values verbatim into every G-code header as `; outer_wall_line_width = 0.42` / `; inner_wall_line_width = 0.45`.

Two things make this a lie rather than a stale constant:

1. **The cited authority does not exist.** The constructor comment reads "matches `config_schema.rs` registration"; `config_schema.rs` was deleted from the tree and registers nothing. The per-field doc comments repeat the same wrong values.
2. **The governing fallback is `0.4`, not `0.42`/`0.45`.** Two independent code paths establish it: `resolve_line_width_mm` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) falls back to `0.4`, and the `classic-perimeters` guest resolves `legacy_line_width` to `0.4` when config omits the keys, yielding outer = inner = `0.4`.

There is **no config-driven setter** for these two serializer fields — the only constructors (`new`, `with_extrusion_mode`, `with_flavor`, `with_filament_config`) never touch line widths — so the header emits the hard-coded values regardless of the slice's actual configuration. The header therefore misreports the width for every print. This is one coherent slice: a wrong constant plus its dangling citation, in one file.

## Architecture Constraints

- The governing authority for `0.4` is `resolve_line_width_mm` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`), whose doc comment states the fallback "matches the guest-side default used by `classic-perimeters`/`arachne-perimeters`". The replacement comment must cite that symbol — not a file that does not exist.
- **Do not preserve the "OrcaSlicer parity" attribution when swapping the number.** The two field doc comments currently read "OrcaSlicer 0.4 mm nozzle parity default: 0.42". Merely changing the digits keeps a false citation: open deviation `wall-width config-type gap` records that upstream registers both keys as `coFloatOrPercent` default `0` (auto-derive from nozzle), so "OrcaSlicer parity default: 0.4" would be a fresh D-165-class lie in the very lines being repaired. Attribute `0.4` to `resolve_line_width_mm`. **The three sibling field doc comments (`sparse_infill_line_width`, `top_surface_line_width`, `support_line_width`) keep the same attribution phrasing, knowingly** — `D-164` disproves it for the wall keys only, and this packet has not established those three fields' governing fallbacks, so rewriting their attribution would repeat the D-165 mistake. See `requirements.md` §Out of Scope.
- **Blast radius includes recorded outputs, not just struct literals.** `DefaultGCodeSerializer` is constructed only via its four constructors, so there are no struct-literal sites to update — but the emitted header is captured byte-for-byte in a golden fixture. A change with zero struct-literal fallout can still have test fallout through recorded output; the golden must be re-blessed in the same packet.
- `DefaultGCodeSerializer` has no config-driven setter for these fields (`new`, `with_extrusion_mode`, `with_flavor`, `with_filament_config` are the complete set on this impl — the second `new`/`with_flavor` pair in the file belongs to `ThumbnailAwareSerializer`; `impl Default` delegates to `new()`). This packet must not add one — making the header config-aware is out of scope.
- The guest-WASM staleness snippet does not apply to this packet's **edit** surface: `crates/slicer-gcode` is a host crate and is not on the guest-WASM input list in `CLAUDE.md` §"Guest WASM Staleness". It does, however, apply to the **test** surface: AC-3's `legacy_zero_matches_golden` shells out to `pnp_cli --module-dir` and executes the real core-module guests. If that test fails, run `cargo xtask build-guests --check` and rebuild if `STALE:` before attributing the failure to this change.
- The same test also requires a freshly built `pnp_cli`. It locates the executable on disk via `pnp_cli_bin()`, and `crates/slicer-runtime` does not depend on the CLI crate, so `cargo test` alone will not pick up a `serialize.rs` edit. `cargo build --workspace` is a precondition of the re-bless step; `cargo check` is insufficient because it produces no executable.
- No coordinate-system bullet applies: the change prints an existing mm-domain `f32` into a comment string; there is no mm↔internal-unit conversion and no geometry.

## Data and Contract Notes

- IR/manifest contracts: none touched. The two fields are private serializer state.
- WIT boundary: none. `crates/slicer-gcode` is host-side and exports no WIT.
- Determinism/scheduler constraints: none. The emitted header is already deterministic; only the printed constant changes — which is exactly why a byte-identity golden notices.

## Locked Assumptions and Invariants

- Locks the emitted default header values to `0.4`/`0.4` for as long as the serializer remains config-blind. Reversible by a one-line edit, and superseded the moment the header is wired to resolved config (out of scope). No behavior lock beyond the printed constant.

## Risks and Tradeoffs

- Any external consumer parsing `; outer_wall_line_width = 0.42` as a stable sentinel value would see `0.4`. Low risk: the value was never config-accurate, and the in-tree sentinel test asserts only the key's presence.
- The header remains a *default*, not the slice's real resolved width. This packet makes it honest, not correct-per-slice; the residual gap is recorded in Out of Scope so it is not mistaken for closure of the broader question.
- Re-blessing a byte-identity golden is a blunt instrument: the diff must be inspected to confirm **only** the two wall-width lines changed. A blind re-bless would silently absorb any unrelated drift.
