# ADR-0045: Per-stage versioned interfaces over monolithic tier worlds

Status: proposed

ADR-0044 established that the world version enforces nothing and removed it from
module identity. This ADR records the deeper finding underneath it, and the fix.

## The finding

**A world is an all-or-nothing contract, and nobody ever decided that.**

`git log --all --grep` for `split.*world|per-stage world|monolithic` returns
nothing. No ADR covers world granularity. ADR-0003 comes closest and *assumes*
world = tier while deciding something else entirely ("a given guest only links the
world it implements"). It is an unexamined default.

The chain:

1. `slicer-sdk/src/traits.rs` gives `LayerModule` default no-op bodies for all 9
   stage methods, so a module overrides only its own stage.
2. `#[slicer_module]` nonetheless emits WIT glue for **all 10** exports; the 9 a
   module doesn't implement collapse to `quote! { Ok(()) }`.
3. wasmtime's generated `Indices::new` resolves and typechecks **every** export in
   the world at `instantiate` time — eagerly, before any call:
   `get_export(None, "run-X").ok_or_else(|| format_err!("no export found"))?`.
4. Therefore the stubs are load-bearing padding, and **any** change to **any**
   export invalidates **every** guest bound to the world.

`arachne-perimeters` — a perimeters module — ships `run-infill-postprocess` with
the `prior-infill` parameter it will never read, and its `.wasm` was invalidated by
packet 130's infill change. Confirmed by decoding the binary.

Two consequences worth stating plainly:

- **`docs/05`'s promise was structurally unachievable**, not merely unimplemented.
  "Modules built against an older SDK minor version always load on a newer host
  (additive compatibility)" cannot hold while a guest must satisfy the world's
  entire export surface. There is no notion of an optional export: `WIT.md` states
  `@since`/`@unstable` gates "are not represented in the component binary… not
  part of the runtime semantics of components".
- **The 9 stubs silently return success.** They violate ADR-0015's "do not catch
  module fatals in the macro or host glue… do not swallow" by construction.

Blast radius scales with export count, which is why the pain landed on
`world-layer` and the other three tiers have been quiet:

| World | Exports |
|---|---|
| world-layer | 10 |
| world-prepass | 4 |
| world-postpass | 2 |
| world-finalization | 1 |

## Decision (proposed)

Restructure each stage into its own **versioned interface**, e.g.
`slicer:world-layer/infill-postprocess@2.0.0`. A module exports only the interface
it implements; the host probes each and tolerates the miss.

This is the Bytecode Alliance's sanctioned pattern for prebuilt plugin
ecosystems, not an invention: since there are no optional exports, **granularity is
the only route to optionality**.

Scope `world-layer` first; prepass/postpass/finalization may not be worth it.

## Why this works

Versioning an *interface* puts the version in the component's export names, where
the engine can act on it. Wasmtime has semver-matched exports since PR #8830
(2024-06-18); we are on 43.0.1. `wasmtime_environ::component::names::alternate_lookup_key()`
registers `1.1.2` under an alternate key of `1` (major nonzero → truncate to
major), so a guest exporting `@1.0.0` and a host wanting `@1.1.0` resolve via the
shared `@1` key, in both directions. Crossing `1.x → 2.0` breaks cleanly.

**We already have that engine; the bare-func world structure routes around it.**

The refactor follows a seam that already exists: `dispatch.rs` already does
`match stage_id.as_str()` *after* instantiating the monolithic world.

|  | today | per-stage versioned interfaces |
|---|---|---|
| `docs/05`'s additive-compat promise | structurally impossible | true, via wasmtime's `@1` alternate key |
| infill change breaks perimeters modules | yes | no — untouched, doesn't even rebuild |
| version enforced | not at all (erased) | by wasmtime, free, at instantiate |
| the 9 lying `Ok(())` stubs | required as padding | gone |
| manifest `wit-world` + allowlist | unfalsifiable ceremony | deletable — binary carries the truth |

## Consequences

- A stage's contract change stops invalidating unrelated modules.
- The version becomes real without any bespoke checking code.
- `wit-world`, `SUPPORTED_WIT_WORLDS`, and `validate_wit_world` can retire.
- Significant refactor of macro glue, host bindgen, and dispatch.
- Timing: do it while the third-party ecosystem is still nascent. The out-of-tree
  search path (`module_search_path.rs`) and the `docs/00` promise ("Community
  modules ship as `.wasm` + `.toml`") are real, but no registry, no install path,
  and no out-of-tree module exist. Breaking changes are free now and will not be
  later.
