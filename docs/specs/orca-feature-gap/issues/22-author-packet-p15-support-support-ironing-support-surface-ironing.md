# 22 — Author packet P15 — Support / Support ironing — support-surface-ironing

Type: task
Status: resolved
Assignee: wayfinder session — claimed 2026-09-02
Blocked by: 06, 106
Map: ../map.md

## Question

Author the spec packet for **P15 — Support / Support ironing — support-surface-ironing** — 2 keys, Tier A plumbing, owner support-surface-ironing. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P15 — Support / Support ironing — support-surface-ironing), amended by ticket 07:

`support_ironing_pattern`, `support_ironing`

The `support_ironing` key is the 07 reclassification: an independent bool so
support-interface ironing no longer rides the shared `ironing_enabled` bool
(declared identically by both support-surface-ironing and
top-surface-ironing — the two Orca features cannot be toggled independently
today).

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the authoring decision is recorded and the single atomic change is
implemented directly; no retained packet is required.

## Answer

The generated draft packet was intentionally discarded. Its authoring analysis
is retained below, and the single atomic change was implemented directly in
session.

**P15 covers one key, not two.** The two scoped keys were re-derived against
canonical and against this tree, and they land in different places.

**`support_ironing` — wired.** Canonical `coBool`, default `false`, assigned to
`SupportParameters::ironing` by the `SupportParameters` constructor
(`Support/SupportParameters.hpp`) and consumed by `generate_support_toolpaths`
(`Support/SupportCommon.cpp`), where `support_params.ironing &&
!top_contact_layer.empty()` captures the polygons a later block fills at
`ExtrusionRole::erIroning`. At authoring, this key had zero occurrences in the
tree. The port had the behaviour but not the key: `SupportSurfaceIroning::from_config`
(`modules/core-modules/support-surface-ironing/src/lib.rs`) gates on a
PnP-invented `ironing_enabled` bool that `top-surface-ironing` **also**
declares, independently. That is not only a naming gap — it is a reachability
bug in both directions: an Orca configuration setting `support_ironing = 1`
cannot enable support ironing at all, and a user setting `ironing_enabled` to
reach *top-surface* ironing silently enables support ironing too. The direct
implementation makes `support_ironing` the module's sole gate (grilling ruling **Q10(b)**;
Authoring rule 5 forbids the two-gates-one-decision alternative). Default
`false` equals the current default and the current absent-key behaviour, so the
**default path is byte-identical**; the behaviour change is at `true`, pinned by
AC-2, and reachability through the real host config path is pinned by AC-3
(the support integrated-parity contract test).

**`support_ironing_pattern` — returned to the queue as unimplemented.**
Canonical `coEnum` over `InfillPattern`, values `rectilinear` / `concentric`,
default `ipRectilinear`, consumed as
`Fill::new_from_type(support_params.ironing_pattern)`. That is cross-module
algorithm selection, so Authoring rule 4 and grilling **Q3(a)** apply verbatim:
holder-only, never a declared input key. This port has no support-ironing claim
(`[claims] holds = []`), no holder key, and no concentric filler — standing that
seam up is Tier C, not the Tier A this key was tiered at. Under Authoring rule 1
it is therefore **left out of the packet**, not declared with a gap. AC-N2 pins
its honest absence from the tree. Records updated: the tier row in
[`04-asset-tier-assignment.md`](./04-asset-tier-assignment.md) and the P15 entry
in [`05-asset-packet-list.md`](./05-asset-packet-list.md) now carry the
returned-to-queue ruling and name the missing feature.

**Tier A confirmed** for `support_ironing` — the decision point already exists
(`run_support_postprocess`'s `if !self.support_ironing` early return); the direct implementation plumbs
a canonical key into it. Rule 4 does not fire on it: a bool that turns one
implemented behaviour on and off is in-module branching, not cross-module
algorithm selection (the Q8 trigger test).

**Two pre-existing divergences recorded, neither created here** (design-local
labels, *not* `DEVIATION_LOG.md` rows — no row filed, no human sign-off
consumed):
- `DIV-267-A` — **the port irons a different subject than canonical does.**
  Canonical irons the support **top contact (interface) layer**'s polygons. This
  module runs at `Layer::SupportPostProcess` and its trait entry
  (`LayerModule::run_support_postprocess`, `crates/slicer-sdk/src/traits.rs`)
  receives only `&[SliceRegionView]` — there is no support-interface geometry at
  this seam to select instead. It scan-fills every slice-region polygon it is
  handed. Closing this means carrying support contact/interface polygons across
  the WIT boundary: an IR field plus a WIT accessor. Graduated to the map's fog.
- `DIV-267-B` — canonical reaches the ironing branch only inside the
  `top_interfaces` arm (interface layers requested). Unexpressible here, and a
  consequence of `DIV-267-A` rather than an independent decision.

**Preflight finding, fixed in place:** the packet cited `FEEDRATE_KEYS`, which
does not exist in this tree — the symbol is `SPEED_KEYS`
(`crates/slicer-ir/src/feedrate.rs`). Corrected in all three places. The
fictional name is inherited from this map's own
[`key-correction-inventory.md`](./key-correction-inventory.md) Q11(a) row, which
still carries it.

**Out of scope, flagged not folded in:** grilling **Q11(a)** renames this
module's `ironing_speed` → `support_ironing_speed` and leaves its `SPEED_KEYS`
membership open. Same manifest, but a PnP naming fix with no canonical key and
its own unadjudicated question — filed as a follow-up ticket rather than
smuggled into a packet whose acceptance is about a different key.

No user rulings were required; no deviation rows filed; `ORCA_CONFIG_PADDING`
untouched in both directions (Authoring rule 2 — and re-derived: neither key is
in that table).

## Direct implementation

The support manifest now declares `support_ironing` with the canonical false
default, and `SupportSurfaceIroning::from_config` reads it as the sole gate.
The module tests cover true, false, absent, and legacy `ironing_enabled` input;
the support integrated-parity contract uses the canonical key; and the module
owns a direct TOML schema guard. The generated config reference was refreshed,
the guest artifact was rebuilt and freshness-checked, and the focused module
and parity tests pass.
