"""t70: vendor pristine clipper2-rust 1.1.0 in-tree at vendor/clipper2-rust.

Discipline (t69 setup-engines.py shape): the registry extraction is verified
against the cached .crate archive, whose SHA-256 must equal the root
Cargo.lock's published checksum (0fd663fe...). The vendor tree is
byte-identical to the registry extraction — NO sentinel, NO edits: it is a
plain member of the root workspace (members list in the root Cargo.toml),
the same role crates/slicer-core already plays for the independent
wit-guest workspaces. slicer-core path-depends on it explicitly so the
guest freshness fingerprint walk covers engine source changes.

The registry is never mutated. An existing vendor/ tree that differs from
the registry extraction causes failure, not overwrite. Idempotent: a second
run re-verifies everything and exits 0 without changing bytes.
"""

import hashlib
import json
import sys
import tarfile
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
VENDOR = REPO / "vendor" / "clipper2-rust"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(path: Path) -> dict:
    return {
        p.relative_to(path).as_posix(): digest(p)
        for p in sorted(path.rglob("*"))
        if p.is_file()
    }


PUBLISHED_CHECKSUM = "0fd663fe209e7030c956e3be4c051dcc20cdb73da794f31466762cff12ca11bf"


def main() -> int:
    cache = list((Path.home() / ".cargo/registry/src").glob("*/clipper2-rust-1.1.0"))
    assert len(cache) == 1, cache
    upstream = cache[0]
    original = inventory(upstream)
    archive = (
        Path.home()
        / ".cargo/registry/cache"
        / upstream.parent.name
        / "clipper2-rust-1.1.0.crate"
    )
    locked = [
        p
        for p in tomllib.loads((REPO / "Cargo.lock").read_text())["package"]
        if p["name"] == "clipper2-rust" and p["version"] == "1.1.0"
    ]
    assert len(locked) == 1, "root Cargo.lock must pin exactly one clipper2-rust 1.1.0"
    archive_sha = digest(archive)
    # Pre-vendoring the lock entry carries the published registry checksum; once
    # the vendored path dep lands, the entry is a path package with no checksum.
    # The published checksum is a pinned constant here (the provenance the t70
    # ticket requires recorded), verified against the cached archive either way.
    assert archive_sha == PUBLISHED_CHECKSUM, "crate archive checksum mismatch"
    if "checksum" in locked[0]:
        assert archive_sha == locked[0]["checksum"], "lock checksum mismatch"
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            if member.isfile():
                relative = Path(member.name).relative_to("clipper2-rust-1.1.0")
                assert (upstream / relative).read_bytes() == tar.extractfile(
                    member
                ).read(), (
                    "registry differs from archive",
                    str(relative),
                )
    print(f"[t70] registry extraction verified against archive sha256={archive_sha}")

    if not VENDOR.exists():
        VENDOR.mkdir(parents=True)
    copied = inventory(VENDOR)
    if copied:
        # Idempotent re-verification: the vendor tree must be byte-identical
        # to the registry extraction (no delta at all).
        assert copied == original, (
            "existing vendor/ tree differs; refusing to overwrite"
        )
        print("[t70] existing vendor tree re-verified byte-identical")
    else:
        for src in sorted(upstream.rglob("*")):
            if src.is_file():
                rel = src.relative_to(upstream)
                dst = VENDOR / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.write_bytes(src.read_bytes())
        copied = inventory(VENDOR)
        assert copied == original, "fresh copy mismatch"
        print("[t70] vendored byte-identical copy created")
    assert (
        tomllib.loads((VENDOR / "Cargo.toml").read_text())["package"]["version"]
        == "1.1.0"
    )
    members = tomllib.loads((REPO / "Cargo.toml").read_text())["workspace"]["members"]
    assert "vendor/clipper2-rust" in members, "vendored crate missing from members"

    provenance = {
        "registry_path": str(upstream),
        "crate_archive_sha256": archive_sha,
        "root_lock_checksum": archive_sha,
        "registry_matches_verified_archive": True,
        "vendor_path": str(VENDOR),
        "vendor_inventory_sha256": hashlib.sha256(
            json.dumps(copied, sort_keys=True).encode()
        ).hexdigest(),
        "vendor_inventory": copied,
        "pristine_copy_byte_identical": True,
        "workspace_member": "vendor/clipper2-rust",
    }
    (HERE / "vendor-provenance.json").write_text(
        json.dumps(provenance, indent=2, sort_keys=True) + "\n"
    )
    print("[t70] provenance written:", HERE / "vendor-provenance.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
