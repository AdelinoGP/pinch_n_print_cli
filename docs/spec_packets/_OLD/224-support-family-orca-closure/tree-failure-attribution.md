# Tree-family failure attribution (packet 224)

Forensic worker `w12`. Read-only investigation; no source edits, no commits, no branch/stash
operations.

## Method

`git archive <C> modules/core-modules/tree-support-planner modules/core-modules/tree-support | tar -x`
materialised **both source and test files** of the two tree modules at each reference commit,
then `cargo test -p tree-support-planner --no-fail-fast` and `cargo test -p tree-support
--no-fail-fast` were run. Materialising the tests too (not just `src/lib.rs`) reproduces the
true historical state of each crate and avoids the new-test-against-old-source artefact.

Guest WASM rebuild was **not** required for these runs: every failing test is a native Rust
integration test in a workspace member crate (`use tree_support::TreeSupport;`,
`use tree_support_planner::SupportPlanner;`) and neither module has a `build.rs`, so
`cargo test` compiles the module natively and never touches the `.wasm` artefacts. The tar
extraction did bump source mtimes, which made `cargo xtask build-guests --check` report
`STALE: tree-support-guest` / `STALE: tree-support-planner-guest` afterwards; a full
`cargo xtask build-guests` (43 guests) was run and `--check` is now clean.

**Method limitation, stated per instruction:** shared crates that also changed this session
(`slicer-ir`, `slicer-runtime`, `slicer-wasm-host`) were **not** reverted — only the two tree
modules were. All shared-crate changes in the window are additive, and every historical
configuration compiled and ran, so no result below is a compile artefact; but a failure caused
by a shared-crate change rather than by the module change would be attributed to the module
commit by this method.

## Raw counts

| commit | tree-support-planner + tree-support failures |
|---|---|
| `5a38fdce` (last pre-session commit) | 3 |
| `9f4540bd` (tree renderer rewrite) | 11 |
| `4d1848eb` (HEAD) | 10 |

Logs: `target/test-output-5a38fdce.log`, `target/test-output-9f4540bd.log`,
`target/test-output-base.log`.

## Attribution table

