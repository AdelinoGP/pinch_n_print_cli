//! Run-wide aggregation and presentation for fuel-based module profiling
//! (ADR-0055).
//!
//! `slicer-wasm-host`'s [`profiling`](slicer_wasm_host::profiling) module owns
//! the *per-call* half of the contract: it records [`ProfileMark`]s and folds
//! one call's mark stream into [`ScopeTotals`]. This module owns the *run-wide*
//! half — merging every call's fold into per-(module, scope) totals, resolving
//! scope ids to names, ranking, and rendering.
//!
//! # Two units, never mixed in one ranking
//!
//! A guest module is fuel-metered: its cost is a deterministic count of
//! executed wasm instructions, identical across runs and machines. A host
//! built-in (`host:slice`, `host:shell_classification`,
//! `host:paint_segmentation`) is native `slicer-core` code calling the same
//! marked `polygon_ops` primitives — same vocabulary, but wasmtime does not
//! meter native code, so the only signal available for it is wall-clock.
//!
//! Ranking those two together would produce a table whose rows are not
//! comparable, so [`ProfileModuleRow::unit`] tags every row and
//! [`format_profile_summary`] renders them as two separately-normalised
//! sections with the unit stated in each heading.
//!
//! # The observer-effect contract
//!
//! Marks are host calls, and host calls burn no fuel — so **fuel ratios are
//! exact**, unaffected by the act of measuring. Wall-clock is not: every mark
//! costs a guest→host transition, and the marked primitives sit in hot loops,
//! so wall-clock under `--profile` is inflated. It is reported because a
//! millisecond figure is what turns "this got 20% cheaper in fuel" into a
//! decision, but it is **indicative only**; absolute timings come from a
//! profiling-off `--instrument-stderr` run.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use slicer_core::profile::ScopeId;
use slicer_wasm_host::profiling::{fold_marks, scope_name, MarkEdge, ProfileMark, ScopeTotals};

/// Module id used for native `polygon_ops` activity that no host built-in
/// bracket claimed — e.g. a marked primitive reached from the per-layer tier
/// rather than from a prepass built-in.
pub const UNATTRIBUTED_NATIVE_MODULE: &str = "host:native";

/// Label for the share of a module's cost that no marked scope accounted for.
///
/// Not a scope: it is `module total − Σ scope self`, i.e. the guest's own code
/// plus anything it called that carries no marks.
pub const MODULE_SELF_LABEL: &str = "<module self>";

/// Which quantity a [`ProfileModuleRow`]'s cost is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileUnit {
    /// wasmtime fuel — executed guest instructions. Deterministic across runs
    /// and machines, and unaffected by mark overhead.
    Fuel,
    /// Wall-clock nanoseconds. The only signal available for native host
    /// built-ins, and inflated by mark overhead (see the module docs).
    WallNs,
}

impl ProfileUnit {
    /// Short human label used in section headings and row tags.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Fuel => "fuel",
            Self::WallNs => "wall",
        }
    }
}

/// One scope's run-wide cost inside one module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileScopeRow {
    /// Human-readable scope name (e.g. `polygon_ops::offset2_ex`), or
    /// `scope#<id>` when the id was never registered.
    pub scope: String,
    /// Raw scope id, retained so a consumer can correlate across runs without
    /// depending on the name table.
    pub scope_id: u32,
    /// Completed activations summed over every call of the owning module.
    pub calls: u64,
    /// Fuel spent in this scope excluding nested scopes. Always `0` for a
    /// [`ProfileUnit::WallNs`] module.
    pub self_fuel: u64,
    /// Fuel spent in this scope including nested scopes.
    pub total_fuel: u64,
    /// Wall-clock nanoseconds excluding nested scopes. Indicative — see the
    /// module docs.
    pub self_wall_ns: u64,
    /// Wall-clock nanoseconds including nested scopes.
    pub total_wall_ns: u64,
}

/// One module's run-wide cost, with its scope breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileModuleRow {
    /// Module id, or the `host:*` built-in id for native rows.
    pub module_id: String,
    /// Whether this row's ranking quantity is fuel or wall-clock.
    pub unit: ProfileUnit,
    /// Number of dispatch calls (guest) or built-in activations (native)
    /// that contributed.
    pub calls: u64,
    /// Total fuel consumed across every call. `0` for a native row.
    pub total_fuel: u64,
    /// Total wall-clock nanoseconds covered by marks across every call.
    pub total_wall_ns: u64,
    /// `total_fuel` minus the sum of every scope's `self_fuel` — the part of
    /// the module that ran outside any marked scope.
    pub self_fuel: u64,
    /// `total_wall_ns` minus the sum of every scope's `self_wall_ns`.
    pub self_wall_ns: u64,
    /// Per-scope breakdown, ranked by this row's unit, most expensive first.
    pub scopes: Vec<ProfileScopeRow>,
}

impl ProfileModuleRow {
    /// The quantity this row is ranked by, in its own unit.
    #[must_use]
    pub fn rank_cost(&self) -> u64 {
        match self.unit {
            ProfileUnit::Fuel => self.total_fuel,
            ProfileUnit::WallNs => self.total_wall_ns,
        }
    }
}

