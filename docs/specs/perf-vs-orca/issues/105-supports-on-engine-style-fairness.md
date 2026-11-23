# Supports-on engine-style fairness: price Orca under `tree_strong`

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; measurement + human ruling — **pin Orca to
`tree_strong`**. The pin costs Orca **+39.6 / +44.4 %** wall on the two base
cells and **+7.6 / +9.5 %** on the two benchy cells, changes the job
(**+23.5–35.3 %** bytes; support-interface sections 230 → 423 base, 7 → 57
benchy), and moves the four supports-on ratios by **−7.1…−30.8 %** — best cell
4.3x, park still needs 65–73 % cuts. Implemented in the standing rig; a
supports-on Orca scoreboard re-derivation is the follow-up. See ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named), five routes were offered against t104's
empty frontier, and the human selected supports-on engine fairness
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the post-t104 fog route "The supports-on job is style-unmatched:
Orca organic vs PnP Strong" (t97 / `DEV-156`), which t104 positioned as ~50 % of
the base classic-on cell

## Question

The four supports-on cells compare **Orca's organic engine** against **PnP's
Strong substitution**:
`support_style = default` on a tree support family resolves to `smsTreeOrganic`
in canonical `SupportParameters`, and `PrintObject::_generate_support_material`
forks that style to `TreeSupport::generate`'s organic path
(`generate_tree_support_3D`). PnP implements only the classic engine, and
`TreeSupportStyle::from_config`
(`modules/core-modules/tree-support-planner/src/lib.rs`) aliases every
canonically-organic input (`default`, `organic`, `grid`, `snug` on a tree family)
to **Strong** — `DEV-156`, recorded as a deliberate product decision with the
organic port queued as item 7 of
`docs/specs/support-generation-remediation-plan.md`.

[t104](104-prepass-vs-per-layer-attribution.md) measured the site at ~50 % of
the base classic-on cell (`PrePass::SupportGeometry` 91,902 ms = 40.1 % plus
`PrePass::SupportAnalysis` 22,254 ms = 9.7 % of 228,997 ms) and 35 % of benchy
classic-on (3,260 + 1,096 ms of 12,458 ms), on the map's two worst ratios
(8.25x and 5.43x). The mismatch is therefore no longer a footnote: it sits on
the cells that bind the destination.

The fairness contract requires matched settings. Two ways to match the style:

- **port organic** — canonical-parity work (queue item 7, canonical
  `TreeSupport3D.cpp` plus its `TreeModelVolumes` avoidance model, ~4,200
  lines). Organic is the *more expensive* canonical engine, so the port would
  not move the destination; it would repair the comparison.
- **pin the Orca profile to `tree_strong`** — a protocol change that makes both
  sides run the classic class PnP implements. Its effect on **Orca's** wall is
  unmeasured.

This ticket measures the second so the human can rule. **How much wall does
Orca's `tree_strong` arm cost relative to its canonical organic arm on the four
supports-on cells, what does it change in Orca's output, and does the pin move
any cell's ratio enough to matter?**

## Scope

- **Cells:** the four supports-on cells — {benchy, base} × {classic, arachne}.
  The supports-off cells are unaffected and are not re-measured.
- **Arms (Orca only):** `support_style` left at its resolved default (organic on
  the matched `tree(auto)` job) versus `support_style = tree_strong`. The PnP
  side is untouched; its denominators are t103's paired ordinary medians.
- **Walls:** uninstrumented, process creation-to-exit, 1 warmup + 4 measured per
  arm per cell, arms interleaved at the run level, median claimed, quiet machine
  (load recorded before, between rounds and after; starvation rule per the
  standing protocol).
- **Disclosure:** G-code bytes, SHA, TYPE/section counts, resolved
  `support_type` and `support_style`, and a non-vacuity check that the two arms
  differ (identical bytes would mean the override did not take).

## Deliverable

1. A paired per-cell table: Orca wall / CPU / cpu-wall under organic and under
   `tree_strong`, the delta, and the ratio against t103's PnP medians (raw, not
   resolution-corrected).
2. The style's price in output: bytes, section counts and support-section
   counts, so the human can see whether the pinned job is materially different.
3. A recommendation to the human among **pin** / **port organic** /
   **keep-disclose**, with the measured numbers attached and the port's queue
   status restated from `docs/specs/support-generation-remediation-plan.md`.

## Stop rule

Measurement-only. No production edit, no PnP config change, no parity port, no
scoreboard rewrite. The pin is a **protocol decision for the human**; this
ticket does not adopt it. If Orca's CLI rejects `--support-style=tree_strong` or
the appendix never resolves to `tree_strong`, the ticket resolves as blocked
with that fact rather than substituting another style.

## Method

- Extend the standing rig with an opt-in `-OrcaSupportStyle` parameter
  (default empty — standing behaviour unchanged) so the candidate arm's style is
  appended after the matched-job keys, and add `support_style` to the disclosed
  appendix excerpt and to the fail-closed assertion so every row proves which
  style it ran.
- Campaign runner, reduced rows and `verify.py --self-test` under
  `evidence/t105-tree-strong/`; the verifier must exit 0 from a plain checkout.

## Answer (2026-11-02) — measurement complete; human ruled **pin to `tree_strong`**

**The style price is real: pinning Orca to `tree_strong` costs it +39.6 % /
+44.4 % wall on the two base cells and +7.6 % / +9.5 % on the two benchy cells,
and changes the job materially (+23.5–35.3 % G-code bytes; support-interface
sections 230 → 423 base classic-on, 7 → 57 benchy classic-on).** It moves the
four supports-on ratios by −7.1 % to −30.8 % and does **not** approach the park.

