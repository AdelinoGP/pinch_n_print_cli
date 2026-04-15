//! Source-level regression guard for the ConfigView encapsulation
//! contract. Complements
//! `crates/slicer-ir/tests/config_view_encapsulation_tdd.rs`: that test
//! proves the contract is enforced at compile time for external crates;
//! this one guards against the `ConfigView.fields` field being
//! re-exposed as `pub` in the future, which would silently re-open the
//! deviation (docs/03 §host-boundary access enforcement;
//! `wit/deps/config.wit` `resource config-view`).

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

#[test]
fn config_view_backing_map_stays_private() {
    let ir_src = repo_root().join("crates/slicer-ir/src/slice_ir.rs");
    let text = fs::read_to_string(&ir_src).expect("read slice_ir.rs");
    // Find the `pub struct ConfigView { ... }` block and assert no `pub`
    // field inside it. We look for the literal declaration since the
    // grep needs to ignore unrelated structs in the same file.
    let start = text.find("pub struct ConfigView").expect("ConfigView struct present");
    let tail = &text[start..];
    let brace_open = tail.find('{').expect("ConfigView struct open brace");
    // Walk to the matching close brace.
    let mut depth = 0i32;
    let mut end_idx = None;
    for (i, ch) in tail[brace_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(brace_open + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let body_end = end_idx.expect("ConfigView struct close brace");
    let body = &tail[brace_open..body_end];
    assert!(
        !body.contains("pub fields"),
        "regression: `ConfigView.fields` is `pub` again — the \
         read-only/declared-reads-only contract in `wit/deps/config.wit` \
         requires the backing map to stay private (docs/03 \
         §host-boundary enforcement). ConfigView body was:\n{body}"
    );
}

#[test]
fn main_production_entry_path_uses_bind_module_config_view() {
    // The live-plan path constructs per-module ConfigViews via
    // `build_live_execution_plan` → `bind_module_config_view`
    // → `ConfigView::from_declared`, which is the only docs-compliant
    // constructor that pre-filters to the module's declared reads.
    let main = fs::read_to_string(repo_root().join("crates/slicer-host/src/main.rs"))
        .expect("read main.rs");
    assert!(
        main.contains("build_live_execution_plan"),
        "main.rs Run arm must route through build_live_execution_plan so \
         bind_module_config_view runs on the live path"
    );
}
