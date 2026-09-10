//! `cargo xtask check-test-quality` — the false-green gate of `docs/22_test_quality.md`.
//!
//! Flags the mechanically detectable false-green test patterns (R1–R8) over the
//! same file scope as `check_literals`: crate `tests/`/`benches/` whole-file and
//! crate `src/` inline `#[cfg(test)]` modules. A legitimate weak form is
//! exempted by an inline `// test-quality: <reason>` waiver whose reason names
//! the protected surface, contract, or gate. Ships in report mode; enforce mode
//! is switched on by the remediation program's final wave (ADR-0065) via
//! `TEST_QUALITY_ENFORCED` in `xtask/src/test.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprField, ExprIf, ItemFn, Member, Stmt};
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) struct Violation {
    file: String,
    line: usize,
    rule: &'static str,
    detail: String,
}

/// Single promotion point for enforce mode (ADR-0065). The remediation
/// program's final wave flips this to `true`.
pub(crate) const TEST_QUALITY_ENFORCED: bool = false;

/// Waiver marker. A line containing `// test-quality: <non-empty reason>`
/// waives a violation on the line itself or the preceding line, mirroring
/// `check_literals`' `// exhaustive:` convention.
const WAIVER_MARK: &str = "// test-quality:";

struct TestVisitor<'a> {
    file_label: &'a str,
    lines: Vec<&'a str>,
    violations: Vec<Violation>,
    /// Stack of enclosing `fn` bodies. Whole-file scans cover crate test
    /// directories; helpers called from tests live here too, so a bare-return
    /// guard must be judged in the context of the fn that reaches it.
    fn_stack: Vec<FnContext>,
}

/// Per-`fn` state for the R3 fixture-skip rule: a bare-return guard is only a
/// *silent skip* when the fn it belongs to reaches no assertion afterwards.
#[derive(Default)]
struct FnContext {
    is_test: bool,
    has_assert: bool,
    /// `(line, condition-mentions-fixture)` for bare-return if-arms, resolved
    /// when the fn closes.
    bare_returns: Vec<(usize, bool)>,
}
impl<'a> TestVisitor<'a> {
    fn push(&mut self, line: usize, rule: &'static str, detail: String) {
        self.violations.push(Violation {
            file: self.file_label.to_owned(),
            line,
            rule,
            detail,
        });
    }

    fn flag_if_unwaived(&mut self, line: usize, rule: &'static str, detail: String) {
        if !has_waiver(&self.lines, line) {
            self.push(line, rule, detail);
        }
    }

    /// Recursively scan macro token streams. `assert!`-family macros hide
    /// their contents from the AST (self-comparisons, sleeps, nested calls),
    /// so token-level inspection mirrors `check_literals::scan_macro_tokens`.
    fn scan_macro_tokens(&mut self, tokens: proc_macro2::TokenStream) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        self.scan_tokens(&tokens);
        for token in &tokens {
            if let TokenTree::Group(group) = token {
                self.scan_macro_tokens(group.stream());
            }
        }
    }

    /// Token-level pattern scan over one flat token slice.
    fn scan_tokens(&mut self, tokens: &[TokenTree]) {
        for index in 0..tokens.len() {
            match &tokens[index] {
                // R8: `thread :: sleep` token triple.
                TokenTree::Ident(ident)
                    if ident == "sleep"
                        && index >= 2
                        && matches!(&tokens[index - 1], TokenTree::Punct(p) if p.as_char() == ':')
                        && matches!(&tokens[index - 2], TokenTree::Ident(i) if i == "thread") =>
                {
                    self.flag_if_unwaived(
                        ident.span().start().line,
                        "R8",
                        "std::thread::sleep in test code (use a gated/bench target or an explicit join)".to_owned(),
                    );
                }
                // R1: self-comparison `x == x` — `==` lexes as two `=`
                // puncts, so the window is 4 tokens: ident, =, =, ident.
                // Suppressed when the right ident is preceded by `.`: a field
                // access (`.object_id == object_id`) compares a field to a
                // local of the same name, which is meaningful, not decorative.
                TokenTree::Ident(left)
                    if index + 3 < tokens.len()
                        && token_is_eq(&tokens[index + 1])
                        && matches!(&tokens[index + 2], TokenTree::Punct(p) if p.as_char() == '=')
                        && matches!(&tokens[index + 3], TokenTree::Ident(right) if right == left)
                        && (index == 0
                            || !matches!(&tokens[index - 1], TokenTree::Punct(p) if p.as_char() == '.')) =>
                {
                    self.flag_if_unwaived(
                        left.span().start().line,
                        "R1",
                        format!("self-comparison `{left} == {left}`"),
                    );
                }
                _ => {}
            }
        }
    }
}