| test | failing at `5a38fdce`? | failing at `9f4540bd`? | failing at `4d1848eb` | verdict |
|---|---|---|---|---|
| `tree-support-planner/tests/orca_parity_tdd.rs::benchy_orca_parity_within_tolerance` | no | yes | yes | INTRODUCED-BY-9f4540bd (caveat below) |
| `tree-support-planner/tests/to_buildplate_tdd.rs::contact_xy_outside_footprint_sets_to_buildplate_true` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support-planner/tests/to_buildplate_tdd.rs::default_config_does_not_reject_to_model_contacts` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support-planner/tests/tree_family_tdd.rs::disabled_and_declined` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support-planner/tests/tree_family_tdd.rs::distributed_contacts` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support-planner/tests/tree_family_tdd.rs::anchored_heights_and_termination` | **yes** | yes | yes | INHERITED |
| `tree-support-planner/tests/tree_family_tdd.rs::radius_aware_collision` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support/tests/enforcer_blocker_tdd.rs::default_ineligible_region_generates_zero_support` | **yes** | yes | yes | INHERITED |
| `tree-support/tests/tree_support_tdd.rs::branching_pattern_present` | no | yes | yes | INTRODUCED-BY-9f4540bd |
| `tree-support/tests/tree_support_tdd.rs::density_affects_coverage` | no | yes | yes | INTRODUCED-BY-9f4540bd |

**Bonus finding (not among the 10):**
`tree-support-planner/tests/tree_family_tdd.rs::invalid_body_rejected` was failing at
`5a38fdce` **and** at `9f4540bd`, and passes at `4d1848eb` — repaired by `2afa4cf9`
(extent-based `in_routing_cell`) together with the fixture rewrite in `4d1848eb`. This is the
one net repair the session made inside these two crates.

Test-file churn in the window, for completeness: planner `tree_family_tdd.rs` changed only the
`planner_config` key `support_branch_angle_deg` -> `tree_support_branch_angle` and the
`invalid_body_rejected` fixture; `diagnostics_tdd.rs` and `orca_parity_tdd.rs` changed;
`to_buildplate_tdd.rs`, `tree_support_tdd.rs`, `enforcer_blocker_tdd.rs` and
`tree-support/tests/tree_family_tdd.rs` are **byte-identical across the whole window**, so
their attributions are unqualified.

### Caveat on `benchy_orca_parity_within_tolerance`

The test body *was* edited in `9f4540bd`: the call site moved from `run_support_geometry` to
`run_support_geometry_with_analysis`. The **assertions and the goldens did not change** —
tolerances are still `+/-10%` branch count and `<= 0.5 mm` Hausdorff, and
`resources/golden/benchy_tree_support_orca_*.txt` have no commits in `5a38fdce..HEAD`. The
attribution therefore stands, but note the comparison is against a *self-captured* baseline
recorded before the rewrite, not against OrcaSlicer output. Observed failure at `9f4540bd` and
at HEAD: `Hausdorff distance 1.2884mm exceeds tolerance 0.5mm`. Per `CLAUDE.md` test
discipline, a red self-captured baseline is acceptable if the canonical behaviour is correct.

## Assertions that broke (INTRODUCED tests)

`to_buildplate_tdd.rs::contact_xy_outside_footprint_sets_to_buildplate_true`:

```rust
let entries = output.entries();
assert!(
    !entries.is_empty(),
    "AC-2: contact at the plate centroid is outside the [-10,-10]..[-5,-5] \
     footprint at layer 8 and must be admitted under \
     support_on_build_plate_only=true. Empty plan means to_buildplate \
     was incorrectly false. entries={}, diagnostics={:?}",
```

`to_buildplate_tdd.rs::default_config_does_not_reject_to_model_contacts`:

```rust
assert!(
    !entries.is_empty(),
    "AC-N1: default config must admit a to_model contact (centroid inside \
     footprint at the contact's layer). Expected non-empty plan, got {} \
     entries. diagnostics={:?}",
```

`tree_family_tdd.rs::distributed_contacts`:

```rust
assert!(
    output.entries().len() >= 2,
    "planner must emit multiple layers"
);
```

`tree_family_tdd.rs::radius_aware_collision`:

```rust
assert!(
    emitted_survivor,
    "non-colliding fixture body should remain emitted"
);
```

`tree_family_tdd.rs::disabled_and_declined`:

```rust
assert!(!disabled.entries().is_empty());
```

`tree_support_tdd.rs::branching_pattern_present`:

```rust
assert!(
    has_different_angles,
    "tree support should have varying branch directions, but all angles are similar: {:?}",
    angles
);
```

`tree_support_tdd.rs::density_affects_coverage`:

```rust
assert!(
    count_high > count_low,
    "higher density should produce more paths: low={}, high={}",
    count_low,
    count_high
);
```

The dominant symptom is uniform: **the planner now emits an empty or near-empty plan** on
fixtures that previously produced entries (`!entries.is_empty()` and `len() >= 2` fail), and
the renderer produces fewer / less varied paths downstream. Six of the seven quoted assertions
are emptiness or count assertions, not shape assertions.

## Overall read

**The tree family is materially worse than at `5a38fdce` on the evidence available here.** The
two module crates went from 3 failures to 11 at `9f4540bd` and stand at 10 at HEAD: the session
repaired one pre-existing failure (`invalid_body_rejected`), fixed neither of the other two
inherited ones, and introduced eight. All eight arrived in the single commit `9f4540bd`, and
seven of them are on test files that did not change at all in the window, so they are
unambiguous behavioural regressions rather than moved goalposts. The failure signature — mostly
`!entries.is_empty()` / `len() >= 2` / `count_high > count_low` — says the rewrite made the
planner *stop emitting* on fixtures it used to cover, which is the opposite of the coverage the
rewrite was meant to improve. That said, this report measures only these two crates' own test
suites; it says nothing about the packet-224 runtime closure tests
(`crates/slicer-runtime/tests/integration/tree_support_family.rs` and
`support_family_closure.rs`), which were rewritten this session and may encode intentionally
changed expectations. A complete better/worse judgement needs those runtime results alongside
this table, and needs someone to decide whether the eight fixtures encode behaviour the
canonical algorithm should still exhibit — if they do, `9f4540bd` needs rework, not a fixture
update.
