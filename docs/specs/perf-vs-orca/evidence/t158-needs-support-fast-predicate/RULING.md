# t158 human KEEP ruling

Date: 2026-11-17

The human reviewed the original mechanical DROP and ruled: **“I accept it as a keep.”**

## Decision

Land `expolygons_intersect_area_positive` in
`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) and its
boundary/hole/fallback tests. The final ticket verdict is **KEEP by explicit human
gate amendment**.

This does not rewrite the campaign's mechanical result. The original standing
rule required a separate-arm median wall saving of at least 0.80 % in every cell;
`benchy-classic-on` recorded +0.766 % and `benchy-arachne-off` recorded −0.076 %,
so `reduced/campaign.json` and `verify.py` continue to report that original gate
as DROP. The human ruling supersedes the stop rule's revert requirement after
considering the complete evidence:

- byte-identical, clean G-code in all eight cells against the active reference;
- lower process CPU in all eight cells, with 39/40 pairs candidate-lower;
- decisive wall wins in all four `base` cells (20/20 pairs);
- positive median within-pair wall savings on the two mechanically failing
  `benchy` cells (+2.690 % and +3.158 %).

## Disclosures

- The same-arm null control covered three `base` cells, not `benchy`; no
  `benchy` null-band claim is made.
- The two sub-floor separate-arm `benchy` medians remain failures of the original
  gate, not newly-passing measurements.
- No fresh Orca arm or accelerated timing arm was run. Route ratios derived from
  this campaign are cross-window compositions, not a new scoreboard of record.
- The destination remains 8/8 cells won against Orca; this KEEP banks a tranche,
  not the destination.

## Landing verification

After restoring `candidate.patch`, all 72 `slicer-sdk` `layer_module_tdd` tests
passed. `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask
build-guests --check`, `cargo xtask check-literals`, and `cargo xtask
check-test-quality --report` exited 0. Fresh release slices of
`benchy-arachne-off` and `base-classic-off` on both baseline and candidate arms
exited 0 with `degraded=false`, zero fatal/non-fatal errors, and the active
reference hashes (`39ad6289…` and `9bb18f0e…`). The raw landing-check rows and
logs are in gitignored `target/t158/output/keep-check/`.
