# Accelerated shim cannot resolve the workspace root from a dependency build script

Type: task
Label: wayfinder:task
Status: resolved (2026-10-25; infra prerequisite of the t85 take — exported-root contract for the accelerated shim; `cargo test -p xtask` 187/0, cold accelerated build of 48 guests passes)
Assignee: 2026-10-25 OpenCode wayfinder session (inside the t85 take)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: discovered inside the [seam inset-offset batching](85-seam-inset-offset-batching.md)
take, which could not build an accelerated artifact without it

## Question

Why does a **cold** accelerated build die inside a registry dependency's build
script, and what is the correct root-resolution contract for
`cargo xtask rustc-shim`?

## Observed failure (2026-10-25, inside t85)

`pwsh …/t85-seam-inset-batching/build-snapshots.ps1 -Arm candidate -Modes accelerated`
on a tree whose accelerated namespace was empty:

```text
thread 'main' panicked at xtask/src/build_guests.rs:54:17:
xtask: cannot locate the workspace root by walking up from
C:\Users\agpen\.cargo\registry\src\index.crates.io-…\proc-macro2-1.0.107
(no ancestor directory contains both Cargo.toml and crates/).
```

The failing invocation is `target/accelerated-driver/rustc-shim.exe` compiling
`proc-macro2`'s `build.rs`. Cargo runs `rustc` for a build script with the
**package** directory as its working directory, and the shim then calls
`build_guests::workspace_root()`, which walks up from CWD. No ancestor of a
cargo-registry package holds both `Cargo.toml` and `crates/`, so the walk is
guaranteed to fail — for every registry dependency with a build script, i.e. on
any cold accelerated build.

Ticket 71 replaced the compile-time `CARGO_MANIFEST_DIR` root with a runtime CWD
walk for the user-facing commands, and that is right for them: the binary can be
built in one checkout and run in another. The shim is the one caller for which
the CWD is not the tree being built.

## Answer

**Resolved 2026-10-25 inside the [t85](85-seam-inset-offset-batching.md) take as
an infrastructure prerequisite** — the accelerator had to build for that
ticket's gate, and a cold accelerated build could not. This is a carve-out
recorded on the map, not a second wayfinder take.

Root cause confirmed at disk: `run_shim_mode`
(`xtask/src/rustc_driver.rs`) resolved the root with
`build_guests::workspace_root()`, which walks up from the process's **current
directory**. Ticket 71 made that the right contract for the user-facing
commands (the binary can be built in one checkout and run in another), but the
shim is the one caller for which CWD is the *package* directory cargo chose —
a registry path for any dependency with a `build.rs`. The walk cannot succeed
there, so the panic was deterministic for every registry dependency on every
cold accelerated build.

Fix — the entry points hand the shim the root they already resolved, and the
shim re-validates it:

- `PNP_XTASK_WORKSPACE_ROOT` is exported by the three accelerated cargo call
  sites (`guest_build_cargo_command_for_mode` in `xtask/src/build_guests.rs`,
  the accelerated branch in `xtask/src/dist.rs`, and
  `configure_accelerated_host_command` in `xtask/src/test.rs`).
- `resolve_shim_root` (`xtask/src/rustc_driver.rs`) prefers that export,
  falls back to the working-directory walk (hand-run `cargo xtask rustc-shim`
  from inside the repo), then to the directory of the running xtask binary
  (which sits inside the tree that installed the shim). Every candidate must
  hold both `Cargo.toml` and `crates/`, so a bogus export cannot be trusted.
- `run_shim_mode` no longer panics when no candidate resolves; it prints the
  three attempted sources and exits 1.

Verification:

- `cargo test -p xtask` **187 passed / 0 failed**, including
  `shim_root_prefers_the_exported_workspace_root_over_a_dependency_cwd` and
  `shim_root_falls_back_to_the_xtask_binary_directory`.
- The failing command now completes:
  `cargo xtask dist --accelerated --edition developer` built **48 guests** from
  an empty accelerated namespace (the run that previously died on
  `proc-macro2`'s build script) and staged the developer snapshot.
- `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check` both exit 0 afterwards.

Not in scope here: the same CWD-walk contract for any *other* caller that runs
outside the repository (none is known), and the shim's launcher compile step,
which already resolves nothing.

