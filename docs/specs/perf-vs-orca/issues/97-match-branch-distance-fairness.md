# Match Orca's tree-support branch distance in the matched job

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; **REJECTED on fairness — the premise is false**:
Orca's `support_style = default` on a tree family resolves to **organic**, whose
active key is `tree_support_branch_distance_organic = 1 mm`, already matched by
PnP's 1 mm default. The normal `tree_support_branch_distance = 5 mm` is inert on
that path, so the t88 arm was a work reduction, not a match. No profile edit,
no campaign. See ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human said "continue the
perf vs orca map"; the frontier re-derivation at HEAD `f48a0488` found **96/96
children resolved**, the session offered six measured fog routes, and the human
selected **match 5 mm branch distance**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: fog (t83/t88 branch-distance observation; t82 fairness audit)

## Question

Is the matched supports-on job actually unmatched on the tree-support branch
distance — i.e. is the **normal** key `tree_support_branch_distance = 5 mm` the
key Orca's `support_type = tree(auto)` / `support_style = default` consumes —
and if so, does setting the PnP matched profiles to 5.0 reproduce the t88
**−3.94 %** wall win at current HEAD, or would it be an unfair work reduction?

## Phase 0 — the decisive fork (before any profile edit)

`support_style = default` is **not** style-neutral. Canonical resolves it by
support family, and the normal key is inert on the matched path:

- `SupportParameters`' substitution chain: on a tree family, `smsDefault` →
  `smsTreeOrganic` (the code comment reads `// Orca: use organic as default`),
  while `grid`/`snug` degrade to `smsDefault`.
- `PrintObject::_generate_support_material` and `TreeSupport::generate` route
  that style to `generate_tree_support_3D`, the organic engine.
- The organic engine's config struct (`TreeSupportCommon.hpp`) reads **only**
  `tree_support_branch_distance_organic`. The normal
  `tree_support_branch_distance` is never consulted on this path.

So the matched job's effective distance is the **organic** key, which the t82
appendix resolves to **1 mm** — the same value as PnP's default. The normal key
the deviation table compares (`5 mm`) is inert under the matched style.

PnP's side is in-repo and already logged: `TreeSupportStyle::from_config`
aliases `default`/`organic`/`grid`/`snug` on a tree family to **Strong** (the
classic engine), because the organic engine is not implemented. That is
**DEV-156**, whose mitigation is the organic-engine port queued in
`docs/specs/support-generation-remediation-plan.md`.

The fairness gap is therefore the **style substitution**, not the distance.

## Premise (measured)

| fact | value | source |
|---|---|---|
| PnP default `tree_support_branch_distance` | 1.0 | `docs/15_config_keys_reference.md`; planner manifest |
| Orca resolves `tree_support_branch_distance` | 5.0 | t82 appendix, all four supports-on cells |
| Orca resolves `tree_support_branch_distance_organic` | 1.0 | t82 appendix, all four supports-on cells |
| PnP matched profiles set either key | no | `evidence/matched-pair/configs/pnp-{classic,arachne}-supports-on.json` |
| PnP `support_style` resolution | `default` → Strong | `TreeSupportStyle::from_config`; DEV-156 |
| t88 5 mm arm, base classic-on | 184,115 → 176,869 ms (−7,246 ms, −3.94 %), G-code −0.83 %, SHA changed | t88 FINDINGS |

Floor **0.80 %** of the cell.

## Plan

1. **Phase 0** — confirm the canonical dispatch on the installed binary by
   key-liveness at `support_style = default`: changing the **organic** key must
   move the output, changing the **normal** key must not. Record the source
   citations (canonical checkout `main` `33909a51cd` and fork `pnp/main`
   `97d1745e5a`) and the binary/version disclosure. Stop if the premise fails.
2. **If the normal key is live** (premise holds): edit the two supports-on
   matched profiles, record fresh supports-on references, prove 16 cells
   (supports-off byte-identical to the current references; supports-on
   deterministic, ordinary ≡ accelerated, zero degraded/fatal/non-fatal,
   support roles present), then run the four supports-on cells × both modes ×
   6 pairs.
3. **If the organic key is live** (expected): **no profile edit**. Record the
   rejection and correct the map's fog: the 5 mm lead is dead, the effective
   organic key is already matched at 1 mm, and the real supports-on fairness
   gap is DEV-156's organic-vs-Strong style substitution.

## Pre-declared gate

