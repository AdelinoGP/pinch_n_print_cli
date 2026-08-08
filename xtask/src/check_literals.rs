use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::visit::Visit;
use syn::{ExprStruct, Fields, File, ItemImpl, ItemMod, ItemStruct, Type, Visibility};
use walkdir::WalkDir;

pub(crate) struct Violation {
    file: String,
    line: usize,
    type_name: String,
}

#[derive(Clone, Copy)]
enum ScanMode {
    WholeFile,
    CfgTestOnly,
}

struct LiteralVisitor<'a> {
    file_label: &'a str,
    lines: Vec<&'a str>,
    mode: ScanMode,
    watch: &'a BTreeSet<String>,
    impl_targets: Vec<String>,
    cfg_test_depth: usize,
    violations: Vec<Violation>,
}

impl<'a> LiteralVisitor<'a> {
    fn in_scope(&self) -> bool {
        matches!(self.mode, ScanMode::WholeFile) || self.cfg_test_depth > 0
    }

    fn cfg_test_module(item: &ItemMod) -> bool {
        item.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Ident>()
                    .map(|ident| ident == "test")
                    .unwrap_or(false)
        })
    }

    fn scan_macro_tokens(&mut self, tokens: TokenStream) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for index in 0..tokens.len() {
            let TokenTree::Ident(ident) = &tokens[index] else {
                if let TokenTree::Group(group) = &tokens[index] {
                    self.scan_macro_tokens(group.stream());
                }
                continue;
            };

            if let Some(TokenTree::Group(group)) = tokens.get(index + 1) {
                if group.delimiter() == Delimiter::Brace {
                    let type_name = if ident == "Self" {
                        self.impl_targets.last().cloned()
                    } else {
                        Some(ident.to_string())
                    };
                    let has_top_level_rest = tokens_have_top_level_rest(group.stream());
                    if self.in_scope()
                        && !has_top_level_rest
                        && type_name
                            .as_deref()
                            .is_some_and(|name| self.watch.contains(name))
                    {
                        let line = ident.span().start().line;
                        if !has_waiver(&self.lines, line) {
                            self.violations.push(Violation {
                                file: self.file_label.to_owned(),
                                line,
                                type_name: type_name.expect("watched macro type name"),
                            });
                        }
                    }
                }
            }

            if let TokenTree::Group(group) = &tokens[index] {
                self.scan_macro_tokens(group.stream());
            }
        }
    }
}

impl<'ast> Visit<'ast> for LiteralVisitor<'_> {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let is_cfg_test = matches!(self.mode, ScanMode::CfgTestOnly) && Self::cfg_test_module(item);
        if is_cfg_test {
            self.cfg_test_depth += 1;
        }
        syn::visit::visit_item_mod(self, item);
        if is_cfg_test {
            self.cfg_test_depth -= 1;
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let target = match &*item.self_ty {
            Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            _ => None,
        };
        let pushed = target.is_some();
        if let Some(target) = target {
            self.impl_targets.push(target);
        }
        syn::visit::visit_item_impl(self, item);
        if pushed {
            self.impl_targets.pop();
        }
    }

    fn visit_expr_struct(&mut self, expr: &'ast ExprStruct) {
        if self.in_scope() && expr.qself.is_none() && expr.rest.is_none() {
            let segment = expr.path.segments.last();
            let type_name = segment.map(|segment| segment.ident.to_string());
            let resolved = (type_name.as_deref() == Some("Self"))
                .then(|| self.impl_targets.last().cloned())
                .flatten()
                .or(type_name.clone());
            if let Some(type_name) = resolved {
                let line = expr.brace_token.span.open().start().line;
                if self.watch.contains(&type_name) && !has_waiver(&self.lines, line) {
                    self.violations.push(Violation {
                        file: self.file_label.to_owned(),
                        line,
                        type_name,
                    });
                }
            }
        }
        syn::visit::visit_expr_struct(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.scan_macro_tokens(mac.tokens.clone());
    }
}

fn tokens_have_top_level_rest(tokens: TokenStream) -> bool {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    tokens.windows(2).any(|pair| {
        matches!((&pair[0], &pair[1]), (TokenTree::Punct(first), TokenTree::Punct(second))
            if first.as_char() == '.' && second.as_char() == '.')
    })
}

fn has_waiver(lines: &[&str], line_1based: usize) -> bool {
    let end = line_1based
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let start = end.saturating_sub(1);
    (start..=end).any(|index| {
        lines[index]
            .split_once("// exhaustive:")
            .is_some_and(|(_, reason)| !reason.trim().is_empty())
    })
}

fn scan_source(
    file_label: &str,
    src: &str,
    mode: ScanMode,
    watch: &BTreeSet<String>,
) -> Vec<Violation> {
    let file = match syn::parse_file(src) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("xtask: cannot parse {file_label}: {error}");
            return Vec::new();
        }
    };
    let mut visitor = LiteralVisitor {
        file_label,
        lines: src.lines().collect(),
        mode,
        watch,
        impl_targets: Vec::new(),
        cfg_test_depth: 0,
        violations: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.violations
}

