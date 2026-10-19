# t38 campaign snapshots (ordinary + accelerated)

**Status: frozen.** Both mode-distinct developer snapshots were built once
with the controlled entry points and copied into the durable namespace.
No slice, test or timing command ran.

- durable root: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z`
- manifest: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\snapshot-manifest.json`
- manifest sha256: `5c58b30145413df82ea9dc93a6b81bb778c69256550dc14d89ead7a61b654ec5`
- ordinary exe: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\ordinary\pnp_cli.exe`
- ordinary modules: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\ordinary\modules`
- accelerated exe: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\accelerated\pnp_cli.exe`
- accelerated modules: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\accelerated\modules`
- t41 corpus root for `-CorpusRoot`: `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t41-20260930T232255Z\corpus`

All recorded gates exited 0: t41 read-only identity gate (before and
after), exact rustc identity, source-input equality (before and after),
both dist builds, both mode-specific freshness checks (ordinary also
rechecked after the accelerated build), copy inventories, 24 per-guest
accelerated metadata identities, and four read-only module-diagnosis
probes (runner-default and isolated) showing 24 external modules with
exactly one search root per snapshot.

Accelerated WASMs differ from the ordinary WASMs for 23 of 24 modules; this is recorded,
not asserted as a correctness signal. Frozen ordinary/accelerated
subdirectories contain no mutable logs; logs live under the durable
`logs/` directory and in this evidence directory.
