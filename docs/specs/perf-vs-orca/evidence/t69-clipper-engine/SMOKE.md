# Dual-engine smoke measurement

Measured actual results; nanoseconds. One untimed warm-up, then five repeats per case.
Each engine entry is median(across cases, median(five repeats)); ratio is Rust / C++.

| Opcode | Cases | Rust median ns | C++ median ns | Rust / C++ | Ordered hash match | Shape-count match |
|---|---:|---:|---:|---:|---:|---:|
| 2 | 16 | 106,491,750.0 | 40,378,050.0 | 2.6374 | 16/16 | 16/16 |

Opcode 2 ordered-hash divergences: 0 of 16 cases.

## Method and limitations

The t55 corpus includes all ordinary/accelerated base/benchy captures; mode-identical duplicates are deliberately retained, not independent workloads.
Only union is covered by t55; no offset, difference, intersection or xor headroom is inferred from this run.
Timing includes engine creation, AddSubject/AddClip or AddPaths, Execute into PolyTree, and engine destruction. It excludes parsing, input flattening, result hashing, tree traversal and output tree destruction.
Fresh engines are constructed for every repeat. Repeat hashes/counts are checked against the warm-up result.
Output rings are raw engine results grouped as outer + direct holes; nested islands precede enclosing outers. No coordinate, ring-start, orientation or component-order normalization is performed.
Ordered coordinate hash disagreement is not a geometric equivalence verdict. Rust 1.1.0 vs canonical C++ 1.5.2 can differ (ticket 31 clean_collinear delta); divergences are disclosed, not investigated here.
FNV-1a hashes use x then y as little-endian two's-complement u64 bytes for every vertex, no framing bytes. Shape counts are compared separately; collisions remain possible.
These are microbenchmarks under different compilers, runtime allocators, and engine revisions. They size possible headroom, not like-for-like optimization speedups or slicer wall-time savings. Sequential execution is not a paired process-level benchmark and has no CPU isolation.

## Toolchain and seal

```text
rustc 1.96.0 (ac68faa20 2026-05-25)
binary: rustc
commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
commit-date: 2026-05-25
host: x86_64-pc-windows-msvc
release: 1.96.0
LLVM version: 22.1.2
g++ (x86_64-posix-seh-rev0, Built by MinGW-Builds project) 16.1.0
Copyright (C) 2026 Free Software Foundation, Inc.
This is free software; see the source for copying conditions.  There is NO
warranty; not even for MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
```

C++ uses g++ -O3 -std=c++17 and static runtime linking; Rust uses cargo build --release --locked --offline (default release optimization).
Crate package SHA-256: `0fd663fe209e7030c956e3be4c051dcc20cdb73da794f31466762cff12ca11bf`.
Registry files match the lock-checksummed cached .crate archive; scratch copy is byte-identical. Own Cargo.lock resolves exactly clipper2-rust 1.1.0, with no registry source. Seal passed before and after build; root lock and canonical sources unchanged.

## Raw results (gitignored)

* rust: `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t55-smoke/rust.jsonl` (SHA-256 `529bd9999babf2bfcdd2c60fd25f91a89412378718ae9cd1b3525705b0944deb`).
* cpp: `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t55-smoke/cpp.jsonl` (SHA-256 `d7546750bc1f1f5fae18d625828bad49e3830f1e05596e95be96d0082aae543c`).
* Reduction: `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t55-smoke/summary.json`.
* Build provenance and exact compiler argv/cwd: `.local-artifacts/perf-vs-orca/t69-clipper-engine/seal.json`.

Reproduce with the commands in HARNESS.md. No production tests or end-to-end parity qualification were run.

## Independent protocol validation

* cpp: 6 known-coordinate rectangle/empty-output controls passed; unknown argument rejected: True.
* rust: 6 known-coordinate rectangle/empty-output controls passed; unknown argument rejected: True.
* Saved result: `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/protocol-controls/verification.json`. These controls cover all opcodes, negative coordinates, named site tokens, optional call index and the FNV byte definition; they are not performance evidence.
