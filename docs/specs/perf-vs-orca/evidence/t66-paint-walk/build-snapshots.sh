#!/usr/bin/env bash
# t66 snapshot builder — run from the repo root.
#
# Builds ONE arm's complete distribution snapshots (ordinary + accelerated)
# from the CURRENT working tree, into
#   .local-artifacts/perimeter-reference-preparation/t66-paint-walk/snapshots/{ordinary,accelerated}-<arm>/
# The arm argument is a label only; the caller is responsible for the tree
# state (baseline = HEAD with the candidate stashed; candidate = HEAD + the
# take's diff). Both guest freshness checks must exit 0 for the tree state
# before running this script (see the ticket's take pre-declaration).
#
# Usage: docs/specs/perf-vs-orca/evidence/t66-paint-walk/build-snapshots.sh <baseline|candidate>
set -euo pipefail

ARM="${1:?usage: build-snapshots.sh <baseline|candidate>}"
ROOT=".local-artifacts/perimeter-reference-preparation/t66-paint-walk"
mkdir -p "$ROOT/snapshots"

cargo xtask build-guests
cargo xtask dist --edition developer
rm -rf "$ROOT/snapshots/ordinary-$ARM"
mkdir -p "$ROOT/snapshots/ordinary-$ARM"
cp target/dist/developer/pnp_cli.exe "$ROOT/snapshots/ordinary-$ARM/"
cp -r target/dist/developer/modules "$ROOT/snapshots/ordinary-$ARM/modules"

cargo xtask build-guests --accelerated
cargo xtask dist --accelerated --edition developer
rm -rf "$ROOT/snapshots/accelerated-$ARM"
mkdir -p "$ROOT/snapshots/accelerated-$ARM"
cp target/dist-accelerated/developer/pnp_cli.exe "$ROOT/snapshots/accelerated-$ARM/"
cp -r target/dist-accelerated/developer/modules "$ROOT/snapshots/accelerated-$ARM/modules"

( cd "$ROOT/snapshots" && sha256sum ./*/pnp_cli.exe > "hashes-$ARM.txt" )
echo "built arm=$ARM"