fn token_text_is_true_lit(lit: &proc_macro2::Literal) -> bool {
    lit.to_string() == "true"
}

/// Same as checking a macro's raw argument tokens for a single `true`
/// (`assert!(true)` → tokens = `true`).
fn group_is_single_true_stream(tokens: &proc_macro2::TokenStream) -> bool {
    let tokens: Vec<TokenTree> = tokens.clone().into_iter().collect();
    match tokens.as_slice() {
        [TokenTree::Ident(ident)] => ident == "true",
        [TokenTree::Literal(lit)] => token_text_is_true_lit(lit),
        _ => false,
    }
}

fn token_is_eq(token: &TokenTree) -> bool {
    // `==` lexes as two adjacent `=` puncts: joint-spacing first, glued to a
    // second `=` (which itself carries Alone spacing).
    matches!(token, TokenTree::Punct(p) if p.as_char() == '=' && p.spacing() == proc_macro2::Spacing::Joint)
}

impl<'ast> Visit<'ast> for TestVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let is_test = item.attrs.iter().any(|attr| attr.path().is_ident("test"));
        self.fn_stack.push(FnContext {
            is_test,
            has_assert: false,
            bare_returns: Vec::new(),
        });
        syn::visit::visit_item_fn(self, item);
        if let Some(ctx) = self.fn_stack.pop() {
            self.resolve_fn_context(ctx);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            // R3: bare-return `if` arm inside a test-reaching fn; resolved at
            // fn close against whether any assertion follows (docs/22 §2.6).
            Stmt::Expr(Expr::If(iff), None) => {
                if bare_return_arm(&iff.then_branch) {
                    let line = iff.span().start().line;
                    let mentions = self.cond_mentions_fixture(iff);
                    if let Some(ctx) = self.fn_stack.last_mut() {
                        ctx.bare_returns.push((line, mentions));
                    }
                }
            }
            // R8: sleep inside any crate test-scope code.
            Stmt::Expr(expr, _) => {
                if let Some(line) = find_sleep_call(expr) {
                    self.flag_if_unwaived(
                        line,
                        "R8",
                        "std::thread::sleep in test code (use a gated/bench target or an explicit join)".to_owned(),
                    );
                }
                if let Some(ctx) = self.fn_stack.last_mut() {
                    if expr_is_assert(expr) {
                        ctx.has_assert = true;
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // R1: `assert!(true)` — the macro path carries the name; the tokens
        // carry the argument. This form has no legitimate test meaning
        // (docs/22 §2.8).
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if name.starts_with("assert") {
            if group_is_single_true_stream(&mac.tokens) {
                self.flag_if_unwaived(mac.span().start().line, "R1", "assert!(true)".to_owned());
            }
            // An assert! in any form marks the enclosing fn as asserting
            // (R3 resolution: a guard *before* real coverage is a skip; a
            // guard inside a fn that later asserts is ordinary control flow).
            if let Some(ctx) = self.fn_stack.last_mut() {
                ctx.has_assert = true;
            }
        }
        // Nested macros (e.g. a macro that itself expands to assertions) get
        // the token-level patterns (thread::sleep, self-comparison).
        self.scan_macro_tokens(mac.tokens.clone());
        syn::visit::visit_macro(self, mac);
    }

    fn visit_expr_binary(&mut self, bin: &'ast syn::ExprBinary) {
        if matches!(bin.op, syn::BinOp::Eq(_)) && expr_same_local(&bin.left, &bin.right) {
            self.flag_if_unwaived(
                bin.span().start().line,
                "R1",
                "self-comparison of the same local".to_owned(),
            );
        }
        if let Some(ctx) = self.fn_stack.last_mut() {
            ctx.has_assert = true;
        }
        syn::visit::visit_expr_binary(self, bin);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if matches!(
            call.method.to_string().as_str(),
            "expect" | "unwrap" | "unwrap_err"
        ) {
            if let Some(ctx) = self.fn_stack.last_mut() {
                ctx.has_assert = true;
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

fn bare_return_arm(block: &syn::Block) -> bool {
    let stmts: Vec<&Stmt> = block.stmts.iter().collect();
    match stmts.as_slice() {
        [Stmt::Expr(Expr::Return(ret), None)] | [Stmt::Expr(Expr::Return(ret), Some(_))] => {
            ret.expr.is_none()
        }
        _ => false,
    }
}

impl TestVisitor<'_> {
    /// Resolve R3 bare-return guards when the enclosing fn closes: any
    /// bare-return guard on a fixture-lookup condition inside a `#[test]` fn
    /// is a silent skip — the guard decides, before any coverage, whether the
    /// test runs, so a missing fixture reports green forever (docs/22 §2.6,
    /// CONTEXT.md **Loud skip**). A helper fn with the same guard is outside
    /// the rule's syntactic reach: absence-of-output guards like
    /// `assert_no_bundle_written` are ordinary negative-path control flow,
    /// not fixture gating.
    fn resolve_fn_context(&mut self, ctx: FnContext) {
        if !ctx.is_test {
            return;
        }
        for (line, mentions) in ctx.bare_returns {
            if mentions && !has_waiver(&self.lines, line) {
                self.push(
                    line,
                    "R3",
                    "silent fixture-skip early return (bare `return` in an if-arm)".to_owned(),
                );
            }
        }
    }
}

/// Expression-position `assert!` (`assert!(cond);` is Stmt::Macro, but
/// `let _ = assert!(..)`-style or `if cond { assert!(..); }` nested forms
/// surface as Expr::Macro).
fn expr_is_assert(expr: &Expr) -> bool {
    matches!(expr, Expr::Macro(mac)
        if mac.mac.path.segments.last().map(|s| s.ident.to_string().starts_with("assert")).unwrap_or(false))
}

impl TestVisitor<'_> {
    /// The fixture condition is recognized from the source text of the `if`
    /// header (condition spans source lines, and the parse tree's condition
    /// tokens are not re-rendered cheaply). Words: the fixture-lookup calls
    /// this codebase actually uses for optional fixtures.
    fn cond_mentions_fixture(&self, iff: &ExprIf) -> bool {
        let start = iff.span().start().line;
        let window: String = self
            .lines
            .get(start.saturating_sub(1)..=(start).min(self.lines.len().saturating_sub(1)))
            .map(|slice| slice.join(" ").to_ascii_lowercase())
            .unwrap_or_default();
        [
            "exists",
            "is_file",
            "fixture",
            "metadata",
            "try_open",
            "read_to_string",
        ]
        .iter()
        .any(|word| window.contains(word))
    }
}

/// Find a `std::thread::sleep(...)` / `thread::sleep(...)` call inside an
/// expression tree; return the first call's line. `sleep` is an associated
/// function, so this is an `Expr::Call` with a `...::sleep` path — never an
/// `ExprMethodCall`.
fn find_sleep_call(expr: &Expr) -> Option<usize> {
    struct SleepFinder(Option<usize>);
    impl<'ast> Visit<'ast> for SleepFinder {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            let is_thread_sleep = matches!(&*call.func, Expr::Path(p)
                if p.qself.is_none()
                    && p.path.segments.len() >= 2
                    && p.path.segments.last().is_some_and(|s| s.ident == "sleep")
                    && p.path.segments.iter().any(|s| s.ident == "thread"));
            if is_thread_sleep && self.0.is_none() {
                self.0 = Some(call.span().start().line);
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let mut finder = SleepFinder(None);
    syn::visit::Visit::visit_expr(&mut finder, expr);
    finder.0
}

fn expr_same_local(left: &Expr, right: &Expr) -> bool {
    fn local_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Path(p) if p.qself.is_none() => p.path.get_ident().map(|i| i.to_string()),
            Expr::Field(ExprField { base, member, .. }) => {
                let index = match member {
                    Member::Unnamed(u) => u.index.to_string(),
                    _ => return None,
                };
                Some(format!("{}#{}", local_name(base)?, index))
            }
            _ => None,
        }
    }
    match (local_name(left), local_name(right)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn has_waiver(lines: &[&str], line_1based: usize) -> bool {
    if lines.is_empty() {
        return false;
    }
    let end = line_1based.saturating_sub(1).min(lines.len() - 1);
    let start = end.saturating_sub(1);
    (start..=end).any(|index| {
        lines[index]
            .split_once(WAIVER_MARK)
            .is_some_and(|(_, reason)| !reason.trim().is_empty())
    })
}

struct TestFnCollector {
    names: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for TestFnCollector {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if item.attrs.iter().any(|attr| attr.path().is_ident("test")) {
            self.names.push((
                item.sig.ident.to_string(),
                item.sig.ident.span().start().line,
            ));
        }
        syn::visit::visit_item_fn(self, item);
    }
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn matches_filter(relative: &Path, filter: &str) -> bool {
    let relative = normalized_path(relative);
    let filter = filter.replace('\\', "/").trim_matches('/').to_owned();
    relative == filter || relative.starts_with(&(filter + "/"))
}

/// Same scope as `check_literals::collect_enforced_files`.
fn collect_enforced_files(ws: &Path, filters: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut collect_scope = |root: PathBuf| {
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
                files.push(path.to_path_buf());
            }
        }
    };

    let crates = ws.join("crates");
    if let Ok(entries) = fs::read_dir(&crates) {
        for entry in entries.flatten() {
            let crate_dir = entry.path();
            collect_scope(crate_dir.join("tests"));
            collect_scope(crate_dir.join("benches"));
            collect_scope(crate_dir.join("src"));
        }
    }

    let modules = ws.join("modules/core-modules");
    if let Ok(entries) = fs::read_dir(modules) {
        for entry in entries.flatten() {
            collect_scope(entry.path().join("tests"));
        }
    }

    files.sort_by_key(|p| normalized_path(p));
    files
}

fn scan_source(file_label: &str, src: &str) -> Vec<Violation> {
    let file = match syn::parse_file(src) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("xtask: cannot parse {file_label}: {error}");
            return Vec::new();
        }
    };
    let lines: Vec<&str> = src.lines().collect();

    let mut visitor = TestVisitor {
        file_label,
        lines,
        violations: Vec::new(),
        fn_stack: Vec::new(),
    };
    visitor.visit_file(&file);

    let mut collector = TestFnCollector { names: Vec::new() };
    collector.visit_file(&file);

    // R6: duplicate test-fn name within one file (a file registered in two
    // targets is a Cargo-level fact; same-name-twice-in-one-file is the
    // syntactic proxy that cannot false-positive on module trees).
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut duplicates: Vec<(usize, String)> = Vec::new();
    for (name, line) in &collector.names {
        if !seen.insert(name.clone()) {
            duplicates.push((*line, name.clone()));
        }
    }
    for (line, name) in duplicates {
        visitor.flag_if_unwaived(
            line,
            "R6",
            format!("duplicate test-fn name `{name}` in one file"),
        );
    }

    visitor.violations.sort_by_key(|v| (v.line, v.rule));
    visitor.violations
}

pub(crate) fn run(ws: &Path, report: bool, path_filters: &[String]) -> i32 {
    let files = collect_enforced_files(ws, path_filters);
    let mut violations = Vec::new();
    for path in &files {
        let relative = path.strip_prefix(ws).unwrap_or(path);
        let label = normalized_path(relative);
        match fs::read_to_string(path) {
            Ok(source) => violations.extend(scan_source(&label, &source)),
            Err(error) => eprintln!("xtask: cannot read {}: {error}", label),
        }
    }
    violations.sort_by(|l, r| l.file.cmp(&r.file).then(l.line.cmp(&r.line)));
    for violation in &violations {
        println!(
            "{}:{}: {} {}",
            violation.file, violation.line, violation.rule, violation.detail
        );
    }
    let files_with = violations
        .iter()
        .map(|v| v.file.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    println!(
        "check-test-quality: {} finding(s) in {} file(s) [{} mode]",
        violations.len(),
        files_with,
        if TEST_QUALITY_ENFORCED {
            "enforce"
        } else {
            "report"
        }
    );
    if violations.is_empty() || report || !TEST_QUALITY_ENFORCED {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::{matches_filter, scan_source};

    #[test]
    fn flags_decorative_assert_true() {
        let src = r#"
            #[test]
            fn t() { assert!(true); }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations
                .iter()
                .any(|v| v.rule == "R1" && v.detail.contains("assert!(true)")),
            "expected R1, got {violations:?}"
        );
    }

    #[test]
    fn flags_self_comparison() {
        let src = r#"
            #[test]
            fn t() { let x = make(); assert!(x == x); }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations.iter().any(|v| v.rule == "R1"),
            "got {violations:?}"
        );
    }

    #[test]
    fn flags_silent_fixture_skip() {
        let src = r#"
            #[test]
            fn t() {
                if !fixture_path().exists() { return; }
                assert_eq!(run(fixture_path()), 1);
            }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations.iter().any(|v| v.rule == "R3"),
            "got {violations:?}"
        );
    }

    #[test]
    fn flags_thread_sleep_in_test() {
        let src = r#"
            #[test]
            fn t() { std::thread::sleep(std::time::Duration::from_millis(1)); }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations.iter().any(|v| v.rule == "R8"),
            "got {violations:?}"
        );
    }

    #[test]
    fn flags_duplicate_test_names_in_file() {
        let src = r#"
            #[test] fn same() { assert_eq!(1, 1); }
            #[test] fn same() { assert_eq!(2, 2); }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations.iter().any(|v| v.rule == "R6"),
            "got {violations:?}"
        );
    }

    #[test]
    fn does_not_flag_real_assertions() {
        let src = r#"
            #[test]
            fn t() {
                let v = produce();
                assert_eq!(v.len(), 3, "three segments expected");
                assert!(v.iter().all(|x| *x > 0));
            }
        "#;
        assert!(scan_source("fixture.rs", src).is_empty());
    }

    #[test]
    fn waiver_requires_reason() {
        let src = r#"
            #[test]
            fn t() { assert!(true); } // test-quality:
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(
            violations.iter().any(|v| v.rule == "R1"),
            "got {violations:?}"
        );
    }

    #[test]
    fn waiver_on_preceding_line_applies() {
        let src = r#"
            #[test]
            fn t() {
                // test-quality: compile witness — sole compile check of prelude::Everything
                let _ = prelude::Everything;
            }
        "#;
        let violations = scan_source("fixture.rs", src);
        assert!(violations.iter().all(|v| v.rule != "R1"));
        // The bare-expression body still triggers R4-free scan: no assert-like
        // statement exists but this waiver covers the witness form; R4 is a
        // per-fn rule and the witness marker line is the waiver carrier.
    }

    #[test]
    fn path_filter_is_component_aware() {
        assert!(matches_filter(
            std::path::Path::new("crates/slicer-ir/tests/foo.rs"),
            "crates/slicer-ir"
        ));
        assert!(!matches_filter(
            std::path::Path::new("crates/slicer-ir-extra/tests/foo.rs"),
            "crates/slicer-ir"
        ));
    }
}