/// Run-wide profiling result: the payload of the `profile_summary` event and
/// the input to [`format_profile_summary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSummary {
    /// Sum of `total_fuel` over every [`ProfileUnit::Fuel`] module. The
    /// denominator for every fuel percentage in the report.
    pub fuel_total: u64,
    /// Sum of `total_wall_ns` over every [`ProfileUnit::WallNs`] module. The
    /// denominator for every wall-clock percentage.
    pub wall_total_ns: u64,
    /// Module rows: fuel-metered modules first (by fuel, descending), then
    /// native rows (by wall-clock, descending). Ties break on `module_id` so
    /// two runs of the same workload render identically.
    pub modules: Vec<ProfileModuleRow>,
}

impl ProfileSummary {
    /// `true` when nothing was recorded — no guest emitted a mark and no host
    /// built-in ran marked native code.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

/// Per-call scope detail attached to a `module_complete` event under
/// `--profile-verbose`.
///
/// Aggregation is the default because a 0.2 mm benchy emits ~2,900
/// `module_complete` events, and hanging a scope array off each of them would
/// force every consumer to write a reducer before reading anything. This is the
/// opt-in escape hatch for the opposite question — *which single call* was
/// pathological — so it carries one call's fold, not the run's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileCallDetail {
    /// Total fuel this one dispatch call consumed.
    pub call_fuel: u64,
    /// This call's scope fold, ranked by `self_fuel` descending.
    pub scopes: Vec<ProfileScopeRow>,
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

/// Per-module accumulator. Folded totals only — raw marks are discarded after
/// each call, so memory stays proportional to (modules × distinct scopes)
/// rather than to the number of marks, which on a real slice runs to millions.
#[derive(Debug, Clone, Default)]
struct ModuleAcc {
    unit: Option<ProfileUnit>,
    calls: u64,
    total_fuel: u64,
    total_wall_ns: u64,
    scopes: BTreeMap<u32, ScopeTotals>,
}

impl ModuleAcc {
    fn merge_scopes(&mut self, folded: &[ScopeTotals]) {
        for row in folded {
            let entry = self.scopes.entry(row.scope).or_insert(ScopeTotals {
                scope: row.scope,
                ..ScopeTotals::default()
            });
            entry.calls += row.calls;
            entry.self_fuel += row.self_fuel;
            entry.total_fuel += row.total_fuel;
            entry.self_wall_ns += row.self_wall_ns;
            entry.total_wall_ns += row.total_wall_ns;
        }
    }
}

/// Thread-safe run-wide accumulator, shared by every executor drop-site.
///
/// One instance per `run_slice` call. Folding happens on the calling thread
/// (rayon workers included) and only the folded result crosses the lock, so the
/// critical section is proportional to the number of *distinct scopes* in a
/// call, not to the number of marks.
#[derive(Debug, Default)]
pub struct ProfileAggregator {
    modules: Mutex<BTreeMap<String, ModuleAcc>>,
}

impl ProfileAggregator {
    /// Construct an empty aggregator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one guest dispatch call.
    ///
    /// A call with no marks and no fuel is dropped rather than counted: that is
    /// what every call looks like when profiling is off, and counting it would
    /// make the summary non-empty on a run that measured nothing.
    pub fn record_call(&self, module_id: &str, marks: &[ProfileMark], call_fuel: u64) {
        if marks.is_empty() && call_fuel == 0 {
            return;
        }
        let folded = fold_marks(marks);
        // Marks carry wall-clock cumulative since the start of the call, so the
        // last one is how much of the call the marks actually spanned.
        let call_wall_ns = marks.last().map_or(0, |m| m.wall_ns);

        let mut guard = self.modules.lock().expect("profile aggregator poisoned");
        let acc = guard.entry(module_id.to_string()).or_default();
        acc.unit = Some(ProfileUnit::Fuel);
        acc.calls += 1;
        acc.total_fuel += call_fuel;
        acc.total_wall_ns += call_wall_ns;
        acc.merge_scopes(&folded);
    }

    /// Record one native (host built-in) activation, already folded.
    ///
    /// `wall_ns` is the activation's own span; there is no fuel because native
    /// code is not metered.
    pub fn record_native(&self, module_id: &str, folded: &[ScopeTotals], calls: u64, wall_ns: u64) {
        if folded.is_empty() && wall_ns == 0 {
            return;
        }
        let mut guard = self.modules.lock().expect("profile aggregator poisoned");
        let acc = guard.entry(module_id.to_string()).or_default();
        acc.unit = Some(ProfileUnit::WallNs);
        acc.calls += calls;
        acc.total_wall_ns += wall_ns;
        acc.merge_scopes(folded);
    }

    /// `true` when nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules
            .lock()
            .expect("profile aggregator poisoned")
            .is_empty()
    }