struct WatchlistVisitor {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for WatchlistVisitor {
    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if matches!(item.vis, Visibility::Public(_))
            && matches!(&item.fields, Fields::Named(fields) if fields.named.len() >= 5)
        {
            self.names.insert(item.ident.to_string());
        }
        syn::visit::visit_item_struct(self, item);
    }
}

fn collect_watchlist(file: &File) -> BTreeSet<String> {
    let mut visitor = WatchlistVisitor {
        names: BTreeSet::new(),
    };
    visitor.visit_file(file);
    visitor.names
}

fn watchlist_from_source(source: &str) -> Result<BTreeSet<String>, syn::Error> {
    syn::parse_file(source).map(|file| collect_watchlist(&file))
}

fn derive_watchlist(ws: &Path) -> BTreeSet<String> {
    let mut watchlist = BTreeSet::new();
    let crates = ws.join("crates");
    let entries = match fs::read_dir(&crates) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("xtask: cannot read {}: {error}", crates.display());
            return watchlist;
        }
    };

    for entry in entries.flatten() {
        let src = entry.path().join("src");
        if !src.is_dir() {
            continue;
        }
        for file in WalkDir::new(src).into_iter().filter_map(Result::ok) {
            if !file.file_type().is_file()
                || file.path().extension().and_then(|ext| ext.to_str()) != Some("rs")
            {
                continue;
            }
            let path = file.path();
            match fs::read_to_string(path) {
                Ok(source) => match watchlist_from_source(&source) {
                    Ok(names) => watchlist.extend(names),
                    Err(error) => eprintln!("xtask: cannot parse {}: {error}", path.display()),
                },
                Err(error) => eprintln!("xtask: cannot read {}: {error}", path.display()),
            }
        }
    }
    watchlist
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn matches_filter(relative: &Path, filter: &str) -> bool {
    let relative = normalized_path(relative);
    let filter = filter.replace('\\', "/").trim_matches('/').to_owned();
    relative == filter || relative.starts_with(&(filter + "/"))
}

fn collect_enforced_files(ws: &Path, filters: &[String]) -> Vec<(PathBuf, ScanMode)> {
    let mut files = Vec::new();
    let mut collect_scope = |root: PathBuf, mode: ScanMode| {
        if !root.is_dir() {
            return;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(|ext| ext.to_str()) != Some("rs")
            {
                continue;
            }
            let relative = path.strip_prefix(ws).unwrap_or(path);
            if filters.is_empty()
                || filters
                    .iter()
                    .any(|filter| matches_filter(relative, filter))
            {
                files.push((path.to_path_buf(), mode));
            }
        }
    };

    let crates = ws.join("crates");
    if let Ok(entries) = fs::read_dir(&crates) {
        for entry in entries.flatten() {
            let crate_dir = entry.path();
            collect_scope(crate_dir.join("tests"), ScanMode::WholeFile);
            collect_scope(crate_dir.join("benches"), ScanMode::WholeFile);
            collect_scope(crate_dir.join("src"), ScanMode::CfgTestOnly);
        }
    }

    let modules = ws.join("modules/core-modules");
    if let Ok(entries) = fs::read_dir(modules) {
        for entry in entries.flatten() {
            collect_scope(entry.path().join("tests"), ScanMode::WholeFile);
        }
    }

    files.sort_by_key(|(path, _)| normalized_path(path));
    files
}

