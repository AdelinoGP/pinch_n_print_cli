# Matched settings vs the 8 mm brim: close the asymmetry or keep it disclosed?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-09, session 38; human ruling — **close both
  adhesion/priming asymmetries by canonical port + shared authored values**
  (brim 8 mm; skirt 6 loops / 3.0 mm / 1 layer), staged into
  [t135](135-cross-layer-occupancy-batching.md) as its Stage-1 re-take;
  disclosure declined. See ## Answer)
Assignee: dev (session 38 — the human said "Continue the perf vs orca map"
  (no ticket named); 130 was the first frontier ticket in order and was
  resolved the same session through a four-round grilling)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Destination arithmetic after t123: re-aim, rewrite, or park?](128-destination-arithmetic-ruling.md)'s
  secondary decision (2026-11-08).

## Question

The destination says **"matched settings"**, but the two arms are not printing
the same adhesion geometry:

- PnP prints its **8 mm brim** in every cell. It is the skirt-brim module's own
  default (`modules/core-modules/skirt-brim/skirt-brim.toml` declares
  `brim_width = 8.0`), the matched config authors no brim key, and the emitted
  PnP appendix resolves `brim_width = 8` in all 16 PnP groups.
- The pinned Orca rig prints **none** (`brim_width = 0`, asserted in all 32
  runs).

Its slice-time price is small — `com.core.skirt-brim` measured at **~235 ms** for
the whole print ([t103](../evidence/t103-orca-rebaseline/SCOREBOARD.md)) — but it
is a job difference carried by every ratio, and it is a separate key from the
filament leg t98 repaired.

Which is it:

- **Match at 0.** Author `brim_width = 0` on both arms. PnP stops printing its
  product default; ~235 ms leaves PnP's side; every PnP G-code reference moves
  (a brim is real extrusion, not a no-op).
- **Match at 8.** Author `brim_width = 8` on both arms. Orca prints the same
  adhesion geometry and gets slower by its own brim; PnP's references stay. A
  same-value variant (e.g. 5 mm, Orca's 2.5 default) is equally matched and
  needs the rig pin updated to assert it.
- **Keep the disclosure.** Reword the destination to *"matched except the
  disclosed adhesion key"*, and keep the disclosure beside every scoreboard.
  No re-take and no reference move.

## Acceptance

- A recorded answer in the map's **Destination** wording or **Notes**, naming
  the value (or the disclosure) and which ratios it applies to.
- If the key is matched, a ticket for the re-take that moves the reference set
  and re-derives the scoreboard; the re-take keeps the standing 16-cell and
  all-8-cell gates.
- No production edit in this take.

## Answer

**Ruling (2026-11-09, session 38; the human, grilling): close both asymmetries
— the brim and a second one the take surfaced — by canonical port + shared
authored values, staged into
[Cross-layer occupancy offset batching](135-cross-layer-occupancy-batching.md)
as its Stage-1 re-take. "Keep the disclosure" is declined.**

1. **The job keeps one adhesion feature and one priming skirt, matched at
   shared values.** `brim_width = 8` on both arms; the skirt closes at **6
   loops / 3.0 mm / 1 layer** (`skirt_loops = 6`, `skirt_distance = 3.0`,
   `skirt_height = 1`). PnP authors the keys in the four matched configs (they
   author none today); the Orca rig amends `New-OrcaOverrideArgs`
   (`--brim-width=0` → `8`, plus `--skirt-loops=6 --skirt-distance=3
   --skirt-height=1`) and both arms' resolved values are asserted per run
   fail-closed (`Test-OrcaEvidence` plus a PnP-side appendix assert). The t103
   "pinned back to 2.4-era values" doctrine re-scopes: pins serve **the matched
   job**, not a version era.
2. **Both surfaces go canonical (parity work licensed for this).** Port
   canonical `make_brim` (`Brim.cpp`, pinned `v2.5.0` = `40ecc66f78`) and the
   skirt construction in `Print.cpp` (object-convex-hull anchored) into
   `com.core.skirt-brim`, replacing `SkirtBrim::generate_brim_entities` /
   `generate_skirt_entities` (`modules/core-modules/skirt-brim/src/lib.rs`)
   bounding-box rectangles. The port carries the standard OrcaSlicer
   attribution header (`docs/ORCASLICER_ATTRIBUTION.md`); `min_skirt_length` is
   pinned as part of it; the `skirt_per_object` semantics difference (Orca
   chain `1`, PnP global) is disclosed as single-object-moot in this matrix.
3. **Standing policy: close-by-default, with parity work licensed only to close
   asymmetries.** When the next resolved-value asymmetry surfaces between the
   arms, it is closed — by authored shared values and/or a canonical port —
   before any new ratio of record is quoted; disclosure is a named exception,
   not the default. Open-ended route parity work still takes the per-key
   t81-style exception path; the t111/t128 prepass exception is unchanged. The
   allowance is recorded as the third exception in the map's **Out of scope**
   parity entry (route-wide lifting declined; a repo ADR declined — the map
   entry is the established record).
4. **Staged fold into t135** (a separate re-take ticket declined; a fused
   single candidate rejected as the t115 failure mode):
   [135](135-cross-layer-occupancy-batching.md) becomes the re-take ticket.
   **Stage 1** (runs unblocked) = port + keys + asserts + a fresh 16-cell
   output campaign that records the **new reference set** (t123's
   candidate-ordinary set retired as the currency; every acceptance citing it
   re-points), with its own old-vs-new paired A/B so the port's cost is
   attributable; **Stage 2** = the cross-layer occupancy batch against the new
   baseline under whatever gate form
   [137](137-planner-scheduling-gate.md) rules; **closing gate** = the
   8-cells × 3-tools scoreboard re-derivation. **Which ratios this applies to:**
   every ratio quoted after Stage 1's re-record and the closing scoreboard;
   t124's ratios of record stand as historical until then, and every scoreboard
   keeps the disclosure until the re-take lands.

**Fact base (re-derived 2026-11-09, session 38 — not quoted from the map):**
PnP's `config.schema.brim_width` default is `8.0`, `skirt_loops` `6`,
`skirt_distance` `3.0`, `skirt_height` `1`
(`modules/core-modules/skirt-brim/skirt-brim.toml`); the matched configs author
no skirt/brim key at all; and the recorded "26 concentric rectangles" first
layer is 20 brim + 6 skirt loops of bounding-box geometry
(`generate_brim_entities` / `generate_skirt_entities`,
`modules/core-modules/skirt-brim/src/lib.rs`). Orca's matched chain resolves
`skirt_loops = 0`, `skirt_distance = 2`, `skirt_height = 1`,
`skirt_per_object = 1` (`resources/profiles/BBL/process/fdm_process_common.json`
at the pinned `v2.5.0`; `0.20mm Standard @BBL X1C.json` overrides none of them)
and the rig pins `brim_width = 0` (`New-OrcaOverrideArgs`,
`evidence/matched-pair/run_scoreboard.ps1`). So Orca prints no skirt and no
brim while PnP prints 26 loops — two asymmetries, one surface. The recorded
slice-time price of PnP's whole skirt-brim surface is ~235 ms (t103), so no
cell's verdict turns on this ruling; it governs whether "on matched settings"
is literally true.

**This take made no production edit** — tracker-only: this ticket, the map, the
[135](135-cross-layer-occupancy-batching.md) re-scope, and the new
`GLOSSARY.md` term. The port, config, rig and campaign edits all ride t135
Stage 1.
