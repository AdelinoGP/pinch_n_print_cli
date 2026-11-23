# t97 — the branch-distance "match" is a false premise

Route: [Match Orca's tree-support branch distance in the matched job](../../issues/97-match-branch-distance-fairness.md).
HEAD: `f48a0488` (2026-10-30). No production or profile change.

## Verdict

**REJECTED on fairness grounds — the premise is false.** The matched job's
Orca side does **not** consume `tree_support_branch_distance = 5 mm`. Under
`support_type = tree(auto)` with `support_style = default`, canonical resolves
the tree style to **organic** and the organic engine reads **only**
`tree_support_branch_distance_organic`, which the matched job resolves to
**1 mm**. PnP's matched job also runs at **1 mm**. The effective distances are
already matched; moving PnP's normal key to 5 mm would make PnP do one fifth of
Orca's branch spacing in a *different* engine, i.e. an unfair work reduction,
not a match.

The real supports-on fairness gap is the **style substitution**: Orca runs the
organic engine, PnP aliases every canonically-organic input to the classic
engine's **Strong** style because the organic engine is not implemented. That
is already `DEV-156`; it is not new, but the map has been treating the distance
row as the supports-on fairness lead, which it is not.

## Phase 0 — the canonical dispatch

`support_style = default` is not style-neutral. On a tree family canonical
resolves it to organic, then forks to the organic engine:

- `SupportParameters`' substitution chain: `grid`/`snug` degrade to
  `smsDefault`; `smsDefault` on a tree family becomes `smsTreeOrganic` (the
  code comment reads `// Orca: use organic as default`). Verified at upstream
  `main` `33909a51cd`, at the fork's `pnp/main` `97d1745e5a`, and re-confirmed
  against upstream `main` fetched 2026-10-30.
- `PrintObject::_generate_support_material` and `TreeSupport::generate` both
  route `smsTreeOrganic` to `generate_tree_support_3D`, the organic engine.
- The organic engine's config struct (`TreeSupportCommon.hpp`) reads
  `tree_support_branch_distance_organic` for
  `support_tree_branch_distance` (re-confirmed on upstream `main`,
  2026-10-30). The normal key is not consulted on that path; it still appears
  in the G-code appendix only because the appendix dumps every resolved option.

The `t82` appendix makes both values explicit on all four supports-on cells:
`support_style = default`, `support_type = tree(auto)`,
`tree_support_branch_distance = 5`, `tree_support_branch_distance_organic = 1`.

## The PnP side (in-repo)

- `TreeSupportStyle::from_config` maps `tree_slim`/`tree_strong`/
  `tree_hybrid` to themselves and **every other tree-family style —
  including `default` and `organic` — to `Strong`**, because the organic
  engine is not implemented.
- `tree-support-planner.toml` documents the same chain above
  `[config.schema.support_style]`, and declares
  `tree_support_branch_distance` default **1.0** and `support_style` default
  **`"default"`**.
- `DEV-156` records the substitution, its product rationale, the silent
  `default` alias and the explicit-`organic` warning, and points at the queued
  organic-engine port.
- The matched profiles (`configs/pnp-classic-supports-on.json`,
  `configs/pnp-arachne-supports-on.json`) set **neither** branch-distance key,
  so PnP runs its 1.0 mm default.

## What t88 actually measured

The t88 arm set `tree_support_branch_distance = 5.0` on PnP only and measured
184,115 → 176,869 ms (**−7,246 ms, −3.94 %**) on base classic supports-on, with
G-code −0.83 % and a changed SHA. That is a real PnP work reduction: PnP's
Strong engine honours the normal key, so 1 mm → 5 mm removes branch placement
work. It is **not** a match to Orca, whose active organic key stays at 1 mm.
The measurement stands as a bound on PnP's branch-distance sensitivity; it does
not license the change.

## Disclosure — the installed Orca could not be exercised

The map's Orca baseline tool is currently not runnable through the rig. The
install at `C:\Program Files\OrcaSlicer` was updated on 2026-10-27 (its CLI
reports **2.5.0-beta**) and now rejects the rig's file-path `--load-settings`
form:

```
Slic3r::CLI::run found error, exit
can not resolve preset <path>: Preset was not found in the loaded bundle
```

The install no longer carries `resources/profiles/BBL/{machine,process}` (it
ships `BBL.opc`), and the in-tree checkout's CLI source predates the change.
Phase 0 therefore rests on the canonical source and the in-repo deviation
record, not on a fresh Orca run. This also means **no Orca-side scoreboard
refresh can run until the rig's preset loading is updated or a runnable stock
Orca is restored** — a separate infrastructure question, not this ticket's.

## Consequences for the map

- The 5 mm branch-distance lead is **dead**; no profile edit, no campaign.
- The generated deviations table's `tree_support_branch_distance` row
  (PnP `1.0` vs Orca `5.0`) compares the **inert** normal key under the matched
  organic style. The effective key, `tree_support_branch_distance_organic`, is
  `1.0` on both sides and is not registered by PnP (no organic engine).
- Any supports-on comparison in this map compares **Orca organic** against
  **PnP Strong**. That is the fairness question the distance row was hiding.
  Closing it is either the queued organic-engine port (parity work, and not
  obviously a PnP speed win — organic is the more expensive engine) or forcing
  the Orca matched profile to `support_style = tree_strong` so both sides run
  the class PnP implements (a protocol change whose effect on Orca's wall is
  unmeasured). Both need a human ruling; neither is a candidate in this ticket.

## Evidence

- `verify.py` — checkout-only re-derivation of every in-repo fact above, with
  five negative controls (`python verify.py --self-test` exits 0).
- `REPRODUCE.md` — the two-layer reproduction, including the external canonical
  check.
