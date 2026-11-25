# t162 reproduce — accelerated host build and its C1083

This take changes `xtask` only (`workspace_root`, `relative_input_path`,
`strip_verbatim_prefix`) plus three unit tests. No slicer, guest or schema code
changes.

## Prerequisites

- Rust toolchain per `docs/23_controlled_perimeter_builds.md` policy
  (`rustc 1.96.0`, host `x86_64-pc-windows-msvc`, LLVM 22.1.2).
- `wasm-tools` on PATH (the guest freshness gate needs it).
- MSVC 14.51 (the cl.exe that mis-parses extended-length paths).
- A 12-logical-core box for the parity campaign (it only needs to run, not to
  time).

## 1. The failure and its reproduction

The bug: a verbatim `CARGO_TARGET_DIR` reaches `cl.exe`.

```powershell
$env:CARGO_TARGET_DIR = '\\?\D:\slicerProject\pinch_n_print_cli_2\target\clean-probe-v'
cargo build -p libmimalloc-sys --release
```

Expected before the fix (and still, with any verbatim target dir — this is an
upstream/toolchain interaction, not something t162 removes):

```text
c1xx: fatal error C1083: Cannot open source file: '\\mimalloc-static.cc'
```

The accelerated entry point produced the same failure because
`build_guests::workspace_root()` used `canonicalize()`:

```bash
cargo xtask dist --accelerated --edition developer   # exit 1 before t162
```

## 2. The fix's verification

```bash
cargo test -p xtask --bin xtask -- workspace_root_is_never_verbatim \
    verbatim_paths_are_flattened relative_input_path_is_spelling
cargo xtask build-guests --check                      # 0
cargo xtask build-guests --accelerated --check        # 0
cargo xtask dist --accelerated --edition developer    # exit 0, 24 modules
```

Watch the first test go red on the pre-fix line (temporarily restore
`Some(ws) => ws.canonicalize().unwrap_or(ws)` in `workspace_root`), then green
again — that is the regression proof.

## 3. Output parity (eight cells, both PnP arms)

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy,base -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools pnp-ordinary,pnp-accelerated -Runs 1 -Warmups 0 -BatchId t162-accel-parity8"
```

Rows land in `target/matched-pair/t162-accel-parity8/scoreboard-rows.csv`
(gitignored). The reduced copy is `reduced/parity8.json`; `python verify.py`
re-derives 139 checks from it and `--self-test` rejects 12/12 mutations.

`-Runs 1 -Warmups 0` is deliberate: this is an **output-identity** campaign, not
a timing one. No wall number in this take is a claim.

## 4. Restore

No production revert is needed; both freshness gates exit 0 and clippy is clean.
The take left `target/guests-accelerated/` freshly rebuilt (60 guests) and a
staged `target/dist-accelerated/developer/` snapshot, both gitignored.