    /// Resolve names, compute self-shares, and rank into a [`ProfileSummary`].
    #[must_use]
    pub fn finish(&self) -> ProfileSummary {
        let guard = self.modules.lock().expect("profile aggregator poisoned");
        let mut modules: Vec<ProfileModuleRow> = guard
            .iter()
            .map(|(module_id, acc)| build_module_row(module_id, acc))
            .collect();
        drop(guard);

        let fuel_total = modules
            .iter()
            .filter(|m| m.unit == ProfileUnit::Fuel)
            .map(|m| m.total_fuel)
            .sum();
        let wall_total_ns = modules
            .iter()
            .filter(|m| m.unit == ProfileUnit::WallNs)
            .map(|m| m.total_wall_ns)
            .sum();

        // Fuel rows first (they are the primary signal), each block ranked by
        // its own unit descending, ties broken on id for run-to-run stability.
        modules.sort_by(|a, b| {
            a.unit
                .cmp(&b.unit)
                .then_with(|| b.rank_cost().cmp(&a.rank_cost()))
                .then_with(|| a.module_id.cmp(&b.module_id))
        });

        ProfileSummary {
            fuel_total,
            wall_total_ns,
            modules,
        }
    }
}

fn build_module_row(module_id: &str, acc: &ModuleAcc) -> ProfileModuleRow {
    let unit = acc.unit.unwrap_or(ProfileUnit::Fuel);
    let mut scopes: Vec<ProfileScopeRow> = acc
        .scopes
        .values()
        .map(|t| ProfileScopeRow {
            scope: resolve_scope_name(t.scope),
            scope_id: t.scope,
            calls: t.calls,
            self_fuel: t.self_fuel,
            total_fuel: t.total_fuel,
            self_wall_ns: t.self_wall_ns,
            total_wall_ns: t.total_wall_ns,
        })
        .collect();
    rank_scopes(&mut scopes, unit);

    let scope_self_fuel: u64 = scopes.iter().map(|s| s.self_fuel).sum();
    let scope_self_wall: u64 = scopes.iter().map(|s| s.self_wall_ns).sum();

    ProfileModuleRow {
        module_id: module_id.to_string(),
        unit,
        calls: acc.calls,
        total_fuel: acc.total_fuel,
        total_wall_ns: acc.total_wall_ns,
        // Saturating: a malformed mark stream can leave Σ self above the call
        // total, and a negative "self" share would be nonsense rather than
        // information.
        self_fuel: acc.total_fuel.saturating_sub(scope_self_fuel),
        self_wall_ns: acc.total_wall_ns.saturating_sub(scope_self_wall),
        scopes,
    }
}

/// Rank scope rows by the given unit, descending, name-ascending on ties.
pub(crate) fn rank_scopes(scopes: &mut [ProfileScopeRow], unit: ProfileUnit) {
    scopes.sort_by(|a, b| {
        let (ka, kb) = match unit {
            ProfileUnit::Fuel => (a.self_fuel, b.self_fuel),
            ProfileUnit::WallNs => (a.self_wall_ns, b.self_wall_ns),
        };
        kb.cmp(&ka).then_with(|| a.scope.cmp(&b.scope))
    });
}

/// Resolve a scope id through the host registry, falling back to a stable
/// synthetic label so an unregistered id is still greppable rather than lost.
pub(crate) fn resolve_scope_name(id: u32) -> String {
    scope_name(id).unwrap_or_else(|| format!("scope#{id}"))
}

/// Fold one call's marks into ranked scope rows for `--profile-verbose`.
#[must_use]
pub fn call_detail(marks: &[ProfileMark], call_fuel: u64) -> ProfileCallDetail {
    let mut scopes: Vec<ProfileScopeRow> = fold_marks(marks)
        .into_iter()
        .map(|t| ProfileScopeRow {
            scope: resolve_scope_name(t.scope),
            scope_id: t.scope,
            calls: t.calls,
            self_fuel: t.self_fuel,
            total_fuel: t.total_fuel,
            self_wall_ns: t.self_wall_ns,
            total_wall_ns: t.total_wall_ns,
        })
        .collect();
    rank_scopes(&mut scopes, ProfileUnit::Fuel);
    ProfileCallDetail { call_fuel, scopes }
}

// ---------------------------------------------------------------------------
// Host-side native sink for prepass built-ins
// ---------------------------------------------------------------------------

/// Whether the native sink is currently recording.
///
/// `slicer_core::profile::install` is install-once for the life of the process,
/// so the sink cannot be uninstalled at the end of a profiled run. This flag is
/// what makes a later profiling-off run in the same process observably
/// unchanged: the sink stays installed but every callback returns on one
/// relaxed load.
static NATIVE_RECORDING: AtomicBool = AtomicBool::new(false);

/// The host built-in that owns native marks emitted right now.
///
/// Prepass built-ins run one at a time on the pipeline thread, and any rayon
/// fan-out inside one of them is nested within its bracket — so a single global
/// is sufficient and correct for the stages this attributes. Marks emitted with
/// no owner set (e.g. native geometry reached from the per-layer tier) land
/// under [`UNATTRIBUTED_NATIVE_MODULE`] rather than being silently credited to
/// whichever built-in ran last.
static NATIVE_OWNER: Mutex<Option<&'static str>> = Mutex::new(None);