| cell | Orca organic | Orca `tree_strong` | delta | ratio (within-take) organic → strong | applied to the record (t103 Orca × multiplier) |
| --- | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 20.449 s | 29.532 s | **+9.083 s / +44.42 %** | 7.87x → **5.45x** | 8.25x → **5.71x** |
| base arachne-on | 20.514 s | 28.627 s | **+8.113 s / +39.55 %** | 7.86x → **5.63x** | 7.00x → **5.01x** |
| benchy classic-on | 2.422 s | 2.651 s | **+0.230 s / +9.48 %** | 5.12x → **4.68x** | 5.43x → **4.96x** |
| benchy arachne-on | 2.655 s | 2.857 s | **+0.202 s / +7.60 %** | 4.60x → **4.28x** | 4.78x → **4.45x** |

The required cut to the park stays **65–73 %** per cell. Base pairs are 4/4
positive in both cells (+5.39…+9.75 s); benchy is 3/4 and 2/2 positive on
2.5 s-scale cells, with the exclusions and the load events disclosed.

**Output corroboration of the substitution:** on the two base cells PnP's byte
volume is within **3.4–5.1 %** of Orca's `tree_strong` arm and **27.4–30.8 %**
above organic (`DEV-156` says PnP runs Strong for every canonically-organic
input). On benchy the two arms bracket PnP.

**The ruling.** Three options, with what each commits to:

1. **Pin** — add `support_style = tree_strong` to the matched Orca profile
   (`New-OrcaOverrideArgs` + the per-run assertion) and disclose the pin beside
   every supports-on ratio. Like-for-like engine class; the cost is comparing
   against a **non-default** Orca job, and the four supports-on ratios move as
   above. No PnP work changes; the scoreboard of record would need the four
   supports-on Orca cells re-run under the pin.
2. **Port organic** — keep Orca at its canonical default and carry the organic
   engine as parity work (queue item 7 of
   `docs/specs/support-generation-remediation-plan.md`, canonical
   `TreeSupport3D.cpp` + `TreeModelVolumes`, ~4,200 lines). Exactly matches
   Orca's default job; no wall claim either way, and organic is the more
   expensive canonical engine, so the destination does not move.
3. **Keep-disclose** — no change; keep the t97 engine-style disclosure on every
   supports-on ratio and treat the four supports-on cells as an engine-mismatched
   comparison.

**Verdict on the ticket's own question:** the pin is a genuine fairness fix with
a measured, asymmetric price, and it is not a route step — even the best
supported-on cell stays ~4.3–4.7x away. Evidence:
[`evidence/t105-tree-strong/FINDINGS.md`](../evidence/t105-tree-strong/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 74/74 claims, 6/6 negative
controls rejected).

### Ruling and implementation (2026-11-02)

The human chose **pin to `tree_strong`** from the three options above. The pin
is implemented in the standing rig
(`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1`):

- `-OrcaSupportStyle` now defaults to `'tree_strong'` (the ruling) and is
  appended after the matched-job overrides **only on supports-on runs**;
  `vendor` (or an empty value) remains the escape hatch for a disclosed
  organic/vendor-default arm (the t105 baseline arm uses it). Supports-off runs
  resolve the vendor default and are asserted as `default`.
- The per-run appendix assertion (`Test-OrcaEvidence`) fails closed on any row
  whose resolved `support_style` does not match its arm, and `support_style` is
  now in the disclosed excerpt for every row.
- Smoke after the change: supports-on resolves `tree_strong`, supports-off
  resolves `default` (`target/matched-pair/t105-pin-*`, gitignored).

**What is not done yet.** Ticket 103's supports-on Orca medians were measured
**before** the pin. The ratios quoted in this Answer's transfer column are
derived (`t103 Orca × measured style multiplier`), not measured. A supports-on
Orca scoreboard re-derivation under the pin — and the matching fairness-contract
update — is the follow-up take; until it lands, no supports-on scoreboard row
may be quoted as post-pin.

## Comments

- 2026-11-02: claimed before any work. Frontier re-derived at HEAD `81219a66`:
  104/104 children resolved, no open child, no claim. Five routes were offered
  from t104's post-resolution state (closing-chain rewrite shape; supports-on
  engine fairness; redraw the destination; park the map; lift the park and
  harvest per-layer leads) and the human selected **supports-on engine
  fairness**.
- 2026-11-02: campaign **paused at the human's request** (the machine was needed).
  State at the pause: **26 Orca runs complete** (8 warmups + 18 measured; paired
  rounds 1 and 2 complete for all four cells, round 3 partial), 14 runs short of
  the 4-round target. Raw batches are preserved under
  `target/matched-pair/t105-*`; the interrupted run left an empty
  `t105-organic-r3-benchy-arachne-on` batch that a `-Resume` run re-runs rather
  than skips. No reduction, verifier run or verdict exists yet — the numbers
  below are **not** a claim.
- 2026-11-02 (partial read, superseded by the campaign): the two complete
  paired rounds put Orca's `tree_strong` arm above organic on every cell. The
  final four-round numbers are in the Answer and
  [`evidence/t105-tree-strong/FINDINGS.md`](../evidence/t105-tree-strong/FINDINGS.md);
  the earlier ad-hoc ratio estimates in this comment are withdrawn.
