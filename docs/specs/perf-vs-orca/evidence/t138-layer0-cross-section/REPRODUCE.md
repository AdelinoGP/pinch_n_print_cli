# t138 reproduce: layer-0 cross-section attribution

This take is a Phase-1 attribution. It reads the **existing** t135 Stage-1
campaign output; it builds nothing and edits no production code.

## Prerequisites

- The t135 Stage-1 raw tree: `target/matched-pair/t135-scoreboard3/gcode/`
  (8 cells × `pnp-ordinary` + `orca`, `-m3` runs) and
  `target/t135/oracle-final/*.json`.
- The heavy fixtures `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied,
  never committed).
- Python with `numpy`, `trimesh`, `shapely` (the repo's analysis environment
  already has them; `python -c "import trimesh, shapely"` must succeed).

## Reduce

From the repository root:

```bash
python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/reduce_t138.py
```

This writes `reduced/summary.json`. It slices the two STLs at the sampled
layer tops and midpoints, offsets inward by 0.20 mm (PnP) / 0.21 mm (Orca), and
compares the translation-invariant hull areas with each arm's emitted
first-layer outer-wall hull. It also re-parses the layer-0 brim/skirt/support
sections from the same g-code and copies the t135 oracle verdicts.

## Verify

```bash
python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/verify.py
python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/verify.py --self-test
```

`verify.py` needs only `reduced/summary.json` (no raw tree, no STL). The
self-test mutates the summary twelve ways and asserts every mutation is
rejected.

## Restore

Nothing to restore: no guest rebuild, no host rebuild, no production edit, no
G-code reference move. Both freshness gates were checked on 2026-11-10 and exit
0 (`build-guests --check`, `build-guests --accelerated --check`).