/// Folded native totals, keyed by owning built-in id.
static NATIVE_ACC: Mutex<BTreeMap<&'static str, NativeAcc>> = Mutex::new(BTreeMap::new());

#[derive(Debug, Default)]
struct NativeAcc {
    activations: u64,
    wall_ns: u64,
    scopes: BTreeMap<u32, ScopeTotals>,
}

thread_local! {
    /// Per-thread mark buffer for the native sink. One outermost scope's worth
    /// at a time: the buffer is folded and flushed the moment the stack
    /// unwinds back to depth 0, so it never grows with the run.
    static NATIVE_STATE: std::cell::RefCell<NativeThreadState> =
        const { std::cell::RefCell::new(NativeThreadState {
            depth: 0,
            epoch: None,
            marks: Vec::new(),
        }) };
}

struct NativeThreadState {
    depth: usize,
    epoch: Option<Instant>,
    marks: Vec<ProfileMark>,
}

/// The `slicer-core` sink that gives host built-ins the same
/// `polygon_ops::*` vocabulary the guests report under.
struct NativeProfileSink;

static NATIVE_SINK: NativeProfileSink = NativeProfileSink;

impl slicer_core::profile::Sink for NativeProfileSink {
    fn enter(&self, scope: ScopeId) {
        if !NATIVE_RECORDING.load(Ordering::Relaxed) {
            return;
        }
        NATIVE_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            if state.depth == 0 {
                state.epoch = Some(Instant::now());
                state.marks.clear();
            }
            let wall_ns = elapsed_ns(state.epoch);
            state.marks.push(ProfileMark {
                scope: scope.raw(),
                edge: MarkEdge::Enter,
                // Native code is not fuel-metered. Leaving this 0 is what makes
                // the fold produce zero fuel for these rows, which is what tags
                // them as wall-clock-only downstream.
                fuel: 0,
                wall_ns,
            });
            state.depth += 1;
        });
    }

    fn exit(&self, scope: ScopeId) {
        if !NATIVE_RECORDING.load(Ordering::Relaxed) {
            return;
        }
        NATIVE_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            // Recording can be switched on inside an already-open scope, in
            // which case this `exit` has no matching `enter`. Dropping it keeps
            // the depth counter honest instead of underflowing it.
            if state.depth == 0 {
                return;
            }
            let wall_ns = elapsed_ns(state.epoch);
            state.marks.push(ProfileMark {
                scope: scope.raw(),
                edge: MarkEdge::Exit,
                fuel: 0,
                wall_ns,
            });
            state.depth -= 1;
            if state.depth == 0 {
                flush_native(&state.marks, wall_ns);
                state.marks.clear();
                state.epoch = None;
            }
        });
    }
}

fn elapsed_ns(epoch: Option<Instant>) -> u64 {
    epoch.map_or(0, |t| t.elapsed().as_nanos() as u64)
}

/// Fold one outermost native activation and merge it under the current owner.
fn flush_native(marks: &[ProfileMark], wall_ns: u64) {
    let folded = fold_marks(marks);
    let owner = NATIVE_OWNER
        .lock()
        .expect("native profile owner poisoned")
        .unwrap_or(UNATTRIBUTED_NATIVE_MODULE);
    let mut acc = NATIVE_ACC.lock().expect("native profile acc poisoned");
    let entry = acc.entry(owner).or_default();
    entry.activations += 1;
    entry.wall_ns += wall_ns;
    for row in &folded {
        let scope = entry.scopes.entry(row.scope).or_insert(ScopeTotals {
            scope: row.scope,
            ..ScopeTotals::default()
        });
        scope.calls += row.calls;
        scope.self_fuel += row.self_fuel;
        scope.total_fuel += row.total_fuel;
        scope.self_wall_ns += row.self_wall_ns;
        scope.total_wall_ns += row.total_wall_ns;
    }
}

/// Whether *this* module won `slicer_core::profile`'s install-once slot.
///
/// Tracked separately from the slot itself because `install` returns `false`
/// both when we already own the slot (fine — just start recording) and when
/// somebody else owns it (not fine — native attribution is unavailable), and
/// those two answers need different handling on the second profiled run in a
/// process.
static NATIVE_SINK_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Install the native sink (idempotent) and start recording.
///
/// Call once at the start of a profiled run. Returns `false` if some other
/// sink had already claimed `slicer_core::profile`'s install-once slot, in
/// which case native attribution is unavailable for this process; the guest
/// half of the report is unaffected either way.
pub fn begin_native_profiling() -> bool {
    if !NATIVE_SINK_INSTALLED.load(Ordering::Acquire) {
        if !slicer_core::profile::install(&NATIVE_SINK) {
            return false;
        }
        NATIVE_SINK_INSTALLED.store(true, Ordering::Release);
    }
    NATIVE_RECORDING.store(true, Ordering::Relaxed);
    true
}