- The change is a **fairness match, not an optimization**: the profile edit is
  justified only if Phase 0 proves the normal key is the live canonical key.
  A 5 mm arm that leaves Orca at 1 mm is a work reduction and is rejected
  regardless of wall.
- Wall gate (only if Phase 0 passes): paired median wall non-positive in all
  four supports-on cells, both modes, zero starvation exclusions; process CPU
  must not contradict (no rise above the pre-declared +3.0 %).
- Output gate: supports-off byte-identical to the current references;
  supports-on deterministic (one SHA per arm/cell across repeats), ordinary ≡
  accelerated, zero degraded/fatal/non-fatal, support roles present.
- Disclosures: geometry changes are intended; the PnP production default stays
  1.0 mm unless separately ruled; no scoreboard refresh in this ticket.

## Answer

**Measured REJECTED on fairness — the premise is false. No profile edit, no
campaign, no production change (2026-10-30, HEAD `f48a0488`).**

Phase 0 resolved the fork against canonical source, the in-repo deviation
record, and the matched job's own resolved appendix:

1. **Canonical resolves `default` to organic.** `SupportParameters`'
   substitution chain turns `smsDefault` into `smsTreeOrganic` on a tree family
   (`// Orca: use organic as default`). `PrintObject::_generate_support_material`
   and `TreeSupport::generate` both fork that style to
   `generate_tree_support_3D`, the organic engine. The organic engine's config
   struct (`TreeSupportCommon.hpp`) reads **only**
   `tree_support_branch_distance_organic`; the normal key is not consulted on
   that path. Re-confirmed against upstream `main` fetched 2026-10-30.
2. **The appendix shows both keys, but only the organic one is live.** All four
   supports-on cells resolve `support_style = default`,
   `support_type = tree(auto)`, `tree_support_branch_distance = 5`,
   `tree_support_branch_distance_organic = 1`. Effective Orca spacing: **1 mm**.
3. **PnP is already at 1 mm.** The matched profiles set neither key, so PnP
   uses its `tree_support_branch_distance` default **1.0 mm**
   (`tree-support-planner.toml`). Effective spacing: **1 mm**. Matched.
4. **The styles are not matched.** `TreeSupportStyle::from_config` aliases every
   canonically-organic input (`default`, `organic`, `grid`, `snug` on a tree
   family) to the classic engine's **Strong** style, because the organic engine
   is not implemented. That is `DEV-156`, with the organic-engine port queued in
   `docs/specs/support-generation-remediation-plan.md`.

The t88 arm (`tree_support_branch_distance = 5.0`) measured a real **−3.94 %**
(184,115 → 176,869 ms, G-code −0.83 %, SHA changed) because PnP's Strong engine
does honour the normal key. Adopting it would make PnP place branches at **one
fifth** of Orca's effective spacing in a different engine — an unfair work
reduction, exactly what the fairness contract forbids. It is a bound on PnP's
branch-distance sensitivity, not a match.

**Disclosure — the installed Orca could not be exercised.** The install at
`C:\Program Files\OrcaSlicer` was updated 2026-10-27 (its CLI reports
**2.5.0-beta**) and now rejects the rig's
file-path `--load-settings` form (`can not resolve preset <path>: Preset was not
found in the loaded bundle`) and no longer carries
`resources/profiles/BBL/{machine,process}` (it ships `BBL.opc`). Phase 0
therefore rests on canonical source plus the in-repo deviation record. This also
means **the map's Orca-side scoreboard cannot be refreshed until the rig's
preset loading is updated or a runnable stock Orca is restored** — recorded in
the map's Notes as an infrastructure blocker.

**What survives for the map.**

- The 5 mm branch-distance lead is **dead**; the fog patch is replaced by the
  style question it was hiding.
- The generated deviations table's `tree_support_branch_distance` row (PnP
  `1.0` vs Orca `5.0`) compares the **inert** normal key under the matched
  organic style. The effective key (`_organic`) is `1.0` on both sides and is
  not registered by PnP.
- Every supports-on comparison in this map compares **Orca organic** against
  **PnP Strong**. Closing that is either the queued organic-engine port (parity
  work; not obviously a PnP speed win) or forcing the Orca matched profile to
  `support_style = tree_strong` so both sides run the class PnP implements (a
  protocol change whose effect on Orca's wall is unmeasured). Both need a human
  ruling; neither is a candidate here.

Evidence: [`evidence/t97-branch-distance/`](../evidence/t97-branch-distance/FINDINGS.md)
(`verify.py` exits 0 checkout-only, `--self-test` rejects 5/5 negative controls).

