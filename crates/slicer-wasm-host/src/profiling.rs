//! Host-side half of fuel-based module profiling (ADR-0055).
//!
//! # What a mark is
//!
//! A guest cannot read a clock (it builds for `wasm32-unknown-unknown`, which
//! has no WASI) and cannot read its own fuel. So the guest only reports *where*
//! it is, via the WIT `profiling` interface's `profile-mark(scope, edge)`, and
//! the host attaches *what it cost* at that instant: consumed fuel plus a
//! wall-clock reading. Because a mark is a host call, it burns no fuel and
//! therefore cannot pollute the measurement it is taking.
//!
//! # Why fuel is recorded as *consumed*, not *remaining*
//!
//! A store is handed [`crate::instance::FUEL_BUDGET`] up front and wasmtime
//! counts downward. Storing `budget - remaining` makes [`ProfileMark::fuel`]
//! monotonically non-decreasing across a call, so folding is plain subtraction
//! and no consumer has to know what the budget was.
//!
//! # The name registry is process-global on purpose
//!
//! Every dispatch call builds a fresh `wasmtime::Store` and instantiates the
//! component again, so anything the guest cached — including scope ids — is
//! gone by the next call. Ids therefore cannot live in the per-call context:
//! [`register_scope`] interns into a process-global table so re-registering a
//! name always yields the same id, and a report can resolve an id it saw on
//! layer 1 while reading a mark from layer 240.
//!
//! Ids at or above [`slicer_core::profile::ScopeId::USER_BASE`] are minted
//! here; ids below it belong to `slicer-core` and resolve through
//! [`slicer_core::profile::ScopeId::name`] instead.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use slicer_core::profile::ScopeId;

/// Which side of a scope a [`ProfileMark`] sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkEdge {
    /// The scope was entered, before its work began.
    Enter,
    /// The scope was left, after its work completed.
    Exit,
}

/// One scope transition observed by the host during a single guest call.
///
/// `fuel` and `wall_ns` are both *cumulative since the start of this dispatch
/// call*, which is what makes a pair of marks subtractable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileMark {
    /// Scope id, as minted by [`register_scope`] or reserved by `slicer-core`.
    pub scope: u32,
    /// Whether this mark opens or closes the scope.
    pub edge: MarkEdge,
    /// Fuel consumed by the guest between the start of the call and this mark.
    ///
    /// Zero on every mark when profiling is off — fuel metering is not enabled
    /// on the store in that case, so there is nothing to sample.
    pub fuel: u64,
    /// Nanoseconds elapsed between the start of the call and this mark.
    ///
    /// ADR-0055: wall-clock under `--profile` is inflated by the mark host calls
    /// themselves. Fuel ratios are unaffected; wall-clock here is indicative.
    pub wall_ns: u64,
}

/// Per-scope cost folded out of a mark stream by [`fold_marks`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopeTotals {
    /// Scope id these totals belong to.
    pub scope: u32,
    /// Number of completed (entered *and* exited) activations.
    pub calls: u64,
    /// Fuel spent inside this scope including everything nested under it.
    ///
    /// Recursive activations are counted once, at the outermost level, so a
    /// self-recursive scope does not multiply-count its own body.
    pub total_fuel: u64,
    /// Fuel spent inside this scope *excluding* time attributed to nested
    /// scopes. Summing `self_fuel` over all scopes never exceeds the call total.
    pub self_fuel: u64,
    /// Wall-clock nanoseconds, inclusive of nested scopes. See the caveat on
    /// [`ProfileMark::wall_ns`].
    pub total_wall_ns: u64,
    /// Wall-clock nanoseconds excluding nested scopes.
    pub self_wall_ns: u64,
}

/// One open scope on the fold's stack.
struct OpenScope {
    scope: u32,
    fuel_at_enter: u64,
    wall_at_enter: u64,
    /// Fuel consumed by scopes opened and closed directly under this one.
    child_fuel: u64,
    /// Wall-clock consumed by direct children.
    child_wall_ns: u64,
    /// `true` when an ancestor on the stack carries the same scope id. The
    /// activation is still folded for `self_*`, but its `total_*` is dropped so
    /// a recursive scope's inclusive figure is not counted once per frame.
    is_recursive: bool,
}