/// Stop recording native marks and hand every accumulated total to `agg`.
///
/// Draining clears the global accumulator so a second profiled run in the same
/// process starts from zero.
pub fn end_native_profiling(agg: &ProfileAggregator) {
    NATIVE_RECORDING.store(false, Ordering::Relaxed);
    let drained = std::mem::take(&mut *NATIVE_ACC.lock().expect("native profile acc poisoned"));
    *NATIVE_OWNER.lock().expect("native profile owner poisoned") = None;
    for (module_id, acc) in drained {
        let folded: Vec<ScopeTotals> = acc.scopes.into_values().collect();
        agg.record_native(module_id, &folded, acc.activations, acc.wall_ns);
    }
}

/// RAII attribution bracket for one host built-in.
///
/// While alive, native marks emitted on any thread are credited to
/// `module_id`. Costs one relaxed atomic load when profiling is off.
pub struct NativeOwnerGuard {
    active: bool,
}

impl NativeOwnerGuard {
    /// Claim native-mark attribution for `module_id`.
    #[must_use]
    pub fn new(module_id: &'static str) -> Self {
        if !NATIVE_RECORDING.load(Ordering::Relaxed) {
            return Self { active: false };
        }
        *NATIVE_OWNER.lock().expect("native profile owner poisoned") = Some(module_id);
        Self { active: true }
    }
}

impl Drop for NativeOwnerGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        *NATIVE_OWNER.lock().expect("native profile owner poisoned") = None;
    }
}

// ---------------------------------------------------------------------------
// Ranked human-readable rendering
// ---------------------------------------------------------------------------

/// Width of the leading label column in the rendered table.
const LABEL_WIDTH: usize = 42;

/// Render `summary` as the ranked stderr table `pnp_cli slice --profile` and
/// `pnp_cli profile --from` both print.
///
/// Percentages are shares of the run total *within a unit*: a fuel row's
/// denominator is [`ProfileSummary::fuel_total`], a wall row's is
/// [`ProfileSummary::wall_total_ns`]. A module's scope rows plus its
/// `<module self>` row therefore sum exactly to the module's own percentage,
/// which is the property that makes the table readable as a decomposition.
#[must_use]
pub fn format_profile_summary(summary: &ProfileSummary) -> String {
    let mut out = String::new();
    out.push_str("=== fuel profile (ADR-0055) ===\n");
    if summary.is_empty() {
        out.push_str(
            "no profile marks recorded.\n\
             Guests emit marks only when built against an SDK that installs the\n\
             profiling sink; host built-ins report only if they ran.\n",
        );
        return out;
    }

    let fuel_rows: Vec<&ProfileModuleRow> = summary
        .modules
        .iter()
        .filter(|m| m.unit == ProfileUnit::Fuel)
        .collect();
    let wall_rows: Vec<&ProfileModuleRow> = summary
        .modules
        .iter()
        .filter(|m| m.unit == ProfileUnit::WallNs)
        .collect();

    if !fuel_rows.is_empty() {
        out.push_str(&format!(
            "units: fuel = executed wasm instructions (deterministic; mark overhead costs no fuel)\n\
             total: {} fuel across {} module(s)\n\n",
            thousands(summary.fuel_total),
            fuel_rows.len()
        ));
        for row in fuel_rows {
            render_module(&mut out, row, summary.fuel_total);
        }
    }

    if !wall_rows.is_empty() {
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&format!(
            "units: wall-clock (native host built-ins are NOT fuel-metered)\n\
             total: {} across {} built-in(s)\n\n",
            human_ns(summary.wall_total_ns),
            wall_rows.len()
        ));
        for row in wall_rows {
            render_module(&mut out, row, summary.wall_total_ns);
        }
    }

    out.push_str(
        "note: fuel ratios are exact (marks burn no fuel); wall-clock under\n\
         --profile is inflated by mark overhead and is indicative only.\n",
    );
    out
}

fn render_module(out: &mut String, row: &ProfileModuleRow, denominator: u64) {
    let cost = row.rank_cost();
    out.push_str(&format!(
        "{label:<width$} {unit} {pct}  {abs:>16}  calls {calls}\n",
        label = truncate(&row.module_id, LABEL_WIDTH),
        width = LABEL_WIDTH,
        unit = row.unit.label(),
        pct = percent(cost, denominator),
        abs = amount(cost, row.unit),
        calls = row.calls,
    ));
    for scope in &row.scopes {
        let scope_cost = match row.unit {
            ProfileUnit::Fuel => scope.self_fuel,
            ProfileUnit::WallNs => scope.self_wall_ns,
        };
        out.push_str(&format!(
            "  {label:<width$} {pct}  {abs:>16}  calls {calls}\n",
            label = truncate(&scope.scope, LABEL_WIDTH - 2),
            width = LABEL_WIDTH - 2,
            pct = percent(scope_cost, denominator),
            abs = amount(scope_cost, row.unit),
            calls = scope.calls,
        ));
    }
    let self_cost = match row.unit {
        ProfileUnit::Fuel => row.self_fuel,
        ProfileUnit::WallNs => row.self_wall_ns,
    };
    out.push_str(&format!(
        "  {label:<width$} {pct}  {abs:>16}\n",
        label = MODULE_SELF_LABEL,
        width = LABEL_WIDTH - 2,
        pct = percent(self_cost, denominator),
        abs = amount(self_cost, row.unit),
    ));
    out.push('\n');
}

