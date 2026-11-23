"""Strip `#[cfg(feature = "using_z")]` items and z-field lines, then diff.

Recovers the *effective* source delta between clipper2-rust 1.1.0 and 1.2.0 when
the crate is compiled with default features (using_z OFF), which is how this
repo consumes it. Re-derives the table in ../FINDINGS.md.

    python3 strip.py [registry-src-root]

Defaults to `$CARGO_HOME/registry/src/*` (the first index dir found). Both
crate sources must be unpacked there; `cargo fetch` in a tree pinned to each
version does that.
"""

import re, sys, pathlib

CFG = re.compile(r'^\s*#\[cfg\(feature\s*=\s*"using_z"\)\]\s*$')


def _skip_brace_block(lines, j, n):
    depth = 0
    started = False
    k = j
    while k < n:
        for ch in lines[k]:
            if ch == "{":
                depth += 1
                started = True
            elif ch == "}":
                depth -= 1
        k += 1
        if started and depth == 0:
            break
    return k


def strip(lines):
    out = []
    i, n = 0, len(lines)
    while i < n:
        ln = lines[i]
        if CFG.match(ln):
            j = i + 1
            while j < n and lines[j].strip() == "":
                j += 1
            if j >= n:
                break
            s = lines[j].strip()
            if s.startswith("{"):
                i = _skip_brace_block(lines, j, n)
                continue
            if re.match(r"^(pub\s+)?z\s*:", s):
                i = j + 1
                continue
            if re.match(r"^(pub\s+)?type\s+", s):
                i = j + 1
                continue
            if re.match(r"^(pub\s+)?use\s+", s):
                i = j + 1
                continue
            if re.match(r"^(pub\s+)?(async\s+)?fn\s+\w+", s):
                i = _skip_brace_block(lines, j, n)
                continue
            if s.startswith("impl") or s.startswith("unsafe impl"):
                i = _skip_brace_block(lines, j, n)
                continue
            if re.match(r"^(pub\s+)?const\s+", s) or re.match(
                r"^(pub\s+)?static\s+", s
            ):
                i = j + 1
                continue
            # unknown item: drop just the attribute, keep the item
            i += 1
            continue
        out.append(ln)
        i += 1
    return out


def unify(text):
    """Collapse whitespace so reformatting and constructor sugar compare equal."""
    lines = strip(text.splitlines())
    # canonicalise Point ctor spellings
    joined = "\n".join(lines)
    joined = re.sub(r"\bPoint64::new\(([^()]*?)\)", r"Point64{ \1 }", joined)
    joined = re.sub(r"\bPointD::new\(([^()]*?)\)", r"PointD{ \1 }", joined)
    joined = re.sub(r"\bPoint::<f64>::new\(([^()]*?)\)", r"PointD{ \1 }", joined)
    joined = re.sub(r"\bPoint::<i64>::new\(([^()]*?)\)", r"Point64{ \1 }", joined)
    out = []
    for ln in joined.splitlines():
        s = re.sub(r"\s+", " ", ln).strip()
        if s:
            out.append(s)
    return out


MODS = [
    "clipper.rs",
    "core.rs",
    "engine.rs",
    "engine_fns.rs",
    "engine_public.rs",
    "lib.rs",
    "offset.rs",
    "minkowski.rs",
    "rectclip.rs",
    "version.rs",
    "utils.rs",
]

if __name__ == "__main__":
    import glob
    import os

    if len(sys.argv) > 1:
        R = pathlib.Path(sys.argv[1])
    else:
        cargo_home = pathlib.Path(
            os.environ.get("CARGO_HOME", pathlib.Path.home() / ".cargo")
        )
        cands = sorted(glob.glob(str(cargo_home / "registry/src/*")))
        if not cands:
            print("no registry src dir found; pass it explicitly")
            sys.exit(2)
        R = pathlib.Path(cands[0])
    A = R / "clipper2-rust-1.1.0/src"
    B = R / "clipper2-rust-1.2.0/src"
    for p in (A, B):
        if not p.is_dir():
            print(f"missing {p}; `cargo fetch` a tree pinned to each version")
            sys.exit(2)
    total = 0
    for mod in MODS:
        a, b = A / mod, B / mod
        if not a.exists() or not b.exists():
            print(
                f"### {mod}: present on only one side "
                f"(1.1.0={a.exists()} 1.2.0={b.exists()})"
            )
            continue
        na = unify(a.read_text(encoding="utf8", errors="replace"))
        nb = unify(b.read_text(encoding="utf8", errors="replace"))
        import difflib

        sm = difflib.SequenceMatcher(None, na, nb, autojunk=False)
        ops = [o for o in sm.get_opcodes() if o[0] != "equal"]
        if not ops:
            print(f"### {mod}: EFFECTIVELY IDENTICAL (using_z stripped)")
            continue
        print(f"### {mod}: {len(ops)} changed hunk(s)")
        total += len(ops)
        for tag, i1, i2, j1, j2 in ops:
            print(f"  -- {tag} (1.1.0 {i1}:{i2} -> 1.2.0 {j1}:{j2})")
            for ln in na[i1:i2]:
                print(f"     - {ln}")
            for ln in nb[j1:j2]:
                print(f"     + {ln}")
    print(f"\nTOTAL changed hunks (feature-off): {total}")
