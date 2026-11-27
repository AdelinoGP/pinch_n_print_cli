# 114 — `sparse_infill_speed`: align the `ResolvedConfig` default and re-base `speed_factor`

Type: task
Status: resolved
Assignee: wayfinder session (ses_f90234641ffe1mV2osTWhGc5F9) — claimed 2026-09-05, resolved 2026-09-05
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q11(b)**:
*"`ResolvedConfig` default 50.0 → 100.0; `speed_factor` relative to resolved
default, not `BASE_SPEED`"* — because *"modules receive the `ResolvedConfig`
value via `to_config_map`, shadowing the manifests' 100.0; `BASE_SPEED = 50.0`
only coincidentally yields factor 1.0."*

This is the **confirmed instance** of the map's standing hazard (carried finding
1): for a plain-typed key that also has a `ResolvedConfig` field, the manifest
`default =` is dead. Ticket 107 aligned three manifests to canonical `100`, and
every module still receives `50.0`. The two numbers currently cancel out — a
`50.0` value against a `BASE_SPEED` of `50.0` gives factor `1.0`, which is
canonical's 100 mm/s — so **today's output is right by coincidence**, and any
change to either number alone breaks it. That is exactly the state a future agent
"fixes" into a regression.

**Correct the ruling's scope before designing.** Q11(b) says it *"touches 3
infill modules"*. Verified against the tree (2026-09-02) — the three do not share
one shape:

- `modules/core-modules/gyroid-infill/src/lib.rs` — has `const BASE_SPEED: f32 = 50.0;`
- `modules/core-modules/lightning-infill/src/lib.rs` — has `const BASE_SPEED: f32 = 50.0;`
- `modules/core-modules/rectilinear-infill/src/lib.rs` — **has no `BASE_SPEED`**.
  It uses a local `configured_base_speed` and, on at least one path, a literal
  `let speed_factor = 1.0;`.

So it is two modules to re-base plus one to investigate, not three of a kind.
Establish what rectilinear actually does before changing anything, or the
"alignment" will silently change only two of the three.

Also in scope for the same reason (same key, third spelling): the host
`FeedrateConfig.sparse_infill_speed` carries `100.0`. Ticket 107 left it
untouched deliberately. Three declarations of one key with two different values
is the condition to end here.

Decide and execute:

1. Move the `ResolvedConfig` default (`crates/slicer-ir/src/resolved_config.rs`)
   from `50.0` to canonical `100.0`.
2. Re-base each module's speed factor on the resolved default rather than a
   module-private constant, so the factor means the same thing everywhere and
   cannot drift from the config.
3. Resolve rectilinear-infill's different shape into the same contract.
4. **Prove output is unchanged at defaults** — this is the whole risk. A
   before/after G-code comparison on a fixture that actually emits sparse infill,
   not just a unit test on the factor arithmetic.
5. Decide whether `speed_factor`-style module-private base constants are a
   pattern to retire generally; the same shape may exist for other speed keys.

Not a queue key (it is a defaults/plumbing correction, not a gap), so the queue
count is unchanged. Ticket 109 raises the same "two mechanisms for one quantity"
question for `support_ironing_speed`; whoever takes either should read both.

## Answer

**Resolved 2026-09-05 by direct implementation — no packet** (defaults/plumbing
correction, not a gap; the "Packets are for complex implementation only" rule).

**Scope correction executed before designing** (the ticket's own mandate): the
ruling's "factor relative to the resolved default" was implemented as
**retiring the module-side factor base entirely**, because the literal
`value / 100` reading double-counts through the shared raw key. Measured, in
tree: the module receives `sparse_infill_speed` from the same raw config map
that `FeedrateConfig::from_raw_config` reads (`crates/slicer-runtime/src/run.rs`
feeds both from `config_source`; the CLI/3MF `--config` key lands in both), so
`F = host_base × 60 × factor` multiplies the same user value twice —
gyroid/lightning with a configured 120 today emit 288 mm/s (120·60·2.4), and
with the literal re-base would still emit 144 (120·60·1.2). Only factor 1.0
emits the configured value exactly. rectilinear-infill already shipped that
contract ("Feedrate is resolved by the host from each emitted role"), so
"resolve rectilinear's different shape into the same contract" = converge the
other two onto it.

**Changes (7 files):**
1. `crates/slicer-ir/src/resolved_config.rs` — `sparse_infill_speed` default
   `50.0 → 100.0` (canonical), doc comment rewritten (the field is now the
   resolved-config twin of the host key, emitted to the module map + block;
   no longer a factor base). One value across all three spellings
   (manifests 100, `ResolvedConfig` 100, `FeedrateConfig` 100).
2. `modules/core-modules/gyroid-infill/src/lib.rs` — deleted `BASE_SPEED`,
   the field, and the read; sparse paths carry `speed_factor = 1.0` with a
   ticket-114 comment.
3. `modules/core-modules/lightning-infill/src/lib.rs` — same.
4. `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` —
   the `samples_tree_ir_raw_emit` factor assertion re-pinned 1.6 (80/50, the
   old double-count contract) → 1.0.
5. `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — new
   `sparse_paths_carry_neutral_speed_factor` (speed 200 → factor 1.0).
6. `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` — new
   `sparse_infill_speed_resolved_default_is_canonical` (default 100.0 AND
   `to_config_map` carries 100.0 — the module-facing value).
7. Follow-up filed: `134-retire-classic-perimeters-speed-factor-bases` —
   inventory found the same `value / BASE_SPEED` shape with the same
   measured double-count in `classic-perimeters` (outer/inner wall factors
   at `from_config`, gap-fill factor ~line 1145; its
   `speed_factor_from_config` test pins the wrong 30/50 = 0.6 contract).
   Not fixed here — wall emission ordering is DEV-166-delicate and needs the
   classic 24-test suite as its gate.

**Step 5 decision (pattern retirement):** retire `factor = module-key /
module-private-constant` wherever the host `FeedrateConfig` reads the same
raw key; keep ratio-of-two-keys factors (wave-overhangs'
`wave_overhang_print_speed / bridge_speed` self-cancels — that is how the IR
expresses an absolute speed) and per-path modifiers (overhang quartiles,
per-point profiles). One same-shape instance remained: ticket 134.

**Proof, step 4 (defaults unchanged):** before/after end-to-end slice of
`resources/test_stl/ASCII/20mmbox-LF.stl` (a sparse-infill-emitting fixture)
with default config — diff is exactly one line:
`; sparse_infill_speed = 50` → `; sparse_infill_speed = 100` (the CONFIG_BLOCK
spelling, which was wrong-by-coincidence at 50 and is now canonical 100). All
686 sparse moves stay F6000 (100 mm/s); geometry byte-identical.

**Verification:** clippy workspace `-D warnings` clean; `check-literals` clean;
slicer-ir full crate green (incl. new default test); gyroid-infill 20+3+1 green
(incl. new factor test); lightning-infill green (re-pinned assertion);
rectilinear-infill green (untouched); slicer-gcode feedrate emission 17/17
(emitter contract untouched); runtime executor
`lightning_pipeline_linked` green (wasm dispatch with `sparse_infill_speed =
50.0` in config — the module now ignores the key); integration
`gcode_header_thumbnail_config_blocks_tdd` 23/23; e2e 144/144. All 46 guests
rebuilt (module sources + slicer-ir sit in every guest's closure).
`docs/15_config_keys_reference.md` needed no regen (all four rows already
100.0); `docs/config/host-keys.toml` already 100.0.

No deviation rows: the default now *matches* canonical, it does not diverge
from it.