fn amount(value: u64, unit: ProfileUnit) -> String {
    match unit {
        ProfileUnit::Fuel => thousands(value),
        ProfileUnit::WallNs => human_ns(value),
    }
}

/// Percentage of `denominator`, or `   n/a` when the denominator is zero —
/// which is honest about "we measured nothing" rather than printing `0.0%`.
fn percent(value: u64, denominator: u64) -> String {
    if denominator == 0 {
        return "   n/a".to_string();
    }
    format!("{:>5.1}%", (value as f64 / denominator as f64) * 100.0)
}

/// Group digits in threes so a 12-digit fuel count is readable at a glance.
fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let lead = digits.len() % 3;
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (i % 3) == lead {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Render nanoseconds at a scale a reader can act on.
fn human_ns(ns: u64) -> String {
    const US: u64 = 1_000;
    const MS: u64 = 1_000_000;
    const S: u64 = 1_000_000_000;
    if ns >= S {
        format!("{:.3} s", ns as f64 / S as f64)
    } else if ns >= MS {
        format!("{:.1} ms", ns as f64 / MS as f64)
    } else if ns >= US {
        format!("{:.1} us", ns as f64 / US as f64)
    } else {
        format!("{ns} ns")
    }
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let keep = width.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('~');
    out
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

    const OFFSET2: u32 = 3; // ScopeId::OFFSET2_EX
    const CLIP: u32 = 1; // ScopeId::CLIP_POLYGONS

    #[test]
    fn empty_aggregator_finishes_empty() {
        let agg = ProfileAggregator::new();
        assert!(agg.is_empty());
        let summary = agg.finish();
        assert!(summary.is_empty());
        assert_eq!(summary.fuel_total, 0);
        assert_eq!(summary.wall_total_ns, 0);
    }

    /// The load-bearing property of the fold: repeated calls of the same module
    /// accumulate into one row, and per-scope self costs add across calls.
    #[test]
    fn repeated_calls_of_one_module_accumulate_into_one_row() {
        let agg = ProfileAggregator::new();
        for _ in 0..3 {
            agg.record_call(
                "com.core.classic-perimeters",
                &[enter(OFFSET2, 0, 0), exit(OFFSET2, 100, 10)],
                250,
            );
        }
        let summary = agg.finish();
        assert_eq!(summary.modules.len(), 1);
        let row = &summary.modules[0];
        assert_eq!(row.module_id, "com.core.classic-perimeters");
        assert_eq!(row.unit, ProfileUnit::Fuel);
        assert_eq!(row.calls, 3);
        assert_eq!(row.total_fuel, 750);
        assert_eq!(row.scopes.len(), 1);
        assert_eq!(row.scopes[0].self_fuel, 300);
        assert_eq!(row.scopes[0].calls, 3);
        // 750 consumed, 300 of it inside a marked scope.
        assert_eq!(row.self_fuel, 450);
        assert_eq!(summary.fuel_total, 750);
    }

    #[test]
    fn scope_ids_resolve_to_core_names() {
        let agg = ProfileAggregator::new();
        agg.record_call("m", &[enter(CLIP, 0, 0), exit(CLIP, 10, 1)], 10);
        let summary = agg.finish();
        assert_eq!(
            summary.modules[0].scopes[0].scope,
            "polygon_ops::clip_polygons"
        );
        assert_eq!(summary.modules[0].scopes[0].scope_id, CLIP);
    }

    #[test]
    fn unregistered_scope_id_gets_a_greppable_placeholder() {
        assert_eq!(resolve_scope_name(ScopeId::USER_BASE - 1), "scope#1023");
    }

    /// A call that produced neither marks nor fuel is exactly what a
    /// profiling-off dispatch looks like. Recording it would make the summary
    /// claim it measured a module it did not.
    #[test]
    fn calls_with_no_signal_are_not_recorded() {
        let agg = ProfileAggregator::new();
        agg.record_call("com.example.quiet", &[], 0);
        assert!(agg.is_empty());
        assert!(agg.finish().is_empty());
    }

    #[test]
    fn nested_scopes_keep_self_and_total_separate_across_calls() {
        let agg = ProfileAggregator::new();
        // offset2_ex calls clip_polygons inside it.
        let marks = [
            enter(OFFSET2, 0, 0),
            enter(CLIP, 100, 10),
            exit(CLIP, 400, 40),
            exit(OFFSET2, 500, 50),
        ];
        agg.record_call("m", &marks, 600);
        let summary = agg.finish();
        let row = &summary.modules[0];
        let by_name = |n: &str| row.scopes.iter().find(|s| s.scope == n).unwrap();
        let outer = by_name("polygon_ops::offset2_ex");
        let inner = by_name("polygon_ops::clip_polygons");
        assert_eq!(outer.total_fuel, 500);
        assert_eq!(outer.self_fuel, 200);
        assert_eq!(inner.total_fuel, 300);
        assert_eq!(inner.self_fuel, 300);
        // Σ self across scopes (200+300) plus module self (100) = call fuel.
        assert_eq!(row.self_fuel, 100);
        assert_eq!(outer.self_fuel + inner.self_fuel + row.self_fuel, 600);
    }

    #[test]
    fn modules_rank_by_cost_descending_with_fuel_before_wall() {
        let agg = ProfileAggregator::new();
        agg.record_call("cheap", &[enter(CLIP, 0, 0), exit(CLIP, 5, 1)], 5);
        agg.record_call("expensive", &[enter(CLIP, 0, 0), exit(CLIP, 900, 1)], 900);
        agg.record_native(
            "host:shell_classification",
            &[ScopeTotals {
                scope: CLIP,
                calls: 4,
                total_wall_ns: 7_000,
                self_wall_ns: 7_000,
                ..Default::default()
            }],
            1,
            9_000,
        );
        let summary = agg.finish();
        let ids: Vec<&str> = summary
            .modules
            .iter()
            .map(|m| m.module_id.as_str())
            .collect();
        assert_eq!(ids, vec!["expensive", "cheap", "host:shell_classification"]);
        assert_eq!(
            summary.fuel_total, 905,
            "wall rows must not enter the fuel base"
        );
        assert_eq!(
            summary.wall_total_ns, 9_000,
            "fuel rows must not enter the wall base"
        );
    }

    #[test]
    fn scopes_within_a_module_rank_by_self_cost_descending() {
        let agg = ProfileAggregator::new();
        agg.record_call(
            "m",
            &[
                enter(CLIP, 0, 0),
                exit(CLIP, 10, 1),
                enter(OFFSET2, 10, 1),
                exit(OFFSET2, 900, 90),
            ],
            900,
        );
        let summary = agg.finish();
        let names: Vec<&str> = summary.modules[0]
            .scopes
            .iter()
            .map(|s| s.scope.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["polygon_ops::offset2_ex", "polygon_ops::clip_polygons"]
        );
    }

    /// Ties must break deterministically or two runs of one workload produce
    /// different reports, which destroys the A/B use this exists for.
    #[test]
    fn equal_cost_modules_break_ties_on_id() {
        let agg = ProfileAggregator::new();
        agg.record_call("zzz", &[enter(CLIP, 0, 0), exit(CLIP, 100, 1)], 100);
        agg.record_call("aaa", &[enter(CLIP, 0, 0), exit(CLIP, 100, 1)], 100);
        let ids: Vec<String> = agg
            .finish()
            .modules
            .into_iter()
            .map(|m| m.module_id)
            .collect();
        assert_eq!(ids, vec!["aaa".to_string(), "zzz".to_string()]);
    }

    #[test]
    fn native_rows_carry_wall_clock_and_zero_fuel() {
        let agg = ProfileAggregator::new();
        agg.record_native(
            "host:slice",
            &[ScopeTotals {
                scope: OFFSET2,
                calls: 12,
                total_wall_ns: 4_000,
                self_wall_ns: 4_000,
                ..Default::default()
            }],
            1,
            5_000,
        );
        let summary = agg.finish();
        let row = &summary.modules[0];
        assert_eq!(row.unit, ProfileUnit::WallNs);
        assert_eq!(row.total_fuel, 0);
        assert_eq!(row.total_wall_ns, 5_000);
        assert_eq!(row.self_wall_ns, 1_000);
    }

    // -- formatting ---------------------------------------------------------

    fn sample_summary() -> ProfileSummary {
        let agg = ProfileAggregator::new();
        agg.record_call(
            "com.core.classic-perimeters",
            &[
                enter(OFFSET2, 0, 0),
                exit(OFFSET2, 4_000, 40),
                enter(CLIP, 4_000, 40),
                exit(CLIP, 5_000, 50),
            ],
            10_000,
        );
        agg.record_call("com.core.gyroid-infill", &[enter(CLIP, 0, 0)], 2_000);
        agg.record_native(
            "host:shell_classification",
            &[ScopeTotals {
                scope: CLIP,
                calls: 3,
                total_wall_ns: 2_500_000_000,
                self_wall_ns: 2_500_000_000,
                ..Default::default()
            }],
            1,
            4_000_000_000,
        );
        agg.finish()
    }

    #[test]
    fn empty_summary_says_so_instead_of_printing_a_blank_table() {
        let text = format_profile_summary(&ProfileAggregator::new().finish());
        assert!(text.contains("no profile marks recorded"), "{text}");
        assert!(!text.contains('%'), "{text}");
    }

    #[test]
    fn ranked_table_lists_modules_then_indented_scopes() {
        let text = format_profile_summary(&sample_summary());
        let module_line = text
            .lines()
            .find(|l| l.starts_with("com.core.classic-perimeters"))
            .expect("module row present");
        assert!(module_line.contains("fuel"), "{module_line}");
        // 10,000 of 12,000 fuel.
        assert!(module_line.contains("83.3%"), "{module_line}");
        assert!(module_line.contains("10,000"), "{module_line}");

        let scope_line = text
            .lines()
            .find(|l| l.trim_start().starts_with("polygon_ops::offset2_ex"))
            .expect("scope row present");
        assert!(scope_line.starts_with("  "), "scope rows must be indented");
        // 4,000 of 12,000.
        assert!(scope_line.contains("33.3%"), "{scope_line}");

        assert!(
            text.contains(MODULE_SELF_LABEL),
            "unattributed share must be shown, not hidden: {text}"
        );
    }

    /// The whole point of tagging units: a wall-clock built-in must never be
    /// ranked in the same block as a fuel-metered guest, and the reader must be
    /// told which is which.
    #[test]
    fn fuel_and_wall_sections_are_separately_normalised_and_labelled() {
        let text = format_profile_summary(&sample_summary());
        assert!(
            text.contains("units: fuel = executed wasm instructions"),
            "{text}"
        );
        assert!(text.contains("NOT fuel-metered"), "{text}");

        let fuel_heading = text.find("units: fuel").unwrap();
        let wall_heading = text.find("units: wall-clock").unwrap();
        let perimeters = text.find("com.core.classic-perimeters").unwrap();
        let shell = text.find("host:shell_classification").unwrap();
        assert!(
            fuel_heading < perimeters && perimeters < wall_heading && wall_heading < shell,
            "every fuel row must precede the wall-clock heading:\n{text}"
        );

        // The native row is 100% of the wall base, not of the fuel base.
        let shell_line = text
            .lines()
            .find(|l| l.starts_with("host:shell_classification"))
            .unwrap();
        assert!(shell_line.contains("100.0%"), "{shell_line}");
        assert!(shell_line.contains("4.000 s"), "{shell_line}");
    }

    #[test]
    fn summary_always_states_the_observer_effect_contract() {
        let text = format_profile_summary(&sample_summary());
        assert!(text.contains("fuel ratios are exact"), "{text}");
        assert!(text.contains("indicative only"), "{text}");
    }

    #[test]
    fn scope_percentages_plus_module_self_reconstruct_the_module_percentage() {
        let summary = sample_summary();
        let row = summary
            .modules
            .iter()
            .find(|m| m.module_id == "com.core.classic-perimeters")
            .unwrap();
        let scope_self: u64 = row.scopes.iter().map(|s| s.self_fuel).sum();
        assert_eq!(
            scope_self + row.self_fuel,
            row.total_fuel,
            "the decomposition must be exact, or the table lies about where cost went"
        );
    }

    #[test]
    fn thousands_groups_every_magnitude() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(12_345), "12,345");
        assert_eq!(thousands(123_456), "123,456");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn human_ns_picks_a_readable_scale() {
        assert_eq!(human_ns(999), "999 ns");
        assert_eq!(human_ns(1_500), "1.5 us");
        assert_eq!(human_ns(2_500_000), "2.5 ms");
        assert_eq!(human_ns(3_000_000_000), "3.000 s");
    }

    #[test]
    fn percent_reports_na_rather_than_zero_when_nothing_was_measured() {
        assert_eq!(percent(0, 0).trim(), "n/a");
        assert_eq!(percent(1, 4).trim(), "25.0%");
    }

    #[test]
    fn long_ids_are_truncated_with_a_marker_rather_than_breaking_the_columns() {
        let long = "com.example.a-very-long-module-identifier-that-overflows-the-column";
        let truncated = truncate(long, LABEL_WIDTH);
        assert_eq!(truncated.chars().count(), LABEL_WIDTH);
        assert!(truncated.ends_with('~'));
        assert_eq!(truncate("short", LABEL_WIDTH), "short");
    }

    #[test]
    fn call_detail_folds_one_call_and_ranks_by_self_fuel() {
        let detail = call_detail(
            &[
                enter(CLIP, 0, 0),
                exit(CLIP, 10, 1),
                enter(OFFSET2, 10, 1),
                exit(OFFSET2, 500, 50),
            ],
            500,
        );
        assert_eq!(detail.call_fuel, 500);
        assert_eq!(detail.scopes[0].scope, "polygon_ops::offset2_ex");
        assert_eq!(detail.scopes[0].self_fuel, 490);
        assert_eq!(detail.scopes[1].scope, "polygon_ops::clip_polygons");
    }

    #[test]
    fn summary_round_trips_through_json() {
        let summary = sample_summary();
        let json = serde_json::to_string(&summary).unwrap();
        let back: ProfileSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, summary);
        assert!(json.contains("\"unit\":\"fuel\""), "{json}");
        assert!(json.contains("\"unit\":\"wall_ns\""), "{json}");
    }

    /// With recording off, the sink must be inert — this is what makes
    /// "`--profile` off changes nothing" hold for a process that profiled
    /// earlier, since the sink cannot be uninstalled.
    #[test]
    fn native_sink_records_nothing_while_recording_is_off() {
        use slicer_core::profile::Sink;
        NATIVE_RECORDING.store(false, Ordering::Relaxed);
        NATIVE_SINK.enter(ScopeId::CLIP_POLYGONS);
        NATIVE_SINK.exit(ScopeId::CLIP_POLYGONS);
        assert!(NATIVE_ACC
            .lock()
            .unwrap()
            .get(UNATTRIBUTED_NATIVE_MODULE)
            .is_none());
        NATIVE_STATE.with(|c| assert_eq!(c.borrow().depth, 0));
    }
}
