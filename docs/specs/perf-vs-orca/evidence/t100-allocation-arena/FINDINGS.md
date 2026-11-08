# t100 findings — allocation volume of the whole-print layer collection

Ticket: [Allocation volume of the whole-print layer
collection](../issues/100-allocation-volume-layer-collection.md).
Take: 2026-10-30, HEAD `cc2c54e6`.
Probe: host-only, `PNP_T100_PROBE=1`, applied to
`crates/slicer-gcode/src/{emit.rs,probe_t100.rs}`,
`crates/slicer-runtime/src/lib.rs` and `crates/pnp-cli/src/main.rs`, reverted
before the take closed; the diff is [`probe.patch`](probe.patch).

**Final ruling: Phase 1 screen — the arena / bulk-free layer collection, as the
fog scoped it, is measured-dead: mis-aimed and construction-unreachable. No
candidate, no campaign, no production edit.** The census it took graduates a
different, reachable shape: the per-path support-attribution side table
([ticket 101](../issues/101-support-identity-table-materialisation.md)).

## Method

Screening cell **ordinary base classic supports-on** (t91's kept ordinary arm,
**164.803 s**), matched config
`evidence/matched-pair/configs/pnp-classic-supports-on.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`, floor **0.80 % = 1,318 ms**.

The probe installs a counting global allocator (relaxed atomics only — the
`--report` accounting allocator's per-block mutex path would dominate what it
measures) armed **only** around the explicit `drop(layer_irs)` in
`DefaultGCodeEmitter::emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`). It
records blocks, bytes, a log2 size histogram, twelve content samples of the
8–15-byte class, and the drop's microseconds; it also runs a bulk-free
counterfactual (allocate *N* blocks summing to the censused byte volume, touch
every page, time the free) and a sequential-copy pass, and it can be built
against a different global allocator.

Four runs (all clean, all byte-identical to the post-t81 reference
`22200671…` / 42,507,912 bytes):

| run | allocator | arms |
| --- | --- | --- |
| `t100-classic-census` | System | census only (sampling-free) |
| `t100-classic-bulk` | System | bulk `both` orders (pre-copy-fix build) |
| `t100-classic-bulk2` | System | shape + census + bulk `after` + copy |
| `t100-classic-mimalloc` | mimalloc | shape + census + bulk `after` + copy |

## The census

| quantity | value |
| --- | ---: |
| layers / entities / points | 495 / 30,076 / 2,782,098 |
| blocks freed | **34,796,481** |
| bytes freed | **1,358,397,544** (1.358 GB) |
| drop, sampling-free | **3,526,544 µs** (t94 measured 3,558 ms on this cell) |
| ns per block | **101.3** |
| blocks in the 8–15-byte class | **34,662,668 = 99.6 %** |
| that class, sampled content | `__skirt_…` (family/body ids), `demand-N` |
| `support_entity_identities` rows | 23,751 |
| demand-id strings (derived: bucket 4 − 2 per row) | 34,615,166 ≈ **1,457 per row** |
| geometry blocks (entities + layers + annotations + retracts) | 32,673 = **0.094 %** |
| geometry bytes (points + entities + layers) | 105.8 MB = **7.8 %** |

