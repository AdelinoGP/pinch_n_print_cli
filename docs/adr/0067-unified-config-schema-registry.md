# ADR-0067 — One config schema registry, assembled per run

Status: **Accepted.** Approved in the config-scope design interview; not yet
implemented.

> **Amendment 2026-09-11 (config-scope resolution revision session).** Three
> additions to the reconciliation rules, settled before implementation:
>
> 1. **Claim-exclusive divergence warning.** The alphabetical default
>    tie-break stands, but when modules that can never both be active (claim
>    alternatives) declare a shared key with divergent defaults or bounds —
>    measured example `detect_thin_wall`: `arachne-perimeters` `false` vs
>    `classic-perimeters` `true` — assembly emits a non-fatal load warning
>    naming both declarers and both values. The coupling stays deterministic;
>    it becomes visible rather than silent.
> 2. **`denied_scopes` unions across declarers** (see ADR-0069's amendment).
> 3. **Enum domains must agree** across declarers, else load error (same
>    class as a type conflict); UI metadata (display/group/unit/description/
>    tags) is advisory only. Every entry retains provenance — the declaring
>    host channel or module ids — so every diagnostic names contributors.
>
> The registry also gains two typed entry fields, validated at assembly:
> `selector` (the key may be read at load-time claim selection; a selector
> flag on a per-region-statable key is a load error) and `base-key` (the
> percent base, `ratio_over`: base must exist, be percent-compatible, and
> form no cycles). Finally, the registry joins **all** host declaration
> channels, not only `declare_resolved_config!`: the DSL rows,
> `FeedrateConfig::SPEED_KEYS`, `HOST_RUNTIME_KEYS`, and module manifests.
> "Host declarations" in the original text meant the macro; the registry means
> the whole universe.

Config keys were declared in two places that nothing joined: `declare_resolved_config!`
(`crates/slicer-ir/src/resolved_config.rs`) declares the host's fields, and each
module's `[config.schema.<key>]` manifest table declares its own. Ingestion could
only consult the host half — `classify_declared_key` probes `apply_cli_key` — so
every module-owned key was "undeclared" and its type was guessed from the value
text. We assemble instead **one registry per run**, joining both declaration sites
at module load, carrying each key's type, default, bounds and **scope eligibility**.
Ingestion, resolution, `ConfigView` filtering and the derived per-object admission
set all read that one registry.

## Reconciliation rules

A key may be declared by several modules, and today 51 of 162 module-declared keys
are. The registry reconciles as follows:

- **Type** must agree across all declarers. Disagreement is a load error.
- **Bounds** intersect, as before, but the intersection is **reported** rather than
  applied silently. (`layer_height` is capped at 1.0 today by one module's
  declaration, invisibly.)
- **Default** comes from the host declaration when the host declares the key;
  otherwise from the alphabetically-first declaring module id. A module default for
  a host-declared key can never apply — 48 such declarations exist — and is kept as
  documentation with a load warning rather than removed.
- **Automatic values** are expanded here, so no consuming module ever reads a
  placeholder. This is the invariant that makes a single default per key safe:
  `line_width = 0` means "derive from nozzle diameter" to some consumers and is a
  literal to others, so any single default breaks one group unless the placeholder
  never escapes resolution.

## Consequences

Module loading must precede config ingestion. This is cheaper than it sounds:
`load_modules_from_roots_with_integrated` takes no config, so only claim dedup
consulted config, and that now reads typed values after ingestion instead of raw
ones — removing the "is this key resolved yet" exception around `wall_generator`,
`spiral_vase`, `support_type` and `support_family`.

The alphabetical tie-break is arbitrary by construction. It is a deterministic
placeholder, not a claim that the alphabetically-first module owns the setting; a
key whose declarers genuinely disagree is a naming problem to fix, not a precedence
to rely on.

## Considered and rejected

- **Manifests authoritative for module keys, host for host keys, kept separate.**
  Cheaper, but makes "which declaration wins" new interface knowledge, and several
  keys are already declared in both.
- **Promote every module key into `declare_resolved_config!`.** One declaration and
  one type policy, but the host would then have to declare every community module's
  keys, which the external-module model forbids.
- **Invert: the host declares its keys in a manifest too.** Most uniform, but it
  rebuilds the one deep module here that already works well.