pub(crate) fn run(ws: &Path, report: bool, path_filters: &[String]) -> i32 {
    let watchlist = derive_watchlist(ws);
    let files = collect_enforced_files(ws, path_filters);
    let mut violations = Vec::new();
    for (path, mode) in &files {
        let relative = path.strip_prefix(ws).unwrap_or(path);
        let label = normalized_path(relative);
        match fs::read_to_string(path) {
            Ok(source) => violations.extend(scan_source(&label, &source, *mode, &watchlist)),
            Err(error) => eprintln!("xtask: cannot read {}: {error}", label),
        }
    }
    violations.sort_by(|left, right| left.file.cmp(&right.file).then(left.line.cmp(&right.line)));
    for violation in &violations {
        println!(
            "{}:{}: exhaustive literal of watched type `{}`",
            violation.file, violation.line, violation.type_name
        );
    }
    let files_with_violations = violations
        .iter()
        .map(|violation| violation.file.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    println!(
        "check-literals: {} violation(s) in {} file(s) (watchlist: {} types)",
        violations.len(),
        files_with_violations,
        watchlist.len()
    );
    if violations.is_empty() || report {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{matches_filter, scan_source, watchlist_from_source, ScanMode};

    #[test]
    fn watchlist_includes_pub_ge5_named_structs_only() {
        let source = r#"
            pub struct Watched {
                a: u8,
                b: u8,
                c: u8,
                d: u8,
                e: u8,
            }

            pub(crate) struct CrateVisible {
                a: u8,
                b: u8,
                c: u8,
                d: u8,
                e: u8,
                f: u8,
            }

            pub struct Tuple(u8, u8, u8, u8, u8, u8);

            pub struct TooSmall {
                a: u8,
                b: u8,
                c: u8,
                d: u8,
            }

            mod nested {
                pub struct NestedWatched {
                    a: u8,
                    b: u8,
                    c: u8,
                    d: u8,
                    e: u8,
                }
            }
        "#;

        let watchlist = watchlist_from_source(source).expect("fixture should parse");

        assert!(watchlist.contains("Watched"));
        assert!(watchlist.contains("NestedWatched"));
        assert!(!watchlist.contains("CrateVisible"));
        assert!(!watchlist.contains("Tuple"));
        assert!(!watchlist.contains("TooSmall"));
    }

    fn watched() -> BTreeSet<String> {
        ["Watched".to_owned()].into_iter().collect()
    }

    #[test]
    fn scan_passes_fru_and_waivered_literals() {
        let source = r#"
            struct Watched { a: u8, b: u8 }
            fn check(base: Watched) {
                let _ = Watched { a: 1, ..base };
                let _ = Watched { a: 1, b: 2 }; // exhaustive: intentional fixture
                // exhaustive: another intentional fixture
                let _ = Watched { a: 1, b: 2 };
            }
            impl Watched {
                fn make(base: Watched) -> Self { Self { a: 1, ..base } }
            }
        "#;

        assert!(scan_source("fixture.rs", source, ScanMode::WholeFile, &watched()).is_empty());
    }

    #[test]
    fn scan_flags_exhaustive_watched_literals() {
        let source = "fn check() {\n    let _ = Watched { a: 1, b: 2 };\n}\n";
        let violations = scan_source("fixture.rs", source, ScanMode::WholeFile, &watched());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2);
        assert_eq!(violations[0].type_name, "Watched");
    }

    #[test]
    fn scan_flags_self_in_impl_blocks() {
        let source = r#"
            struct Watched { a: u8, b: u8 }
            struct Unwatched { a: u8, b: u8 }
            impl Watched {
                fn make() -> Self { Self { a: 1, b: 2 } }
            }
            impl Unwatched {
                fn make() -> Self { Self { a: 1, b: 2 } }
            }
        "#;
        let violations = scan_source("fixture.rs", source, ScanMode::WholeFile, &watched());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].type_name, "Watched");
    }

    #[test]
    fn scan_ignores_enum_variants_and_non_test_src() {
        let source = r#"
            enum SomeEnum { Variant { a: u8, b: u8 } }
            fn enum_variant() { let _ = SomeEnum::Variant { a: 1, b: 2 }; }
            fn outside() { let _ = Watched { a: 1, b: 2 }; }
            #[cfg(test)]
            mod tests {
                fn check() { let _ = Watched { a: 1, b: 2 }; }
            }
        "#;
        let violations = scan_source("fixture.rs", source, ScanMode::CfgTestOnly, &watched());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].type_name, "Watched");
    }

    #[test]
    fn scan_requires_waiver_reason() {
        let source = "// exhaustive:   \nfn check() { let _ = Watched { a: 1, b: 2 }; }\n";
        let violations = scan_source("fixture.rs", source, ScanMode::WholeFile, &watched());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2);
    }

    #[test]
    fn scan_flags_macro_embedded_and_multisegment_literals() {
        let source = r#"fn check(base: Watched) {
    let _ = vec![Watched { a: 1, b: 2 }];
    assert_eq!(value, Watched { a: 1, b: 2 });
    let _ = slicer_ir::Watched { a: 1, b: 2 };
    let _ = vec![Watched { a: 1, ..base }];
}
"#;
        let violations = scan_source("fixture.rs", source, ScanMode::WholeFile, &watched());

        assert_eq!(violations.len(), 3);
        assert_eq!(violations[0].line, 2);
        assert_eq!(violations[1].line, 3);
        assert_eq!(violations[2].line, 4);
    }

    /// Locked blind spot: a top-level range `..` in a macro token tree is
    /// interpreted as struct-update rest, suppressing this exhaustive literal.
    #[test]
    fn scan_macro_range_blind_spot_documented() {
        let source = r#"fn check() {
    assert_eq!(value, Watched { field: 0..2 });
}
"#;
        let violations = scan_source("fixture.rs", source, ScanMode::WholeFile, &watched());

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn path_filter_is_component_aware_and_exempts_deeper_guest_tree() {
        assert!(matches_filter(
            std::path::Path::new("crates/slicer-ir/tests/foo.rs"),
            "crates/slicer-ir"
        ));
        assert!(!matches_filter(
            std::path::Path::new("crates/slicer-ir-extra/tests/foo.rs"),
            "crates/slicer-ir"
        ));
        assert!(!matches_filter(
            std::path::Path::new("crates/slicer-wasm-host/test-guests/foo/src/lib.rs"),
            "crates/slicer-wasm-host/src"
        ));
    }
}