/// Folds a raw mark stream into per-scope self and total cost.
///
/// This is the per-call scope stack: a well-formed stream is a bracket
/// sequence, so an `enter` pushes and a matching `exit` pops, crediting the
/// popped activation's cost to its parent's child total.
///
/// Malformed streams are tolerated rather than rejected, because a mark stream
/// arrives from a guest and a panic here would take down a slice over a
/// diagnostic:
/// - An `exit` with no matching `enter` (or one that does not match the top of
///   the stack) is **ignored**.
/// - An `enter` never closed — the guest trapped, or returned mid-scope — is
///   **dropped**, contributing nothing. Charging it to the end of the call
///   would invent cost that no scope was measured over.
///
/// Output is sorted by scope id so two runs of the same workload produce
/// byte-identical reports.
#[must_use]
pub fn fold_marks(marks: &[ProfileMark]) -> Vec<ScopeTotals> {
    let mut totals: HashMap<u32, ScopeTotals> = HashMap::new();
    let mut stack: Vec<OpenScope> = Vec::new();

    for mark in marks {
        match mark.edge {
            MarkEdge::Enter => {
                let is_recursive = stack.iter().any(|open| open.scope == mark.scope);
                stack.push(OpenScope {
                    scope: mark.scope,
                    fuel_at_enter: mark.fuel,
                    wall_at_enter: mark.wall_ns,
                    child_fuel: 0,
                    child_wall_ns: 0,
                    is_recursive,
                });
            }
            MarkEdge::Exit => {
                // Only a matching top-of-stack closes a scope; anything else is
                // a malformed stream and is discarded.
                if stack.last().map(|open| open.scope) != Some(mark.scope) {
                    continue;
                }
                let open = stack.pop().expect("checked non-empty above");

                let total_fuel = mark.fuel.saturating_sub(open.fuel_at_enter);
                let total_wall = mark.wall_ns.saturating_sub(open.wall_at_enter);
                let self_fuel = total_fuel.saturating_sub(open.child_fuel);
                let self_wall = total_wall.saturating_sub(open.child_wall_ns);

                let entry = totals.entry(open.scope).or_insert(ScopeTotals {
                    scope: open.scope,
                    ..ScopeTotals::default()
                });
                entry.calls += 1;
                entry.self_fuel += self_fuel;
                entry.self_wall_ns += self_wall;
                if !open.is_recursive {
                    entry.total_fuel += total_fuel;
                    entry.total_wall_ns += total_wall;
                }

                if let Some(parent) = stack.last_mut() {
                    parent.child_fuel += total_fuel;
                    parent.child_wall_ns += total_wall;
                }
            }
        }
    }

    let mut out: Vec<ScopeTotals> = totals.into_values().collect();
    out.sort_by_key(|t| t.scope);
    out
}

/// Process-global, append-only name↔id table for guest-minted scopes.
struct ScopeRegistry {
    /// Names in id order; `names[i]` has id `USER_BASE + i`.
    names: Vec<String>,
    ids: HashMap<String, u32>,
}

fn registry() -> &'static RwLock<ScopeRegistry> {
    static REGISTRY: OnceLock<RwLock<ScopeRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        RwLock::new(ScopeRegistry {
            names: Vec::new(),
            ids: HashMap::new(),
        })
    })
}

/// Interns `name` and returns its scope id, allocating one on first sight.
///
/// Backs the WIT `profiling.profile-register`. Idempotent: the same name always
/// returns the same id for the life of the process. Ids start at
/// [`ScopeId::USER_BASE`] so they can never collide with a `slicer-core` scope.
///
/// A name that is already a core scope (e.g. `polygon_ops::offset`) resolves to
/// the reserved core id rather than minting a duplicate, so a guest sink that
/// registers `ScopeId::CORE_SCOPES` by name lands on the same ids the host uses.
#[must_use]
pub fn register_scope(name: &str) -> u32 {
    for core in ScopeId::CORE_SCOPES {
        if core.name() == Some(name) {
            return core.raw();
        }
    }

    // Fast path: an already-interned name needs only a read lock.
    if let Some(&id) = registry()
        .read()
        .expect("scope registry poisoned")
        .ids
        .get(name)
    {
        return id;
    }

    let mut reg = registry().write().expect("scope registry poisoned");
    // Re-check: another thread may have interned between the two locks.
    if let Some(&id) = reg.ids.get(name) {
        return id;
    }
    let id = ScopeId::USER_BASE + reg.names.len() as u32;
    reg.names.push(name.to_string());
    reg.ids.insert(name.to_string(), id);
    id
}

