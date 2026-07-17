# ADR-0045: Per-stage versioned packages over monolithic tier worlds

Status: accepted

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

## Decision

Restructure each stage into its own **versioned WIT package**, e.g.
`slicer:layer-infill-postprocess@1.0.0`, holding one interface. A module exports
only the package for the stage it implements.

```wit
package slicer:layer-perimeters@1.0.0;
interface perimeters { run: func(...) -> result<_, module-error>; }
world perimeters-module { import ...; export perimeters; }
```

17 packages: 10 layer + 4 prepass + 2 postpass + 1 finalization. All four tiers,
so exactly one mechanism exists. The three small tiers are quiet today for the same
accidental reason `world-layer` was quiet for 150 packets — nobody has changed them
yet.

`[stage] id` is **singular in all 20 manifests**, so the host always knows which
stage to instantiate. There is no probing: dispatch resolves `stage_id` → package →
typed instantiate. A module that declares a stage whose interface it does not export
**fails at load**, with a diagnostic naming the expected `package/iface@version`.
That is what retires the lying `Ok(())` stubs and satisfies ADR-0015.

The manifest keeps `[stage] id` — the DAG validator and `dag_cli` plan without
instantiating any WASM (see ADR-0006's rejected alternative), so the stage must be
declarable ahead of load. `wit-world`, `SUPPORTED_WIT_WORLDS` and
`validate_wit_world` retire.

Granularity is the only route to optionality, since the component model has no
optional exports. This is the Bytecode Alliance's sanctioned pattern for prebuilt
plugin ecosystems, not an invention.

### The unit is the package, not the interface

An earlier draft of this ADR proposed `slicer:world-layer/infill-postprocess@2.0.0`
— stages as interfaces *inside* a tier package. **That does not work, and it fails
on this ADR's own motivating example.**

In WIT, `@version` is a property of the **package**; an interface cannot carry one.
The grammar attaches `<semversuffix>` to `package-name`, and there is no
interface-version production. Our own tree demonstrates it — `wit/deps/common.wit`:

```wit
package slicer:common;        // no version
interface module-errors { }   // cannot carry one
interface host-services { }   // cannot carry one
```

So every interface in `slicer:world-layer@2.0.0` shares that one version. Replay
packet 130 under that shape: adding the required `prior-infill` param is breaking →
the package majors `1.x → 2.0.0` → all ten interfaces move from wasmtime alt-key
`@1` to `@2` → `arachne-perimeters`, exporting `perimeters@1.1.0`, misses again.
The promised outcome ("untouched, doesn't even rebuild") is only reachable when
`slicer:layer-infill-postprocess` can bump while `slicer:layer-perimeters` sits
still. Hence: one package per stage.

### Versions reset to 1.0.0

Every stage package starts at `@1.0.0`, discarding `world-layer`'s current
`@2.0.0`. This is mechanical, not cosmetic — see the alt-key table below. Major
must be nonzero for major-track compatibility; at `0.x` every minor bump breaks,
and at `0.0.x` there is no compatibility track at all.

Names are tier-prefixed (`slicer:layer-perimeters`, `slicer:prepass-seam-planning`).
The tier survives as **vocabulary**; it dies as a **contract**.

## Why this works

Versioning a *package* puts the version in the component's export names, where the
engine can act on it. Wasmtime has semver-matched exports since PR #8830
(2024-06-18); we are on 43.0.1. `wasmtime_environ::component::names`'
`alternate_lookup_key` registers a nonzero major under a truncated key, so a guest
exporting `@1.0.0` and a host wanting `@1.1.0` resolve via the shared `@1` key, in
both directions. Crossing `1.x → 2.0` breaks cleanly. Verified against the pinned
source:

| name | alternate key |
|---|---|
| `x:y/z` | `None` — unversioned, exact match only |
| `x:y/z@1.1.2` | `x:y/z@1` — major track |
| `x:y/z@0.1.0` | `x:y/z@0.1` — minor track |
| `x:y/z@0.0.1` | `None` — no compatible track |

A bare func name (`run-perimeters`) yields `None`: no `@`, no semver, no matching.
**We already have that engine; the bare-func world structure routes around it.**

### Verified empirically, not just read

The above is source-reading. Before any packet was built on it, a throwaway spike
executed the mechanism against the pinned wasmtime 43.0.1 / wit-bindgen 0.57.1. All
of it holds:

```
[guest @1.0.0 / host wants @1.0.0 (exact)]        OK
[guest @1.0.0 / host wants @1.5.0 (the claim)]    OK   <- alt-key @1 resolves
[guest @1.5.0 / host wants @1.0.0 (reverse)]      OK   <- both directions, as claimed
[guest @1.0.0 / host wants @2.0.0 (major break)]  FAIL <- clean, at instantiate
[guest @0.1.0 / host wants @0.2.0 (major == 0)]   FAIL <- minor-track
```

`alternate_lookup_key` is genuinely wired into component export resolution on the
typed `bindgen!` path; the version survives into the export name
(`world root { export spike:alpha/foo@1.0.0; }`), which is what makes any of this
possible.

**Stage isolation, the headline benefit, reproduces exactly — at the engine.** A guest
exporting only `spike:alpha` against a host binding both alpha and beta, after beta
took a breaking change and a major bump: `sha256` byte-identical before and after,
never rebuilt, instantiates fine.

**But "doesn't even rebuild" is false in-tree, and not because of packaging.**
`xtask/src/build_guests.rs::compute_shared_mtime` walks *all* of
`crates/slicer-schema/wit`, takes the `.max()`, and applies that one mtime to **every**
guest. So a one-stage `.wit` bump marks all 32 guests STALE however the packages are
cut. Splitting the packages does not fix that; charging WIT mtime per stage does, and
the pilot packet scopes it — otherwise the pilot cannot demonstrate the thing it
pilots. Separate the two claims:

| claim | in-tree | prebuilt / out-of-tree |
|---|---|---|
| an unrelated stage's change **breaks** the module | no (proven) | no (proven) |
| it **rebuilds** the module | yes, until `compute_shared_mtime` is per-stage | no — nobody rebuilds it |

The ecosystem case — the one that justifies this ADR — is the second column, where the
benefit is complete and needs no xtask change: a third-party `.wasm` simply keeps
loading. In-tree the rebuild is incidental and cheap. **Not breaking is the benefit;
not rebuilding is a build-hygiene nicety.** The original table row conflated them.

**And the rejected alternative fails, on a control.** The same two interfaces placed in
one package `spike:mono@2.0.0` — packet 130 replayed under "stages as interfaces in a
tier package" — rejected the partial guest with
`no exported instance named `spike:mono/foo@2.0.0``. `foo` had not changed; its
sibling's bump moved its alt-key from `@1` to `@2`. §"The unit is the package" was
argued rather than measured when written; it is now measured.

Two things the spike surfaced that this ADR overstated or omitted:

- **The diagnostic is thinner than "naming the expected `package/iface@version`"
  implies.** wasmtime says `no exported instance named `slicer:x/y@2.0.0`` — it names
  what the *host wanted* and never what the *guest shipped*. "Expected @2.0.0, found
  @1.0.0" is host-side work decoding the component's exports; budget for it rather
  than assuming the engine supplies it.
- **"Versions reset to 1.0.0" is load-bearing, not housekeeping.** The `@0.1.0` vs
  `@0.2.0` row simply misses. A stage package shipped at `0.x` gets no compatibility
  at all, so `major >= 1` deserves a mechanical assertion.

**Still unproven:** the spike used no host imports. The real stage worlds import
`slicer:common/host-services`, `slicer:config/config-types` and
`slicer:ir-handles/ir-handles`, and depend on `with:`-mapped resource *identity*
holding across many separate `bindgen!` calls (ADR-0002). Nothing in the mechanism
above depends on that, but nothing above tests it either — which is precisely why the
first packet pilots one real stage with its actual imports before the rest follow.

### The naive shape inverts resource ownership

That gap bit immediately, and it changes the WIT this ADR prescribes. A `resource`
declared in an **exported** interface is **guest**-owned; our stages take
**host**-owned resources (`host.rs` really does
`impl HostGcodeOutputBuilder for HostExecutionContext`). So folding a world body
straight into the exported interface — the obvious reading of the spike's skeleton,
whose `foo` takes no parameters — inverts ownership and breaks every host builder
impl.

Each resource-bearing stage package therefore pairs an **imported** `<iface>-types`
interface with an exported, `run`-only interface. Both live in the same package, so
one stage still means one version. A stage with no resources (e.g.
`postpass-text-postprocess`) needs no `-types` half. **An exported interface must
declare no resources**, and that is worth asserting mechanically.

This is why a spike is not a design. It proved the mechanism and hid a requirement,
because the thing it omitted for simplicity — imports and resources — is where the
real contract lives.

The refactor follows a seam that already exists: `dispatch.rs` already does
`match stage_id.as_str()` *after* instantiating the monolithic world.

|  | today | per-stage versioned packages |
|---|---|---|
| `docs/05`'s additive-compat promise | structurally impossible | true, via wasmtime's `@1` alternate key |
| infill change **breaks** perimeters modules | yes | no — proven at the engine, both in-tree and prebuilt |
| infill change **rebuilds** perimeters modules | yes | in-tree: yes, until `compute_shared_mtime` charges WIT mtime per stage (see §"Verified empirically"). Prebuilt/out-of-tree: no — nobody rebuilds it. Not breaking is the benefit; not rebuilding is hygiene |
| version enforced | not at all (erased) | by wasmtime, free, at instantiate |
| the 9 lying `Ok(())` stubs | required as padding | gone |
| manifest `wit-world` + allowlist | unfalsifiable ceremony | deletable — binary carries the truth |

## The lifecycle exports go with them

`on-print-start` / `on-print-end` are deleted from WIT rather than carried into the
new packages. They are the purest padding in the tree:

- `call_on_print_start` / `call_on_print_end` have **zero callers in the host**.
  `docs/04`'s "call on-print-start on all modules" describes a call that was never
  written.
- The macro's `on_print_end` glue is hardcoded `Ok(())` and **never dispatches** to
  the trait. Every module's `on_print_end` body is unreachable.
- The macro's `on_print_start` glue does `Ok(_m) => Ok(())` — constructs the module
  and discards it — while all 15 `run_*` arms construct it again per call. No
  `OnceCell` or `static` retains anything, so `docs/05`'s "initialize expensive
  resources once per print" is exactly inverted: it runs once per *layer*, per
  *stage*.
- `WORLD_LIFECYCLE_EXPORTS` claims all four worlds ship them; only `world-layer.wit`
  declares them. Its guard test `every_world_has_lifecycle_exports` reads that table
  and asserts against the same table — vacuous, the identical pathology ADR-0044
  found in `wit_world_major_version_mismatch_rejects_future_major`.

The SDK trait method survives under an honest name: `on_print_start(config) ->
Result<Self>` is a constructor, so it becomes `from_config`. `on_print_end` is
deleted from all four traits.

Nothing is lost, because the concepts already have homes. **OrcaSlicer has no such
hook** — it expresses lifetime by where the object lives: `SeamPlacer::init` runs
once per print on a `GCode` member, while `Fill` (`Layer::make_fills`) and
`PerimeterGenerator` (`LayerRegion::make_perimeters`) are rebuilt per layer. Our
tier system already encodes both — per-print is the prepass tier plus the
Blackboard (ADR-0029); per-layer is the layer tier. And the *real* print start/end
is a user-editable G-code template (`machine_start_gcode` / `machine_end_gcode`),
read at `run_gcode_postprocess` by `machine-gcode-emit`: a different tier, a
different lifetime, a different owner. Two things were named "print start"; only
one was real.

## Consequences

- A stage's contract change stops invalidating unrelated modules.
- The version becomes real without any bespoke checking code.
- `wit-world`, `SUPPORTED_WIT_WORLDS`, `validate_wit_world`, and
  `WORLD_LIFECYCLE_EXPORTS` retire.
- Significant refactor of macro glue, host bindgen (4 `bindgen!` → 17), and dispatch.
- **Forecloses**, honestly: a layer module holding cheap *private* state across
  layers (a scratch buffer, a warn-once flag). It cannot do so today — the module is
  rebuilt per call — so nothing that currently works is lost. But re-adding it later
  requires a new contract, not this one. `on_print_start(config)` could never have
  served it: it sees only config, so anything it cached would be stale the moment a
  module declares a non-empty `[config.overridable-per-layer]`. Packet 102 already
  ruled that caching "forbidden because it defeats the layer-override mechanism".
- Timing: do it while the third-party ecosystem is still nascent. The out-of-tree
  search path (`module_search_path.rs`) and the `docs/00` promise ("Community
  modules ship as `.wasm` + `.toml`") are real, but no registry, no install path,
  and no out-of-tree module exist. Breaking changes are free now and will not be
  later.

## Alternatives rejected

- **Stages as interfaces inside one tier package per tier.** The original draft of
  this ADR. Rejected: WIT versions packages, not interfaces, so a breaking change to
  any stage majors the package and moves every sibling interface's alt-key. It fails
  the packet-130 case this ADR exists to fix. See "The unit is the package".
- **One tier package, never major-bumped (stay `@1.x` forever).** Mostly works —
  siblings keep resolving via the `@1` alt-key, and a genuinely changed signature
  fails its own typecheck. Rejected because the version would once again claim
  compatibility it does not have, and breaking changes would surface as structural
  type errors rather than clean version misses. That is the precise sin ADR-0044
  spent its length killing; reintroducing it one ADR later is not defensible.
- **Scope `world-layer` only, leave the other three tiers.** Rejected: it leaves two
  contract mechanisms live permanently, and the macro, host bindgen and dispatch
  would each carry both paths. The other three tiers are only 7 exports in total —
  cheap once the machinery exists. "Temporary" branches have a track record here
  (`run.rs`'s advisory-mode "pragmatic fix" is 14 months old and still load-bearing).
- **Probe each stage and tolerate the miss.** The original draft's dispatch rule.
  Rejected: it assumed modules implement several stages, and `[stage] id` is
  singular in all 20 manifests. Tolerating a miss would recreate the silent-success
  failure mode this ADR exists to delete.
