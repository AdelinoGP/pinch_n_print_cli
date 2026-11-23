# t101 findings — support-attribution side table materialisation

Ticket: [Support-attribution side table
materialisation](../issues/101-support-identity-table-materialisation.md).
Take: 2026-10-31, HEAD `681c0daa` (t102's mimalloc KEEP is the shipped allocator).
Cell: ordinary base classic supports-on, `tmp/base.stl`, matched config
`evidence/matched-pair/configs/pnp-classic-supports-on.json`,
`RAYON_NUM_THREADS=12`. Floor **0.80 % = 1,318 ms** (t91's kept ordinary arm,
164.803 s).
Probe: host-only, `PNP_T101_PROBE=1`, applied to
`crates/slicer-gcode/src/{emit.rs,probe_t101.rs}`,
`crates/slicer-runtime/src/{lib.rs,layer_executor.rs}` and
`crates/pnp-cli/src/main.rs`, reverted before the take closed; the diff is
[`probe.patch`](probe.patch).

**Final ruling: Phase 1 screen — both exact families are below the floor. Stop
at Phase 1. No candidate, no campaign, no production edit.** The table's whole
combined price (allocation side + free side) is **822–927 ms = 0.50–0.56 % of the
cell = 0.62–0.70x the floor**, and that is an *upper* bound: it charges the
table's full summed assembly time to the critical path, though assembly runs
across layers in parallel.

The headline is a **ledger correction to [t100](../t100-allocation-arena/FINDINGS.md)**:
its 3.468 s free-side ceiling was a *system-heap* artifact. t102 shipped
mimalloc, and the same free now costs **432–494 ms — 7.1–8.2x less**. The
cause-level route this ticket was graduated to chase had already been banked by
the allocator swap.

## Method

Two env switches on one binary, so the arms differ only in whether the table is
built:

- `PNP_T101_PROBE=1` — shape census at emit, plus a `drop(layer_irs)` timing and
  (opt-in, `PNP_T101_CENSUS=1`) an allocation census over that drop.
- `PNP_T101_SKIP_TABLE=1` — the assembly closure pushes **no**
  `SupportEntityIdentity` row, so the table is never materialised. Production
  behaviour is untouched when unset.

The **allocation side** is measured inside the product, not derived: the probe
wraps `assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) in a `Instant` window and prints
one line per layer. The skip arm runs the same function and the same geometry
work, so the delta between the arms is the table's clone cost **including
allocation, memcpy, and `Vec` growth**.

Five runs, one probe binary (`exe_sha 4095a544…`): three table arms (two
sampling-free drop timings, one censused) and two skip arms (one each). Every
run emitted **`22200671167a97a7…` / 42,507,912 bytes — byte-identical to the
post-t81 reference** (`evidence/t91-exact-z-memo/references.json`), which is
the falsifiable form of "no production reader".

## The shape

One deterministic census, identical on all three table arms:

| quantity | value |
| --- | ---: |
| layers / entities / `support_entity_identities` rows | 495 / 30,076 / **23,751** |
| Σ `demand_ids` across rows | **34,662,046** |
| demand-id payload bytes | 415,444,046 |
| per-row list length (min / p50 / p90 / max) | 6 / **1,322** / 2,821 / 2,847 |
| rows with an empty list | 0 |
| distinct `(family_id, body_id)` pairs | **431** |
| rows per body (p50 / max) | 50 / 129 → **55.1 mean** |
| bodies whose rows carry differing lists | **0** |
| distinct demand-list *contents* | **418** |
| ids retained if one list is shared per body | 428,726 = **1.24 %** |
| blocks present / retained | 34,733,299 / 429,980 = **1.24 %** |
| heap present / retained | 1,248,605,209 B / 15,471,852 B = **1.24 %** |

Read plainly: **the table is one body's demand list cloned once per emitted
support path.** 431 bodies, 55 paths each. The list is not per-path data — it is
per-body data re-materialised 55 times. `mixed_bodies = 0` says every row of a
body carries a byte-identical list, so **sharing one identity per body is
content-exact by construction**, and it would keep 1.24 % of the blocks.

Two shape facts worth carrying forward:

- The list is not really per *body* either: 431 bodies carry only **418 distinct
  contents**, so 13 bodies share a list with another body. Whatever the planner
  is putting in `demand_ids`, it is closer to a per-family set than to a
  per-path attribution.
- 1.248 GB of live heap is the table. That is 92 % of the collection's bytes and
  ~24 % of the process's ~5.3 GB peak working set (t79). **Memory is out of
  scope for this map** (DEV-026 / the map's own *Out of scope*), so it is
  recorded and not priced.

## The two sides

| arm | Σ assembly over 495 layers | whole-print drop | blocks freed |
| --- | ---: | ---: | ---: |
| table (2 sampling-free + 1 censused) | 413.3 / 433.8 / 452.7 ms | 432.4 / 494.1 / 434.4 ms | 34,796,481 |
| skip (1 sampling-free + 1 censused) | 21.1 / 17.3 ms | 2.614 / 2.890 ms | 62,751 |

- **Allocation side: 392–435 ms** (median 433.8 − 21.1 = 412.6 ms). This is
  `Vec::clone` of ~1,459 `String`s per row, 23,751 times: ~34.7 M allocations,
  1.25 GB of header + payload copies.
- **Free side: 429.5–491.5 ms** (median 494.1 − 2.9 = 491.2 ms; uncontended
  432.4 − 2.9 = 429.5 ms).
- **Combined ceiling 821.6 ms uncontended, 926.8 ms most generous**
  (`min`/`max` over the arms, charging 100 % of the parallel assembly time to the
  critical path). **0.50–0.56 % of the cell, 0.62–0.70x the 1,318 ms floor.**

The census counters cost nothing measurable here: the censused table arm's drop
(434.4 ms) sits *between* the two sampling-free drops (432.4 / 494.1 ms), so the
instrumentation overhead is below the run-to-run spread. That is the opposite of
t100's system-heap measurement, where sampling cost a measured 506 ms — another
consequence of mimalloc making each `free` ~7x cheaper.

## Why the wall is not a claim

The paired wall does not resolve this lever. Table-arm medians 154.615 s vs
skip-arm 152.431 s, but the five runs spread **150.195–154.793 s (4.60 s, 3.0 %)**,
and the session drifted upward (a7 151.3 s → a9 154.6 s on the same binary). The
run-to-run spread is **5.0x the entire instrumented ceiling**, so the wall
cannot corroborate or refute a 0.8 s effect on this machine — and no wall claim
is made. The ceiling rests on the within-run instrumented measurement, which is
unaffected by ambient load.

## The families

| family | exact? | ceiling (derived from the measured total) | contract consequence |
| --- | --- | ---: | --- |
| **1. Share one identity per body** | **yes** — `mixed_bodies = 0`, and the census pins the retained content to the same 418 lists | **~812–916 ms** (98.76 % of the table) | Changes `SupportEntityIdentity`'s public shape (`demand_ids: Vec<String>` → a shared handle, or a per-layer body table plus an index), and re-points the two integration tests. |
| **2. Prune / gate the table** | no — removes an attribution surface | **822–927 ms** (100 %) | Deletes or gates a `pub` IR field that has **no production reader**; retires or moves `structured_support_identity` and `raft_geometry`. Needs the human's ruling, as the ticket states. |
| **3. Keep** | — | 0 | No change. |

Neither clears the floor. Family 1 is the cheapest exact family and lands at
**0.62–0.69x**, which fires the ticket's own stop rule. Family 2's extra 82 ms
over family 1 does not change that verdict, and its only other benefit is the
1.248 GB of heap — which this map rules out of scope.

**Recommendation: family 3 (keep).** The lever is real but small: ~0.5 % of a
cell, already 7x cheaper than when it was graduated, and the map needs whole
cells, not half-percents. A future re-entry is legitimate only if (a) the
allocator changes again, or (b) a production consumer of the table appears and
makes it work rather than dead weight.

## Consumers, re-derived

- **Written only** by `assemble_ordered_entities_with_support_identities`
  (`crates/slicer-runtime/src/layer_executor.rs`), one row per support path.
- **Read by no production path.** The only reads in the tree are
  `crates/slicer-runtime/tests/integration/structured_support_identity.rs`
  (assembly retains family/body/demand and `apply_entity_order_proposal`
  preserves the `entity_id` link) and
  `crates/slicer-runtime/tests/integration/raft_geometry.rs` (which stores the
  returned rows in a fixture and never asserts on them).
- **Set empty** on every other producer: `crates/slicer-macros/src/lib.rs` (guest
  layer-collection materialisation) and `crates/slicer-runtime/src/anchored_rows.rs`.
- **Absent from `crates/slicer-schema/wit/`**, so no wire contract depends on it.
- **Proven unread end-to-end**: five runs with the table absent, byte-identical
  G-code, `degraded=false` in every run.

## Disclosures

- The machine was **not quiet**: the owner's browser was using 15–56 % of 12
  logical cores across the session, and the run-to-run wall spread is 3.0 %.
  This is why the ceiling is instrumented and the wall is not claimed.
- The two drop-timing modes were taken on one binary, but the table arm's drops
  still spread 432–494 ms (14 %), which is the free side's own noise. The ceiling
  uses the `min`/`max` envelope, not the median of a quiet campaign — a
  deliberate over-charge in the lever's favour.
- The **allocation side is summed across layers**, not critical-path. Layers run
  in parallel (12 threads), so the true wall contribution is smaller than
  412.6 ms. Charging all of it is conservative for the lever.
- `PNP_T101_SKIP_TABLE` also skips the table in the arm that measures the drop;
  both arms emit the same G-code (C02), so nothing downstream diverges.
- Only the ordinary classic supports-on cell was screened. t94 measured arachne-on
  with the same table shape; it is not re-measured here.
- **The `SupportEntityIdentity` heap figure (1.2486 GB) is derived** from the
  shape (24 B header per id + payloads + id strings) and agrees with the census's
  freed bytes minus the skip arm's (1,358,397,544 − 107,020,175 = 1,251,377,369)
  to 0.2 %.
- Probe reverted; both guest freshness gates exit 0 at close. No production
  edit, no candidate, no campaign, no auto-commit.
- `verify.py --self-test` exits 0 against `summary.json` alone: **12/12 claims,
  15/15 negative controls rejected**.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md).