Read plainly: **the free is a per-block cost of 101 ns, and the blocks are not
geometry.** 34.66M of 34.80M blocks are `SupportEntityIdentity` rows that
`assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) clones **per support path**:
each row carries `family_id`, `body_id` and the body's whole
`demand_ids: Vec<String>`, so a body whose demand list holds ~1,457 ids
re-allocates those strings once for every path it emits. Geometry accounts for
**7.8 %** of the bytes, so the rows' `Vec<String>` buffers and their payloads
are the dominant non-geometry term (~92 % of the collection).

## Family verdicts

| family | verdict | measured |
| --- | --- | --- |
| **bulk-free arena, per print** (1 block) | ceiling above the floor, **construction unreachable** | freeing the same 1.358 GB as one block costs **44.5–63.8 ms** (after the drop) or 189.7 ms (before it, collection still live). Ceiling = 3.527 s − 0.058 s = **3.468 s = 2.10 % of the cell = 2.63x the 1,318 ms floor**. But *flattening a live collection* costs a **73.3–115.1 ms** copy **plus the same 3.527 s storm** (3.60 s total, **+73 ms worse**), and an at-creation arena aimed at the geometry would remove **0.094 %** of the blocks. |
| **bulk-free arena, per layer** (495 blocks) | same | 58.3 ms after / 120.5 ms before — the arena's granularity is not what makes the storm. |
| **allocator substitution** (mimalloc) | **above the floor, but a packaging decision** | drop **3.527 s → 1.302 s measured** (saving **2.225 s = 1.35 % of the cell = 1.69x the floor**). Corrected for the probe's own sampling overhead (**0.506 s**), the sampling-free drop is **0.796 s** → saving **2.731 s = 1.66 % = 2.07x the floor**. The in-repo contract (`docs/16_slicer_report.md` §"Global allocator contract") makes the shipped allocator a **packager** choice, so this arm prices the family and does not propose the change. |

The arena family therefore dies twice: its construction is only reachable at
creation (a flatten is strictly worse), and as scoped it is aimed at 0.094 % of
the blocks. The allocator family clears the floor on paper but changes a
contract the repo deliberately delegates to packagers, and it does not remove
the work — 34.8M blocks are still freed.

## What the screen leaves

The reachable shape is the support-identity table itself:

- **No production reader.** `support_entity_identities` is written at assembly
  and read only by `structured_support_identity.rs` and `raft_geometry.rs` under
  `crates/slicer-runtime/tests/integration/`; the module marshal sets it empty
  (`crates/slicer-macros/src/lib.rs`) and it is absent from
  `crates/slicer-schema/wit/`.
- **Cloned per path.** 23,751 rows, each carrying the body's demand list, so
  the same strings are re-allocated per emitted support entity.
- **Ceiling.** The free side alone is **3.468 s (2.63x the floor)**, plus an
  **unmeasured** allocation-side cost: the 34.66M strings are also allocated
  during assembly, and this screen armed only the drop. A follow-up should
  price both sides, audit the consumers, and rule between sharing one identity
  per body, pruning/gating the table (the t20 pattern), or keeping it.

Graduated: [ticket 101](../issues/101-support-identity-table-materialisation.md).

## Disclosures

- Instrumented walls (200.7–270.7 s) are **not** wall claims:
  `--profile --profile-verbose` plus the probe's `eprintln` inflate them. The
  164.803 s cell wall is t91's kept ordinary arm.
- The sampling instrumentation costs **0.506 s** on the same drop (34.66M extra
  atomic increments plus an 8-byte copy each): 3.527 s sampling-free vs 4.033 s
  sampled. The mimalloc arm was measured with sampling, so its **1.302 s is an
  upper bound**; **0.796 s** is derived by subtracting the measured overhead and
  is labelled as derived everywhere it appears.
- The `before` bulk arms ran while the collection was still live; their free
  prices (189.7 / 120.5 ms) are 2–3x the `after` arms (44.5–63.8 / 58.3 ms)
  because the heap state differs. The verdict uses the **conservative** (max)
  arm; both orders are far below the per-block price.
- The copy arm was **initially elided by the optimiser** (dead stores into
  memory that is freed unread): the first run's `copy_us=0` is discarded, and
  the fixed arm (`black_box`-ed checksum read back out of the destination)
  supplies 73.3–115.1 ms. `summary.json` keeps both runs and the fix is in
  `probe.patch`.
- Only the classic-on cell was censused. t94 measured arachne-on's drop at
  3,242 ms with the same side table, so the shape is not expected to differ, but
  it is not re-measured here.
- The probe is reverted; both guest freshness gates exit 0 at close. No
  production edit, no candidate, no campaign, no auto-commit.
- `verify.py --self-test` exits 0 against `summary.json` alone: 12/12 claims,
  13/13 negative controls rejected.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md).
