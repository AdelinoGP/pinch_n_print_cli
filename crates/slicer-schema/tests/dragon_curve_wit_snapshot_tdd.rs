//! Snapshot-parity guard for the committed dragon-curve community example.
//!
//! The example vendors a frozen copy of the WIT closure it needs, and it is
//! deliberately outside `cargo xtask build-guests` discovery (see the README's
//! "Do not add real community modules to this repository" banner). Nothing
//! rebuilds it or checks it, so the copy silently rotted: its dependency
//! packages drifted from canonical and, because WIT type identity is
//! structural, the component's `slicer:ir-handles/ir-handles` import stopped
//! matching the host. The committed artifact failed typed instantiation at
//! first dispatch with:
//!
//! ```text
//! phase TypedInstantiation: component imports instance
//! `slicer:ir-handles/ir-handles`, but a matching implementation was not found
//! in the linker
//! ```
//!
//! Nothing in the test suite caught that, because no test dispatches the
//! example. This test is the cheap mechanical substitute: the vendored
//! dependency packages must stay byte-identical to canonical. It does not
//! dispatch the component (that needs the MoonBit toolchain); it fails loudly
//! the moment a host WIT change makes the snapshot stale, pointing the author
//! at `make` in the module directory.
//!
//! When this test fails, the fix is:
//!
//! ```text
//! cp crates/slicer-schema/wit/deps/{types,config,common,ir-types}.wit \
//!    modules/community-modules/dragon-curve/wit/deps/<pkg>/
//! cd modules/community-modules/dragon-curve && make
//! ```
//!
//! and commit the rebuilt `dragon-curve.wasm` (the artifact is the shipped
//! product; a stale one defeats the source fix).

#![allow(missing_docs)]

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/slicer-schema
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must resolve")
}

/// Canonical WIT deps live flat (`deps/types.wit`); the example vendors them in
/// per-package directories (`deps/types/types.wit`). Pair them explicitly so a
/// missing vendored file fails as a named absence rather than a silent skip.
const PACKAGES: &[(&str, &str)] = &[
    ("types.wit", "types/types.wit"),
    ("config.wit", "config/config.wit"),
    ("common.wit", "common/common.wit"),
    ("ir-types.wit", "ir-types/ir-types.wit"),
];

#[test]
fn dragon_curve_vendored_wit_matches_canonical() {
    let root = workspace_root();
    let example = root.join("modules/community-modules/dragon-curve");
    let canonical_dir = root.join("crates/slicer-schema/wit/deps");
    let vendored_dir = example.join("wit/deps");

    assert!(
        example.is_dir(),
        "the dragon-curve example must be present at {}",
        example.display()
    );

    for (canonical_rel, vendored_rel) in PACKAGES {
        let canonical = canonical_dir.join(canonical_rel);
        let vendored = vendored_dir.join(vendored_rel);

        let canonical_text = std::fs::read_to_string(&canonical).unwrap_or_else(|e| {
            panic!(
                "canonical WIT {} must be readable: {e}",
                canonical.display()
            )
        });
        let vendored_text = std::fs::read_to_string(&vendored).unwrap_or_else(|e| {
            panic!("vendored WIT {} must be readable: {e}", vendored.display())
        });

        assert_eq!(
            vendored_text, canonical_text,
            "\nThe dragon-curve example's vendored {vendored_rel} has drifted from \
             canonical {canonical_rel}.\n\
             WIT type identity is structural: any shape difference makes the \
             component's `slicer:ir-handles/ir-handles` import unmatched, so it \
             fails typed instantiation at first dispatch (measurably, the \
             committed artifact silently stopped loading this way).\n\
             Refresh the whole snapshot to canonical and rebuild the artifact:\n\
             copy each canonical crates/slicer-schema/wit/deps/<pkg>.wit into the \
             matching modules/community-modules/dragon-curve/wit/deps/<pkg>/ \
             directory, then run `make` in the example directory, and commit the \
             rebuilt dragon-curve.wasm.\n"
        );
    }
}

/// The example's own world file must track canonical too: it names the exported
/// interface the host binds (`slicer:layer-infill/infill@1.0.0#run`), so a
/// divergence there breaks dispatch for the same structural reason.
#[test]
fn dragon_curve_world_matches_canonical() {
    let root = workspace_root();
    let canonical = root.join("crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit");
    let vendored = root.join("modules/community-modules/dragon-curve/wit/layer-infill.wit");

    let canonical_text = std::fs::read_to_string(&canonical).unwrap_or_else(|e| {
        panic!(
            "canonical world {} must be readable: {e}",
            canonical.display()
        )
    });
    let vendored_text = std::fs::read_to_string(&vendored).unwrap_or_else(|e| {
        panic!(
            "vendored world {} must be readable: {e}",
            vendored.display()
        )
    });

    assert_eq!(
        vendored_text, canonical_text,
        "\nThe dragon-curve example's vendored layer-infill.wit has drifted from \
         canonical. Refresh it and rebuild the artifact (see the sibling \
         dependency-parity test for the full recipe).\n"
    );
}
