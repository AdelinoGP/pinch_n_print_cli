# t162 — Accelerated host build: `c1xx C1083` was a verbatim `CARGO_TARGET_DIR`

Route: [Accelerated host build: the C1083 is a verbatim-path bug](../../issues/162-accelerated-host-verbatim-path.md).
Fixes a defect open since packet 142 (2026-11-11): every accelerated timing arm
in the map was closed with a disclosure that named the wrong cause.

**Headline: `cargo xtask dist --accelerated --edition developer` builds again —
exit 0, 1 binary + 24 modules — because the failure was never `libmimalloc-sys`,
the `RUSTC` shim, or MSVC 14.51. xtask handed cargo a verbatim `\\?\`
`CARGO_TARGET_DIR`; cargo propagated it into every build script's `OUT_DIR`,
`cc-rs` passed it unchanged into `cl.exe`, and MSVC read
`-c \\?\D:\...\out\mimalloc-static.cc` as the UNC path `\\mimalloc-static.cc`.**

## 1. The failure, as the map recorded it

```text
xtask dist: building pnp_cli (release)...
  c1xx: fatal error C1083: Cannot open source file: '\\mimalloc-static.cc': No such file or directory
  ... "cl.exe" ... "-Fo\\?\D:\...\out\...o" "-c" "\\?\D:\...\out\mimalloc-static.cc"
```

Attributed in every ticket since t142 to "the controlled-driver `RUSTC` wrapper +
MSVC 14.51 mangling the `libmimalloc-sys` verbatim source path". That attribution
was never tested.

## 2. Separating the hypotheses

Four builds of the same crate, each in a fresh target dir so nothing is cached:

| # | build | result |
| --- | --- | --- |
| 1 | fresh dir, **no wrapper**: `cargo build -p libmimalloc-sys --release --target-dir target/clean-probe` | **green**, 5.2 s |
| 2 | fresh dir + the driver's shim as `RUSTC` + `PNP_ACCELERATED=1` + `PNP_ACCELERATED_MODE=host-release` + `PNP_XTASK_EXE` / `PNP_XTASK_WORKSPACE_ROOT` | **green**, 6.2 s |
| 3 | `CARGO_TARGET_DIR` ordinary absolute (`target/dist-host-accelerated`) | **green**, 2.3 s |
| 4 | `CARGO_TARGET_DIR` verbatim (`\\?\D:\...\target\clean-probe-v`) | **red** in ~5 s, the dist's exact error |

Experiment 2 kills the wrapper hypothesis; experiment 4 reproduces the failure in
a loop short enough to iterate on. The reproduction script is
`target/probe-verbatim.ps1` in the take's raw tree (gitignored):

```powershell
$env:CARGO_TARGET_DIR = '\\?\D:\slicerProject\pinch_n_print_cli_2\target\clean-probe-v'
cargo build -p libmimalloc-sys --release
```

```text
c1xx: fatal error C1083: Cannot open source file: '\\mimalloc-static.cc': No such file or directory
```

No shim, no `PNP_*`, no accelerated mode, same crate, same directory name.

### Why the path was verbatim

`build_guests::workspace_root()` returns `cwd.canonicalize()`, and Windows
`canonicalize` answers with the extended-length spelling `\\?\D:\...`. `dist.rs`
sets

```rust
cmd.env("CARGO_TARGET_DIR", host_target_dir)   // ws_root.join("target").join(...)
```

so the root's spelling rides into cargo, into `OUT_DIR`, and into every external
compiler argument a build script builds from `OUT_DIR`. `cc-rs` 1.2.59 has no
verbatim-path handling at all (`grep -r verbatim` over its `src/` is empty), so
`cl.exe` receives the extended path and mis-parses `-c` as a UNC name.

The ordinary build never hits this because it does not set `CARGO_TARGET_DIR`:
cargo resolves `target/` itself and produces an ordinary path. Only the
`xtask`-driven paths (dist, guest builds, accelerated test runs) set it.

## 3. The fix

**One line, at the root's source:**

`build_guests::workspace_root()` now returns
`rustc_driver::strip_verbatim_prefix(&ws.canonicalize()...)`, so every consumer
of the root — dist, guest builds, accelerated test runs — gets the ordinary
drive-letter form. The helper was already in the tree for the *shim* path (with a
comment naming exactly this hazard), which is why the wrapper got blamed: the
hazard was known and had been handled at one call site and not at the other.

`strip_verbatim_prefix` also gained correct `\\?\UNC\` handling:
`\\?\UNC\server\share` flattens to `\\server\share`, not to the relative
`UNC\server\share`.

**A second bug, exposed by the first.** `relative_input_path` compared the raw
spellings of `ws_root` and the file path, and closure manifests come from
`canonicalize` (verbatim). With the root flattened but the closure paths not,
`strip_prefix` failed and an absolute path entered every guest fingerprint — all
60 guests read `STALE` immediately after the first fix. It now flattens both
sides before comparing, which makes the fingerprint independent of path spelling
(and, as packet 71 wanted, portable across checkouts). With that in place the
ordinary tree check returned to **0 stale without a rebuild**, i.e. the recorded
fingerprints were path-spelling-independent all along; only the *comparison* was
not.

## 4. Verification

| check | result |
| --- | --- |
| `cargo xtask dist --accelerated --edition developer` | **exit 0**, 1 binary + 24 modules |
| `pnp_cli module diagnose --module-dir target/dist-accelerated/developer/modules` | `pass: true`, 24 modules, provenance `external` |
| 8 cells × 2 arms output identity (1 run each, no warmup) | **16/16** equal to [t151's active set](../t151-skirt-hull-contour/reference.json), all `exit 0`, `degraded: false` |
| `cargo xtask build-guests --check` / `--accelerated --check` | **0** / **0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| regression tests | 3, each red before the fix |

The parity rows are recorded in
[`reduced/parity8.json`](reduced/parity8.json); `verify.py` re-derives all 139
checks from it plus the reference artifact and rejects 12/12 mutations, so the
16/16 claim is checkable from a plain checkout.

The eight cells: benchy classic-off `3d28d1b3…`, benchy classic-on `eb202973…`,
benchy arachne-off `39ad6289…`, benchy arachne-on `77c3c578…`, base classic-off
`9bb18f0e…`, base classic-on `eccbd523…`, base arachne-off `2f249cdb…`, base
arachne-on `1811425f…`.

## 5. What this corrects, and what it does not claim

**Corrected:** the t142–t161 disclosures. The wrapper was never involved, MSVC
14.51 was never the cause, `libmimalloc-sys` was never broken, and "the
accelerated host remains unbuildable" was a stale sentence copied forward for
about ten tickets. The manual-cfg prohibition in `docs/23` is real, but it was
never the obstacle.

**Not claimed:** no accelerated timing number. This take restores the build path
and proves output identity; it does not measure the accelerated arm, so every
timing statement in the map remains ordinary-only. Taking the accelerated arm on
the eight cells is now possible and is the obvious next shape.

**Disclosed:** the two `dist` runs rebuilt guest artifacts (60 accelerated guests
once, then 0), so `target/` and `target/guests-accelerated/` carry fresh
artifacts from this take; both freshness gates exit 0 afterwards.
