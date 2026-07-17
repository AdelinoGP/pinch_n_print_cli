# ADR-0044: The WIT world version is not an identity token

Status: accepted

Packet 130 bumped `slicer:world-layer` 1.0.0 → 1.1.0 for a one-line contract
change and touched 107 files. Measured attribution: **77 of them changed only to
re-spell that version string** — 40 tests, 23 manifests/fixtures, 9 src files, a
bench and a doc. The packet's own ~30-file estimate for its real change was
accurate. The other 77 were a tax the design levied on anyone who obeyed the
versioning policy.

Nobody had paid it before because **packet 130 was the first world-version bump in
the project's history**. `world-layer.wit` had read `@1.0.0` continuously since
packet 72 wrote it. Packet 114 restructured the world — hoisting the entire
`host-services` interface out into `slicer:common` — and did not bump it at all.
The design had been free for 150 packets only because the policy was never
followed.

## Decision

The world version lives **solely** in the `package` line of
`crates/slicer-schema/wit/deps/world-*/*.wit`. It is a changelog annotation, not
part of module identity. `wit-world` in a module manifest names an **unversioned**
package (`slicer:world-layer`); a declared version is rejected at load with a
diagnostic naming the corrected value.

## Why the version cannot be an identity token

Not "should not" — **cannot**. Our worlds export bare freestanding funcs, and a
bare extern name carries no semver suffix. Component-model `WIT.md` is explicit
that `<semversuffix>` is a production of `<interfacename>`, not of a plain name.
The version is therefore erased at compile time:

```
$ wasm-tools component wit modules/core-modules/arachne-perimeters/arachne-perimeters.wasm | grep -c world-layer
0
$ ... | grep -oE "package [a-z:-]+(@[0-9.]+)?"
package root:component
package slicer:common
package slicer:config
package slicer:ir-handles
package slicer:types
```

Every surviving package is unversioned; the string `world-layer` does not appear
in the binary at all, in the decoded WIT or the raw bytes.

So a versioned `wit-world` was an **unfalsifiable claim** — there is no fact
anywhere in the system to check it against. That directly contradicts `docs/03`
rule 1, "the host never trusts module declarations at runtime": the host was
comparing one hand-written string to another hand-written string, with no
connection to the artifact either described. A module could declare `@1.1.0` and
ship a binary built against `@1.0.0`; the check passed.

The check was also weaker than documented. `docs/03` specified matching on
"package name and major version"; the implementation was
`WIT_WORLD_ALLOWLIST.contains(&wit_world)` — exact string equality. A test named
`wit_world_major_version_mismatch_rejects_future_major` certified the missing
logic while passing vacuously: it would have passed for any unlisted string, and
would have passed with the version handling entirely absent, which it was.

Meanwhile four hand-copied sources of the world id raced one constant nobody had
wired up (`SUPPORTED_WIT_WORLDS` — defined, zero consumers).

## What actually enforces compatibility

| Guard | Real? | Catches |
|---|---|---|
| wasmtime typed instantiation | yes — the strongest | structural export/signature mismatch, at first dispatch |
| `cargo xtask build-guests --check` | yes — load-bearing | stale in-tree guest (mtime-based) |
| `[compatibility]` min/max-ir-schema | yes, fatal at startup | IR range (currently slack: everything declares 1.0.0–5.0.0) |
| `wit-world` allowlist | ran, caught nothing real | only a module that *honestly self-reports* being wrong |
| `[module] version` field | no — parsed, stored, zero consumers | — |

`wit_world()` had no production callers at all: dispatch selects the world by
`stage_id`, never by the manifest. A manifest could declare `world-prepass` on a
`Layer::Infill` stage and the host would instantiate `LayerModule` regardless.

## Consequences

- Bumping a world edits **2 files** (the `.wit` and the pin in
  `wit_drift_detection_tdd`), down from 79. Verified by simulated bump: workspace
  compiles, 196 contract tests pass.
- `no_versioned_world_identifiers_outside_canonical_wit` fails if
  `slicer:world-x@N` reappears in any `.rs` or `.toml`, in either the bare or
  package-qualified (`slicer:world-layer/layer-module@2.0.0`) shape.
- **Breaking:** a manifest declaring a versioned `wit-world` no longer loads. No
  registry or install path exists and no out-of-tree module was found, so this is
  the cheapest moment to make the break.
- The version now enforces nothing at all — honestly, rather than by accident.
  Giving it teeth requires ADR-0045.

## Alternatives rejected

- **Fix the code to match the docs (major-version comparison).** Would stop
  minor bumps churning, but still compares an unverifiable declaration. It buys a
  fast-fail diagnostic that wasmtime already provides with a better error.
- **Keep exact pinning, add a codemod.** Fixes the labor, keeps the churn in every
  diff, and leaves the docs contradicting the code.
- **Read the version from the guest binary.** The obvious fix, and impossible: the
  binary does not carry it (above). This was proposed during review and killed by
  `wasm-tools`.
