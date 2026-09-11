# 149 — Measure ticket 01's `ResolvedConfig` blind spot in the gap inventory

Type: research
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 96 (2026-09-10). **Ticket 01's asset cannot see a key implemented as a
host-side `ResolvedConfig` field, so an unknown number of queue keys are already live and
counted as gaps.**

Ticket 01's own Reproduction section defines its `live` probe as: module `[config.schema]`
manifests, `docs/config/host-keys.toml`, and a `get_*` / `ConfigKey::from` string-literal
scrape. **`ResolvedConfig`'s `cli "..."` / `cli_opt "..."` macro declarations
(`crates/slicer-ir/src/resolved_config.rs`) are not among them.** A key that is CLI-bound,
resolved host-side, read by a host algorithm, and never declared in any manifest therefore
reads `live=no` — indistinguishable in the asset from a key that does not exist. The asset's
four caveats cover renames, declaration-vs-consumption, `get_*` false positives and CI
drift; none covers this.

**Measured at filing:** of the 73 `cli` / `cli_opt` key declarations in
`crates/slicer-ir/src/resolved_config.rs`, **30 are marked `live=no`** in
[`01-asset-gap-inventory.md`](./01-asset-gap-inventory.md). Reproduce:

```bash
grep -oE '^\s*cli(_opt)?\s+(@[a-z]+\s+)?"[a-z_0-9]+"' crates/slicer-ir/src/resolved_config.rs \
  | grep -oE '"[a-z_0-9]+"' | tr -d '"' | sort -u > /tmp/rc_cli_keys.txt
awk -F'|' '/^\| `[a-z_0-9]+` \|/ {k=$2; gsub(/[ `]/,"",k); live=$4; gsub(/ /,"",live);
           if(live=="no") print k}' \
  docs/specs/orca-feature-gap/issues/01-asset-gap-inventory.md | sort -u > /tmp/asset_live_no.txt
comm -12 /tmp/rc_cli_keys.txt /tmp/asset_live_no.txt
```

30 is an **upper bound**, and the whole point of this ticket is that nobody has split it.
It mixes two populations:

- **Original false negatives** — live in the tree on 2026-08-07 when the asset was
  generated, and still marked absent. Three are proven: `mmu_segmented_region_max_width`,
  `mmu_segmented_region_interlocking_depth` and the beam bool all landed in `b18c00b3`
  (2026-06-13), two months before the asset. **Ticket 98 (2026-09-10) confirmed the first
  two against the tree and closed them**: both drive the host built-in
  `run_phase5_width_limit` with e2e coverage at non-default values, P91 is dissolved, and
  the queue target dropped 409 → 407. So the blind spot has already cost the queue two
  keys of phantom work on top of ticket 96's beam bool — evidence for question 2 below,
  not a substitute for answering it.
- **Post-generation rot** — landed since, e.g. by the rename workstream (`printable_area`
  arrived with ticket 100, `wall_loops` with ticket 102's neighbourhood). Those are the
  asset ageing normally, not a methodology defect.

Separating them is `git log -S'<key>' --before=2026-08-07` per key, 30 times.

## What this ticket must answer

1. **How many of the 30 were live on 2026-08-07?** Per-key, with the commit that introduced
   each. That is the true size of the blind spot.
2. **Is the queue count wrong, and by how much?** The map's scoped target is 407 (409 before ticket 98; re-derive it from the map rather than trusting this line). Every
   original false negative that is also a queue key is a key already implemented and still
   being counted as a gap. `mmu_segmented_region_max_width` and
   `mmu_segmented_region_interlocking_depth` are two such — and they are **ticket 98's
   entire key list**.
3. **Does the probe need a fourth source, or a fifth caveat?** Either extend ticket 01's
   `live` scrape to include `ResolvedConfig`'s `cli` declarations and regenerate the asset,
   or add an explicit caveat naming this class. Extending is better if it is cheap: the
   asset is the thing the map's Notes tell every packet ticket to size work off.
4. **Which open queue tickets are affected?** Give the list, so each one's session checks
   before authoring rather than discovering it at claim time as ticket 96 did.

## Why it matters

The map's Notes already say **"Never size anything off that column — use ticket 01's asset,
or re-derive."** That instruction is now weaker than it reads: the asset has its own
false-negative class, in the same direction as the ✅/❌ column it was built to replace
(over-reporting gaps). Ticket 96 hit it by accident — it went looking for
`interlocking_beam` and found the behaviour already shipped under a PnP name. Ticket 98 is
about to hit it head-on.

## Answer
