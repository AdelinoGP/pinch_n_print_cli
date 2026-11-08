# Reproducing the t97 evidence

The finding is a **negative**: the matched job's effective branch distance is
already 1 mm on both sides, so no profile or production change was made. The
evidence is a source/config re-derivation, not a timing campaign.

## Checkout-only (no model fixtures, no Orca, no build)

```bash
cd docs/specs/perf-vs-orca/evidence/t97-branch-distance
python verify.py
python verify.py --self-test
```

`verify.py` reads only committed files:

- `../matched-pair/configs/pnp-{classic,arachne}-supports-on.json` — neither
  branch-distance key is set, and the family is `tree(auto)`.
- `../t82-fairness-key-audit/appendix/*-supports-on.txt` — Orca resolves
  `support_style = default`, `tree_support_branch_distance = 5` and
  `tree_support_branch_distance_organic = 1` on all four cells.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` —
  PnP defaults (normal distance `1.0`, style `"default"`) and the documented
  `smsDefault` → `smsTreeOrganic` mapping.
- `modules/core-modules/tree-support-planner/src/lib.rs` —
  `TreeSupportStyle::from_config` aliases `default`/`organic`/`grid`/`snug`
  to `Strong`.
- `docs/DEVIATION_LOG.md` — `DEV-156` records the organic-engine substitution
  and the queued port.
- `../t88-supports-increment/FINDINGS.md` — the 5 mm arm's measured numbers.

`--self-test` mutates five of those inputs (one per check) and requires every
mutation to be rejected.

## External canonical check (needs an OrcaSlicer source checkout)

The canonical dispatch is **not in this repo**. With a checkout, read these by
**symbol**, not by line:

- `PrintObject::_generate_support_material` (`src/libslic3r/PrintObject.cpp`):
  a tree family with `support_style == smsDefault` or `smsTreeOrganic` forks to
  `generate_tree_support_3D`; everything else builds `TreeSupport`.
- `TreeSupport::generate` (`src/libslic3r/Support/TreeSupport.cpp`): the same
  fork for `smsTreeOrganic`.
- `SupportParameters`' substitution chain
  (`src/libslic3r/Support/SupportParameters.hpp`): `grid`/`snug` degrade to
  `smsDefault`, and `smsDefault` on a tree family becomes `smsTreeOrganic`
  (`// Orca: use organic as default`).
- `TreeSupportCommon.hpp`: `support_tree_branch_distance` is read from
  `tree_support_branch_distance_organic`.

Checked at upstream `main` `33909a51cd` (2026-07-17), at the local fork's
`pnp/main` `97d1745e5a` (2026-09-03), and re-fetched from upstream `main` on
2026-10-30, all on `D:\slicerProject\Orca(pnp_gui)`. All three agree.

A live re-check without a checkout:

```
https://raw.githubusercontent.com/OrcaSlicer/OrcaSlicer/main/src/libslic3r/Support/SupportParameters.hpp
https://raw.githubusercontent.com/OrcaSlicer/OrcaSlicer/main/src/libslic3r/Support/TreeSupportCommon.hpp
```

Look for `// Orca: use organic as default` in the first and
`config.tree_support_branch_distance_organic.value` in the second.

## Why there is no Orca run in this evidence set

The installed Orca was updated on 2026-10-27 and no longer accepts the map
rig's file-path `--load-settings` form:

```
Slic3r::CLI::run found error, exit
can not resolve preset <path>: Preset was not found in the loaded bundle
```

The install also no longer carries `resources/profiles/BBL/{machine,process}`
(it ships `BBL.opc`), so the rig cannot borrow those paths either. A fresh
Orca-side measurement — Phase 0 or the scoreboard — needs the rig's preset
loading updated first. That is an infrastructure item, recorded on the map,
not a result this ticket can manufacture.

The key-liveness test was staged but not run:
`.local-artifacts/perf-vs-orca/t97-branch-distance/run-keytest.ps1` (gitignored
raw tree) builds the default / organic-5 mm / normal-0.5 mm / slim arms and
fails at the preset-resolution step above. Re-run it after the rig fix if a
behavioural confirmation of the source dispatch is wanted.
