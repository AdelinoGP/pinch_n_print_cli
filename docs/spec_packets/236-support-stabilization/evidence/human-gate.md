# Packet 236 Human Validation Gate Evidence

- Packet: `236-support-stabilization`
- Evidence date: 2026-08-23
- This evidence was produced by agent implementation and is pending HUMAN sign-off.

## Artifact-Producing Commands

Profile flags were resolved via `target/debug/pnp_cli slice --help`; both matched profiles were verified present with `test -s`.

```text
target/debug/pnp_cli slice --help
test -s tmp/support-family-config-tree-matched.json
test -s tmp/support-family-config-normal-matched.json
target/debug/pnp_cli slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-tree-matched.json --output tmp/p236-tree.gcode --module-dir modules/core-modules
target/debug/pnp_cli slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-normal-matched.json --output tmp/p236-normal.gcode --module-dir modules/core-modules
```

## Visual-Debug Bundles

The request files are `tmp/vd-p236-tree-request.json` and `tmp/vd-p236-normal-request.json`. The valid model-request shape produced manifests in both output directories:

```text
target/debug/pnp_cli visual-debug --request tmp/vd-p236-tree-request.json --output tmp/vd-p236-tree --overwrite
target/debug/pnp_cli visual-debug --request tmp/vd-p236-normal-request.json --output tmp/vd-p236-normal --overwrite
```

Outputs: `tmp/vd-p236-tree` (0 PNG files) and `tmp/vd-p236-normal` (0 PNG files). The attempted `Detail` visualization targeting `SupportPlan` was rejected by the binary's `VisualizationSpec` parser. The available successful request shape did not render PNGs or expose the support-plan emission-per-region boundary; render-based inspection is therefore unavailable for this gate.

## Inspection Checklist

- **Termination — UNVERIFIED-BY-RENDER.** No PNG render was produced. Automated proxy: the strengthened tripwire golden freezes 154 support endpoints in `resources/golden/benchy_tree_support_regression_endpoints.txt`.
- **Coverage — UNVERIFIED-BY-RENDER.** No PNG render was produced. Automated proxy: measured support-plus-interface block presence is 124 in each PnP family artifact, against the fixture's overhang geometry.
- **Collision freedom — UNVERIFIED-BY-RENDER.** No PNG render was produced. Automated proxy: the strengthened-tripwire collision-ladder execution is the available automated check.
- **Interfaces — MEASURED.** `;TYPE:Support interface` counts: PnP tree 2, PnP normal 2, Orca tree 2, Orca normal 3. `support_interface_top_layers=2` is configured; the normal-family PnP/Orca interface count differs.
- **Block counts — MEASURED DELTAS (PnP minus Orca).** Exact `;TYPE:Support` counts are PnP tree 122, PnP normal 122, Orca tree 122, Orca normal 121: tree delta 0, normal delta +1. Exact `;TYPE:Support interface` counts are PnP tree 2, PnP normal 2, Orca tree 2, Orca normal 3: tree delta 0, normal delta -1. Support-plus-interface totals are 124, 124, 124, 124 respectively.

- Sign-off (human, blocking): pending — <date + verdict to be recorded by the human>.
