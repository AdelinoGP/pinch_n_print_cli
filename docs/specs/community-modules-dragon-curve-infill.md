# Dragon Curve Infill — first community module (design spec)

Status: grilling complete; ready to be broken into spec packets by a downstream session; Packet queue plan: `docs/specs/community-modules-dragon-curve-plan.md`.

## Purpose

Land the **Dragon Curve Infill Module** as the first `modules/community-modules` entry
in Pinch 'n Print. It is the proof-of-concept that:

1. Go can author a wasm32 module satisfying a PnP **stage contract** (temporary
   workflow while the SDK is not yet authored as a dependency).
2. A community module can control its **own coloring** through the new
   `fill_authored_coloring` setting, granted against the claim mechanism.

This document is the agreed design after grilling. It does **not** contain code.

## Status of open questions (resolved)

Every open question raised at the start of the grill is answered below. The
single item that remains a *spike*, not a decision, is **Go → wasm-component
feasibility** (see [Go feasibility spike](#go-feasibility-spike)); the module's
design assumes the spike succeeds and the spec defines the fallback.

---

## 1. What "community module" means here

**Definition (new CONTEXT.md term):** a **community module** is an *external
module* authored by a non-core party and distributed as a git **submodule** into
a **fork** of Pinch 'n Print. It is the explicit, temporary pre-SDK workflow:
while the SDK is not yet consumable as a dependency, a community module ships in
its own repo (the submodule source), is pinned into a fork, and re-declares the
WIT types it needs instead of depending on the Rust `slicer-sdk` crate.

It is a provenance + distribution term, not a capability term. Capability-wise a
community module is exactly an external module; the distinction is who wrote it
and how it gets to the host.

**The Dragon Curve in THIS repo is a labeled example only.** The example module
is committed here so the pattern is visible, but real community modules must NOT
be added to this repository. The rule is enforced socially (banner README +
docs note + `CLAUDE.md` instruction), not mechanically.

### Packaging

- Lives at `modules/community-modules/dragon-curve/`.
- Contains: Go source, a manifest (`*.toml`), a committed `.wasm` component, a
  build script (`Makefile`/`justfile`), and a banner `README.md`.
- Module id: `com.example.dragon-curve` (the `com.core.` prefix is reserved;
  community ids use `com.<owner>.<name>` and are referenced by full id in config).
- Discovered by the existing search-path machinery: `--module-dir` or
  `SLICER_MODULE_PATH` pointed at the module's parent. Nothing auto-scans the
  workspace `modules/` tree (unchanged).

### Build & artifact

- The committed `.wasm` is produced by the module's build script.
- **Excluded from CI.** The Go module sits outside the workspace Cargo graph, so
  workspace `fmt` / `clippy` / `test` and `cargo xtask build-guests` never touch
  it — that is the natural exclusion, not a special case.
- A manual slice-test path is documented: run `pnp_cli slice --module-dir
  modules/community-modules/dragon-curve …` to verify the module runs.

### Versioning

Follows the existing manifest `[compatibility]` convention exactly (as the
core-modules do):

- `[module] version` — the module's own version.
- `[compatibility] min-host-version` / `min-ir-schema` / `max-ir-schema` — the
  host contract floor/ceiling the module targets.
- Submodule pinned to a tag/commit (not a moving branch) in the consuming fork.

---

## 2. Claim mechanism and `fill_authored_coloring`

### Current claim mechanics (established facts, unchanged)

- Claims are `claim:*` strings. The host grants a fill-role claim via
  `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`; `resolve_held_claims`
  filters which module *holds* each role per region; `should_emit(role)` gates
  on the held set. A module only emits a role it actually holds.
- Any `claim:*` id is accepted at load (no allowlist), but that alone grants
  nothing — the module must be the resolved holder for the region's role.
- Coloring today is **host-resolved per region** (`resolve_region_tool_index`):
  material paint in the variant chain, else a modifier `extruder` delta, else
  `0`. No module controls its own coloring.

### New: `claim:authored-coloring` (capability disclosure)

A module that can control its own coloring **discloses** it in the manifest:

```toml
[claims]
holds = ["claim:sparse-fill", "claim:authored-coloring"]
```

`claim:authored-coloring` is a *capability claim* (what the module can do), not a
contested role claim. It participates in the declared-vs-held vocabulary: it is
held when granted, gated by the setting.

### New: `fill_authored_coloring` setting (config-author authorization)

`fill_authored_coloring` is a config list of **fill-role claim strings**. Listing
a role means "the module that holds this role may self-color its output":

```
fill_authored_coloring = ["claim:sparse-fill"]
```

It is **overridable per region** (like `infill_density`).

### Grant — the intersection

The host grants authored-coloring to a module **on a region** only when **both**
hold:

1. the module is the active holder of a fill-role claim **listed in**
   `fill_authored_coloring` (config author authorizes the role), **and**
2. the module **discloses** `claim:authored-coloring` (module author discloses
   the capability).

Config author and module author must both consent. No `fill_authored_coloring`
entry, or no disclosed claim, ⇒ the module is not granted; the host colors
per-region as today.

### Enforcement

- **Host strips ungranted `Some(tool)`.** At the marshal boundary the host knows
  each region's grant. If a module emits a per-path tool but is not granted for
  that region, the host discards the override and colors per-region. Modules
  never have to guard; ungranted overrides are silently ignored (not a hard
  error).
- When not granted, behavior is a pure function of (module, setting, region) and
  matches the status quo.

---

## 3. The per-path tool carrier (WIT change)

For a module to control coloring it must emit a per-path tool. This requires a
**versioned WIT change** to the shared geometry type.

- Add `tool-index: option<u32>` to the `extrusion-path3d` record
  (`crates/slicer-schema/wit/deps/types.wit`), with **`None` = host decides**.
- Bump the `slicer:types/geometry` package version; regenerate bindings for
  `slicer-sdk`, `slicer-schema`, `slicer-wasm-host`, and every guest's
  `wit-guest`.
- Support and finalization stages also consume `extrusion-path3d`; they set
  `None` and are behaviorally unaffected.

**Why the field, not a builder side-list:** the infill linker clones and re-emits
`ExtrusionPath3D` and produces *new* clipped/re-linked paths inside
`chain_or_connect_infill`; a parallel per-path tool list on the builder would
not survive that and would break. A field on the path survives cloning.

### Downstream consequences (all verified feasible)

- **Marshal boundary:** infill output → entity construction stamps
  `entity.tool_index`; with the carrier it must honor `Some(tool)` as an override
  of `resolve_region_tool_index`, subject to the grant, else strip to the region
  tool.
- **Infill linker:** `paths_compatible` (orchestrate.rs) must add tool equality
  to its predicate, and the linker must **split / refuse to chain** a polyline
  across differing per-path tools. This is the same guard it already applies at
  region level (`compatible_regions` / `majority_owner`).
- **Path optimizer:** clusters by `entity.tool_index` — already per-entity, no
  change.
- **Emitter:** keys T-changes off `entity.tool_index` — already per-entity, no
  change.
- **Wipe tower:** consumes each `ToolChange` and purges per change. Per-line
  coloring therefore raises the tool-change count and purge volume. This is a
  known cost, not a correctness break; documented, not engineered around.

### Precedence

When authored-coloring is granted, the per-path tool **overrides the region's
resolved tool**, including a material-variant tool. Rationale: the setting is the
user's explicit opt-in for that role, so self-coloring winning over the region
default is the requested behaviour, not a conflict.

---

## 4. "Coloring according to tiling" — concrete semantics

- **Driver:** each emitted scan segment receives `tool = f(tiling_index)`, where
  the tiling index is a stable property of the dragon-curve tiling (e.g. fold
  order / generation / segment ordinal). The mapping is **deterministic** so two
  runs are byte-identical (reproducibility invariant).
- **Tool availability:** expose a **tool-count query** to modules (a config key
  or host service) — this closes the earlier-identified gap that modules have no
  way to know how many tools exist. The module wraps its index into that range.
- **Out-of-range:** the host validates any emitted `tool` < tool count and strips
  or clamps out-of-range values.
- **Edge cases in scope:** number of distinct colors > tool count (wrap);
  reproducibility across runs; sparse polygon with holes; per-region overrides of
  the dragon's config keys.
- **Edge cases explicitly out of scope:** bridges, top/bottom solid, per-layer
  angle alternation (see §5).

---

## 5. Minimal scope

**In scope (first community module):**
- One claim: `claim:sparse-fill`. Emits only sparse paths over the sparse fill
  polygon.
- One pattern: the dragon-curve tiling.
- **Full config support:** a complete `config.schema` for the module
  (density, angle, speed, line widths) plus dragon-specific keys (tiling
  depth/generation, color mapping), participating in per-region overrides the
  same way `rectilinear-infill` does.
- The `claim:authored-coloring` disclosure and the tiling-driven color mapping.
- The labeled-example packaging (§1), committed `.wasm`, build script, and
  documented manual slice test.

**Explicitly out of scope:**
- Other fill roles (`claim:top-fill` / `bottom-fill` / `bridge-fill`).
- Bridges, solid infill, per-layer 90° angle alternation.
- Multi-role modules (a la gyroid).
- Any CI integration for the Go module.
- Real community-module authoring in this repo (this example is the exception).

---

## 6. Feasibility gate (prerequisite to any work)

**Before any implementation work, update both crates, then recheck feasibility.**

The Go and MoonBit probes (records: `docs/feasibility-probes/go-wasm.md`,
`docs/feasibility-probes/moonbit-wasm.md`) were measured on **`wit-bindgen`
0.57.1** and **`wasmtime` 43.x**, and both returned "not loadable-and-correct"
(Go: WASI blocker; MoonBit: UTF-16 vs UTF-8 string-encoding mismatch). Those
verdicts are **not final** — they are contingent on the toolchain versions tested.

**Step 0 (mandatory, before any other work):**
1. Update the workspace to **`wit-bindgen` 0.60.0**
   (https://crates.io/crates/wit-bindgen/0.60.0).
2. Update the host to **`wasmtime` 47.0.3**
   (https://crates.io/crates/wasmtime/47.0.3).
3. Re-run the Go and MoonBit feasibility probes against the updated toolchain,
   because either update may change the blockers:
   - a newer `wit-bindgen` may add a **UTF-16 host string-encoding option**
     (which would unblock MoonBit), and
   - a newer `wasmtime` may add **WASI preview2** support to the host linker
     (which would unblock Go).
4. Record the re-check verdicts in `docs/14_submodule_programming_languages.md`
   (§Community-module context) before proceeding.

**Purpose of the gate:** prove a non-Rust language can emit a wasm32 **component**
that satisfies the `Layer::Infill` stage contract (exports `run`, imports the host
services, config, and IR handle types) and is loadable-and-correct under `pnp_cli`,
or confirm the fallback.

**Fallback if the gate still fails:** the Dragon Curve's source computes the
tiling and color mapping; a thin host-side Rust wrapper (reusing the SDK)
satisfies the WIT contract and calls the foreign logic. The foreign language stays
out of the WIT seam. The spec's packaging/claim/coloring design is unchanged
either way.

This spec may be archived, so the probe briefs and verdicts live in the living
document `docs/14_submodule_programming_languages.md` (§Community-module context),
not here.

---

## 7. Docs, glossary, and ADR deliverables

- **CONTEXT.md:** add **Community module** and **Authored coloring** glossary
  entries (see §1 and §2 definitions).
- **ADR:** author an ADR recording the authored-coloring mechanism — per-path
  `tool-index` carrier on `extrusion-path3d`, the two-sided grant (setting ∩
  disclosed claim), host-strips-ungranted enforcement, and color-over-region
  precedence. This is a WIT-rippling, hard-to-reverse, deliberate deviation with
  no OrcaSlicer precedent (see `docs/DEVIATION_LOG.md` discipline).
- **Docs note:** update `docs/` and `CLAUDE.md` so contributors know real
  community modules are authored in forks as submodules, never added here; the
  committed Dragon Curve is a labeled example only.