/// Resolves a scope id to its human-readable name, or `None` if nothing has
/// claimed that id.
///
/// Checks `slicer-core`'s reserved ids first, then the [`register_scope`] table.
#[must_use]
pub fn scope_name(id: u32) -> Option<String> {
    if let Some(name) = ScopeId::new(id).name() {
        return Some(name.to_string());
    }
    let index = id.checked_sub(ScopeId::USER_BASE)? as usize;
    registry()
        .read()
        .expect("scope registry poisoned")
        .names
        .get(index)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter(scope: u32, fuel: u64, wall_ns: u64) -> ProfileMark {
        ProfileMark {
            scope,
            edge: MarkEdge::Enter,
            fuel,
            wall_ns,
        }
    }

    fn exit(scope: u32, fuel: u64, wall_ns: u64) -> ProfileMark {
        ProfileMark {
            scope,
            edge: MarkEdge::Exit,
            fuel,
            wall_ns,
        }
    }

    #[test]
    fn empty_stream_folds_to_nothing() {
        assert!(fold_marks(&[]).is_empty());
    }

    #[test]
    fn one_flat_scope_has_self_equal_to_total() {
        let folded = fold_marks(&[enter(7, 100, 10), exit(7, 400, 40)]);
        assert_eq!(
            folded,
            vec![
                // exhaustive: scope-folding assertion specifies every total field
                ScopeTotals {
                    scope: 7,
                    calls: 1,
                    total_fuel: 300,
                    self_fuel: 300,
                    total_wall_ns: 30,
                    self_wall_ns: 30,
                }
            ]
        );
    }

    /// The load-bearing case: an inner scope's cost must be subtracted from the
    /// outer scope's `self_fuel` but stay in its `total_fuel`.
    #[test]
    fn nested_scopes_split_self_from_total() {
        let marks = [
            enter(1, 0, 0),
            enter(2, 100, 10),
            exit(2, 400, 40),
            exit(1, 500, 50),
        ];
        let folded = fold_marks(&marks);
        assert_eq!(
            folded,
            vec![
                // exhaustive: nested scope assertion specifies every total field
                ScopeTotals {
                    scope: 1,
                    calls: 1,
                    total_fuel: 500,
                    // 500 total − 300 spent in scope 2.
                    self_fuel: 200,
                    total_wall_ns: 50,
                    self_wall_ns: 20,
                },
                // exhaustive: nested child assertion specifies every total field
                ScopeTotals {
                    scope: 2,
                    calls: 1,
                    total_fuel: 300,
                    self_fuel: 300,
                    total_wall_ns: 30,
                    self_wall_ns: 30,
                },
            ]
        );
    }

    #[test]
    fn sibling_scopes_both_subtract_from_the_parent() {
        let marks = [
            enter(1, 0, 0),
            enter(2, 10, 1),
            exit(2, 40, 4),
            enter(3, 50, 5),
            exit(3, 110, 11),
            exit(1, 130, 13),
        ];
        let folded = fold_marks(&marks);
        // Parent self = 130 − (30 + 60) = 40.
        assert_eq!(folded[0].scope, 1);
        assert_eq!(folded[0].total_fuel, 130);
        assert_eq!(folded[0].self_fuel, 40);
        assert_eq!(folded[0].self_wall_ns, 4);
        assert_eq!(folded[1].self_fuel, 30);
        assert_eq!(folded[2].self_fuel, 60);
    }

    #[test]
    fn three_deep_nesting_credits_only_the_direct_parent() {
        let marks = [
            enter(1, 0, 0),
            enter(2, 100, 1),
            enter(3, 200, 2),
            exit(3, 500, 5),
            exit(2, 600, 6),
            exit(1, 700, 7),
        ];
        let folded = fold_marks(&marks);
        // 1: total 700, one direct child (2) costing 500 → self 200.
        assert_eq!((folded[0].total_fuel, folded[0].self_fuel), (700, 200));
        // 2: total 500, one direct child (3) costing 300 → self 200.
        assert_eq!((folded[1].total_fuel, folded[1].self_fuel), (500, 200));
        // 3: leaf.
        assert_eq!((folded[2].total_fuel, folded[2].self_fuel), (300, 300));
        // Self columns partition the outermost total exactly.
        assert_eq!(
            folded.iter().map(|t| t.self_fuel).sum::<u64>(),
            folded[0].total_fuel
        );
    }

    #[test]
    fn repeated_activations_accumulate_calls_and_cost() {
        let marks = [
            enter(4, 0, 0),
            exit(4, 10, 1),
            enter(4, 30, 3),
            exit(4, 55, 6),
        ];
        let folded = fold_marks(&marks);
        assert_eq!(folded[0].calls, 2);
        assert_eq!(folded[0].total_fuel, 10 + 25);
        assert_eq!(folded[0].self_fuel, 10 + 25);
        assert_eq!(folded[0].total_wall_ns, 1 + 3);
    }

    /// A self-recursive scope must not report its own body once per stack frame
    /// in `total_fuel`, or a recursive geometry routine would appear to cost
    /// several times the whole call.
    #[test]
    fn recursion_counts_total_once_but_self_every_time() {
        let marks = [
            enter(9, 0, 0),
            enter(9, 100, 1),
            exit(9, 300, 3),
            exit(9, 400, 4),
        ];
        let folded = fold_marks(&marks);
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].calls, 2);
        // Outer activation only: 400. The inner 200 is nested inside it.
        assert_eq!(folded[0].total_fuel, 400);
        // Outer self = 400 − 200 = 200; inner self = 200. Sum = 400.
        assert_eq!(folded[0].self_fuel, 400 - 200 + 200);
        assert!(folded[0].self_fuel <= folded[0].total_fuel);
    }

    #[test]
    fn unmatched_exit_is_ignored() {
        let folded = fold_marks(&[exit(1, 50, 5), enter(1, 100, 10), exit(1, 200, 20)]);
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].calls, 1);
        assert_eq!(folded[0].total_fuel, 100);
    }

    #[test]
    fn crossed_brackets_do_not_close_the_wrong_scope() {
        // enter 1, enter 2, exit 1 — the exit does not match the top of stack.
        let folded = fold_marks(&[
            enter(1, 0, 0),
            enter(2, 10, 1),
            exit(1, 20, 2),
            exit(2, 30, 3),
            exit(1, 40, 4),
        ]);
        // Scope 2 closes normally; scope 1 closes on the *second* exit(1).
        assert_eq!(folded.len(), 2);
        assert_eq!(folded[0].scope, 1);
        assert_eq!(folded[0].total_fuel, 40);
        assert_eq!(folded[0].self_fuel, 40 - 20);
        assert_eq!(folded[1].scope, 2);
        assert_eq!(folded[1].total_fuel, 20);
    }

    #[test]
    fn never_closed_enter_contributes_nothing() {
        let folded = fold_marks(&[enter(1, 0, 0), enter(2, 10, 1), exit(2, 40, 4)]);
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].scope, 2);
        assert_eq!(folded[0].total_fuel, 30);
    }

    #[test]
    fn output_is_sorted_by_scope_id() {
        let folded = fold_marks(&[
            enter(30, 0, 0),
            exit(30, 1, 1),
            enter(2, 2, 2),
            exit(2, 3, 3),
            enter(11, 4, 4),
            exit(11, 5, 5),
        ]);
        assert_eq!(
            folded.iter().map(|t| t.scope).collect::<Vec<_>>(),
            vec![2, 11, 30]
        );
    }

    #[test]
    fn registering_is_idempotent_and_stays_above_user_base() {
        let a = register_scope("test::alpha");
        let b = register_scope("test::beta");
        assert_ne!(a, b);
        assert_eq!(register_scope("test::alpha"), a);
        assert!(a >= ScopeId::USER_BASE);
        assert!(b >= ScopeId::USER_BASE);
        assert_eq!(scope_name(a).as_deref(), Some("test::alpha"));
        assert_eq!(scope_name(b).as_deref(), Some("test::beta"));
    }

    #[test]
    fn core_scope_names_resolve_to_reserved_ids_without_minting() {
        assert_eq!(
            register_scope("polygon_ops::offset"),
            ScopeId::OFFSET.raw(),
            "a guest registering a core scope by name must land on the core id"
        );
        assert_eq!(
            scope_name(ScopeId::CLIP_POLYGONS.raw()).as_deref(),
            Some("polygon_ops::clip_polygons")
        );
    }

    #[test]
    fn unknown_ids_have_no_name() {
        assert_eq!(scope_name(ScopeId::USER_BASE - 1), None);
        assert_eq!(scope_name(u32::MAX), None);
    }
}
