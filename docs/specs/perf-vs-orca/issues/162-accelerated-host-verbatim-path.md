# Accelerated host build: the `libmimalloc-sys` C1083 is a verbatim-path bug, not the wrapper

Type: task / defect fix
Label: wayfinder:task
Status: active
Assignee: 2026-11-18 OpenCode wayfinder session — the human said "push harder on
  the Accelerated path, it might have broken a few tickets ago and we pushed
  forward without it".
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t158 eight-cell scoreboard](161-post-t158-scoreboard-refresh.md)'s
  disclosure — the accelerated timing arm had been reported unavailable since
  [t142](../issues/142-overhang-band-partition-deletion.md) and every ticket
  since simply carried the disclosure forward.

## Question

`cargo xtask dist --accelerated --edition developer` has failed since packet 142
with

```text
c1xx: fatal error C1083: Cannot open source file: '\\mimalloc-static.cc': No such file or directory
```

and the map has attributed it, without ever verifying, to "the controlled-driver
`RUSTC` wrapper + MSVC 14.51 mangling the `libmimalloc-sys` verbatim source
path", closing every accelerated arm with "a manual-cfg host is barred by
`docs/23`". Is the wrapper actually responsible, and what is the real cause?

## Answer

**The wrapper is irrelevant. The cause is that xtask hands `cargo` a verbatim
(`\\?\`) `CARGO_TARGET_DIR`.**

`build_guests::workspace_root()` returns `cwd.canonicalize()`, and on Windows
`canonicalize` answers with the verbatim spelling. `dist.rs` (and the guest
build, and the accelerated test path) set `CARGO_TARGET_DIR` to a path joined
onto that root, so every build script inherits a verbatim `OUT_DIR`, and
`cc-rs` 1.2.59 — which does no path rewriting at all — passes it straight into
`cl.exe`'s argument vector:

```text
cl.exe ... "-Fo\\?\D:\...\out\...o" "-c" "\\?\D:\...\out\mimalloc-static.cc"
```

MSVC reads that `-c` argument as the UNC path `\\mimalloc-static.cc` and fails.

### The four experiments that separate the hypotheses

| # | build | result |
| --- | --- | --- |
| 1 | `cargo build -p libmimalloc-sys --release --target-dir target/clean-probe` (fresh dir, **no wrapper**) | **green**, 5.2 s |
| 2 | same crate, same fresh dir, with the driver's shim as `RUSTC` plus `PNP_ACCELERATED` / `PNP_ACCELERATED_MODE` / `PNP_XTASK_*` | **green**, 6.2 s |
| 3 | same crate, `CARGO_TARGET_DIR` = an ordinary absolute path | **green**, 2.3 s |
| 4 | same crate, `CARGO_TARGET_DIR` = `\\?\D:\...\target\clean-probe-v` (verbatim) | **red** in ~5 s, byte-for-byte the dist's error |

So the shim, the policy env and the crate are all innocent; the path spelling is
the whole bug. Experiment 4 is the reproduction and the tight loop.

### The fix

1. `build_guests::workspace_root()` flattens the root with the existing
   `rustc_driver::strip_verbatim_prefix` before returning it, so every caller
   that turns the root into `CARGO_TARGET_DIR` gets the ordinary drive-letter
   form. `strip_verbatim_prefix` gained proper `\\?\UNC\` handling
   (`\\?\UNC\server\share` → `\\server\share`, not the relative
   `UNC\server\share`) on the way.
2. **A second, quieter bug surfaced by the fix:** `relative_input_path`
   compared the raw spellings of `ws_root` and the file path, and closure
   manifests arrive from `canonicalize` (verbatim). With the root flattened but
   the closure paths still verbatim, `strip_prefix` failed and the absolute path
   entered the guest fingerprint — every guest in the tree read **stale**. It
   now flattens both sides first, which makes the fingerprint independent of how
   the path was spelled (and keeps it portable across checkouts, which is what
   packet 71 wanted).

### Verification

- `cargo xtask dist --accelerated --edition developer` → **exit 0**, "staged 1
  binary + 24 modules" (first success since packet 142).
- `pnp_cli module diagnose --module-dir target/dist-accelerated/developer/modules`
  → `pass: true`, 24 modules, provenance `external`.
- **Eight-cell output parity, 16/16**: every cell on both arms (`pnp-ordinary`,
  `pnp-accelerated`) is byte-identical to
  [t151's active set](../t151-skirt-hull-contour/reference.json), every run
  `exit 0`, `degraded: false`.
- `cargo xtask build-guests --check` **0** and `cargo xtask build-guests
  --accelerated --check` **0**.
- Three regression tests, each red before the fix: `workspace_root_is_never_verbatim`,
  `verbatim_paths_are_flattened_and_ordinary_paths_are_untouched`,
  `relative_input_path_is_spelling_independent` (the first was watched failing on
  the pre-fix line before being restored).
- Evidence: [`evidence/t162-accelerated-host-verbatim-path/`](../evidence/t162-accelerated-host-verbatim-path/FINDINGS.md)
  (`verify.py` PASS 139/139, `--self-test` 12/12).

### What this corrects

Every accelerated-arm disclosure from t142 to t161 — "the controlled-driver
`RUSTC` wrapper + MSVC 14.51 mangles the verbatim source path", "the accelerated
host remains unbuildable", "accelerated **output** parity used the ordinary host
plus the accelerated component tree" — named the wrong cause and understated the
consequence: it was never libmimalloc, never the wrapper, and never the
toolchain. The accelerated arm is now available for timing campaigns; nothing in
this take measures it.
