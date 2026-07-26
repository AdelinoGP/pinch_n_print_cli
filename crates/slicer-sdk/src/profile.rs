//! Guest-side half of fuel-based module profiling (ADR-0050).
//!
//! # What this module is for
//!
//! `slicer_core::profile` owns the *seam*: [`slicer_core::profile::scope`] marks
//! the clipper2 primitives and forwards to whatever
//! [`Sink`](slicer_core::profile::Sink) has been installed, and installs nothing
//! by default. This module is the guest's installer — it bridges that seam to
//! the WIT `slicer:common/profiling` interface so a mark raised inside a wasm
//! module reaches the host, which attaches the fuel and wall-clock reading it
//! alone can take.
//!
//! Without an installed sink the whole chain is inert: the marks exist in the
//! guest binary and go nowhere. [`install_guest_sink`] is what turns it on, and
//! `#[slicer_module]`'s generated component glue calls it at the top of every
//! WIT export body, so it has already run before any module code can execute.
//!
//! # Cost when profiling is off
//!
//! One host call per guest *instance* — the cached [`ProfilingHost::profile_enabled`]
//! answer. After that, every mark costs an `OnceLock::get` (one acquire load) plus
//! a branch on a cached `bool`, and no scope name is ever registered. Marks ship in
//! every guest precisely because that is cheap enough to leave in (ADR-0050,
//! "Always compiled in, host-gated").
//!
//! # Scope identity
//!
//! The guest and the host mint scope ids independently, so a raw id must be
//! translated before it crosses the boundary:
//!
//! - **Core scopes** (`raw < ScopeId::USER_BASE`, enumerated by
//!   [`ScopeId::CORE_SCOPES`]) are translated through the host: at prime time the
//!   bridge calls `profile-register(ScopeId::name())` for each and remembers what
//!   the host handed back. The host resolves a name that matches a core scope to
//!   its reserved id, so today the answer is the same number — but the bridge
//!   never assumes that, it uses whatever it was told. A core scope the host
//!   declines to name is dropped rather than guessed at.
//! - **User scopes** (`raw >= ScopeId::USER_BASE`) pass through unchanged,
//!   because the only way a guest can obtain one is [`register_scope`], which
//!   returns the host's own id.
//!
//! # Why the bridge is generic over [`ProfilingHost`]
//!
//! The WIT import only exists on `wasm32`, so a sink written directly against it
//! could only be tested inside a running host. [`BridgeSink`] instead talks to a
//! [`ProfilingHost`], of which the WIT import is one implementation and a test
//! double is another. Everything except the four-line WIT shim is therefore
//! exercised natively.
//!
//! # Native builds
//!
//! [`install_guest_sink`] compiles to nothing off `wasm32`, so module unit tests
//! run without a runtime and without perturbing whatever sink the host process
//! installed for itself.

use std::sync::OnceLock;

use slicer_core::profile::{ScopeId, Sink};

/// Which side of a scope a mark sits on. Mirrors the WIT `mark-edge` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkEdge {
    /// The scope was entered, before its work began.
    Enter,
    /// The scope was left, after its work completed.
    Exit,
}

/// The host half of the profiling bridge, as [`BridgeSink`] sees it.
///
/// One implementation is the real WIT import (`wasm32` only); the other is
/// whatever a test installs. Methods take `&self` so a `'static` implementation
/// can sit inside a `static` sink with no interior mutability.
///
/// Implementations MUST NOT panic and MUST NOT call into `slicer-core` geometry:
/// a mark raised from inside a mark would recurse forever.
pub trait ProfilingHost: Send + Sync + 'static {
    /// Whether the host is recording marks at all. Called once per guest
    /// instance, by [`BridgeSink::prime`].
    fn profile_enabled(&self) -> bool;

    /// Interns `name` and returns the host's id for it. Called only while
    /// priming, and only when profiling is on.
    fn profile_register(&self, name: &str) -> u32;

    /// Records one scope transition under a host-minted id.
    fn profile_mark(&self, scope: u32, edge: MarkEdge);
}

/// Sentinel in [`Bridge::core_ids`] for "this core scope has no host id".
const UNMAPPED: u32 = u32::MAX;

/// Everything [`BridgeSink`] caches for the life of the guest instance.
struct Bridge {
    /// The host's answer to `profile-enabled`, asked exactly once.
    enabled: bool,
    /// `core_ids[raw]` is the host id for the core scope with that raw id, or
    /// [`UNMAPPED`]. A direct index rather than a lookup table because core raw
    /// ids are small and dense, so the hot path is one bounds check.
    ///
    /// Empty when profiling is off: nothing will be marked, so nothing is
    /// registered.
    core_ids: Vec<u32>,
}

/// A [`Sink`] that forwards `slicer-core` scope marks to a [`ProfilingHost`].
///
/// Install one with [`install_guest_sink`] (the guest path) or by handing it to
/// [`slicer_core::profile::install`] directly (the native test path). Either way
/// it must be [`BridgeSink::prime`]d first, so a mark can never observe a
/// half-built id table.
pub struct BridgeSink<H: ProfilingHost> {
    host: H,
    bridge: OnceLock<Bridge>,
}

impl<H: ProfilingHost> BridgeSink<H> {
    /// Wraps `host` in an unprimed bridge.
    ///
    /// `const` so the sink can live in a `static` and satisfy the
    /// `&'static dyn Sink` that [`slicer_core::profile::install`] takes.
    pub const fn new(host: H) -> Self {
        Self {
            host,
            bridge: OnceLock::new(),
        }
    }

    /// Asks the host whether profiling is on and, if it is, registers every
    /// core scope name. Idempotent: later calls return the cached answer
    /// without touching the host.
    ///
    /// Returns whether profiling is enabled, so a caller can skip work it would
    /// only do for a profiler.
    pub fn prime(&self) -> bool {
        self.bridge
            .get_or_init(|| {
                let enabled = self.host.profile_enabled();
                if !enabled {
                    // Registration is answered even with profiling off, but
                    // asking would cost one host call per core scope per guest
                    // instance to build a table nothing will read.
                    return Bridge {
                        enabled,
                        core_ids: Vec::new(),
                    };
                }
                let mut core_ids: Vec<u32> = Vec::new();
                for scope in ScopeId::CORE_SCOPES {
                    let Some(name) = scope.name() else { continue };
                    let raw = scope.raw() as usize;
                    if core_ids.len() <= raw {
                        core_ids.resize(raw + 1, UNMAPPED);
                    }
                    core_ids[raw] = self.host.profile_register(name);
                }
                Bridge { enabled, core_ids }
            })
            .enabled
    }

    /// Whether this sink has been primed and the host said profiling is on.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.bridge.get().is_some_and(|bridge| bridge.enabled)
    }

    /// The host id `scope` should be reported under, or `None` if it cannot be
    /// translated and must be dropped.
    fn resolve(bridge: &Bridge, scope: ScopeId) -> Option<u32> {
        let raw = scope.raw();
        if raw >= ScopeId::USER_BASE {
            // Minted by the host itself, via `register_scope`.
            return Some(raw);
        }
        match bridge.core_ids.get(raw as usize).copied() {
            Some(UNMAPPED) | None => None,
            Some(id) => Some(id),
        }
    }

    /// The whole hot path: one `OnceLock::get`, one branch on a cached bool,
    /// one bounds-checked index, then the host call.
    #[inline]
    fn mark(&self, scope: ScopeId, edge: MarkEdge) {
        let Some(bridge) = self.bridge.get() else {
            // Not primed. Nothing can be translated yet, and guessing an id
            // would file this cost under an unrelated scope.
            return;
        };
        if !bridge.enabled {
            return;
        }
        if let Some(host_id) = Self::resolve(bridge, scope) {
            self.host.profile_mark(host_id, edge);
        }
    }

    /// Interns `name` with the host and returns the id it minted, or `None`
    /// when profiling is off (or this sink is unprimed) and there is nothing to
    /// report. See [`register_scope`].
    #[must_use]
    pub fn register_scope(&self, name: &str) -> Option<ScopeId> {
        let bridge = self.bridge.get()?;
        if !bridge.enabled {
            return None;
        }
        Some(ScopeId::new(self.host.profile_register(name)))
    }
}

impl<H: ProfilingHost> Sink for BridgeSink<H> {
    fn enter(&self, scope: ScopeId) {
        self.mark(scope, MarkEdge::Enter);
    }

    fn exit(&self, scope: ScopeId) {
        self.mark(scope, MarkEdge::Exit);
    }
}

// ── The WIT-backed host, and the guest sink built on it ─────────────────

#[cfg(target_arch = "wasm32")]
mod wit {
    // Self-contained mini WIT world scoped to just the profiling interface, as
    // with `host::log`'s and `host::medial_axis`'s equivalent inline bindings:
    // it does not depend on the full world generated by the `#[slicer_module]`
    // macro (private inner module, not accessible from this crate). Component
    // imports resolve structurally against the real host runtime's
    // `slicer:common/profiling` interface, so only the wire shape has to match
    // the canonical `profiling` interface in
    // `crates/slicer-schema/wit/deps/common.wit`.
    #[allow(dead_code)]
    mod bindings {
        ::wit_bindgen::generate!({
            inline: r#"
package slicer:sdk-profiling-helper;

package slicer:common {
    interface profiling {
        enum mark-edge { enter, exit }
        profile-enabled: func() -> bool;
        profile-register: func(name: string) -> u32;
        profile-mark: func(scope: u32, edge: mark-edge);
    }
}

world sdk-profiling {
    import slicer:common/profiling;
}
"#,
            world: "sdk-profiling",
            generate_all,
        });
    }

    use super::{MarkEdge, ProfilingHost};

    /// The real host: every method is one WIT import call.
    pub(super) struct WitProfilingHost;

    impl ProfilingHost for WitProfilingHost {
        fn profile_enabled(&self) -> bool {
            bindings::slicer::common::profiling::profile_enabled()
        }

        fn profile_register(&self, name: &str) -> u32 {
            bindings::slicer::common::profiling::profile_register(name)
        }

        fn profile_mark(&self, scope: u32, edge: MarkEdge) {
            let wit_edge = match edge {
                MarkEdge::Enter => bindings::slicer::common::profiling::MarkEdge::Enter,
                MarkEdge::Exit => bindings::slicer::common::profiling::MarkEdge::Exit,
            };
            bindings::slicer::common::profiling::profile_mark(scope, wit_edge);
        }
    }
}

#[cfg(target_arch = "wasm32")]
static GUEST_SINK: BridgeSink<wit::WitProfilingHost> = BridgeSink::new(wit::WitProfilingHost);

/// Installs the guest profiling sink, exactly once per guest instance.
///
/// Called by `#[slicer_module]`-generated glue at the top of every WIT export
/// body, so the sink is in place before any module code — and therefore before
/// any `slicer_core::polygon_ops` call — can run. Repeat calls are cheap: both
/// the prime and the install are `OnceLock`-guarded.
///
/// Priming happens *before* installing so that no scope can observe a sink whose
/// id table is not yet built.
///
/// A no-op off `wasm32`: there is no host to bridge to, module unit tests must
/// keep running natively, and the native sink slot belongs to whoever owns that
/// process (`slicer-wasm-host` installs its own).
pub fn install_guest_sink() {
    #[cfg(target_arch = "wasm32")]
    {
        GUEST_SINK.prime();
        slicer_core::profile::install(&GUEST_SINK);
    }
}

/// Interns a module-defined scope name with the host and returns the id to mark
/// it under, or `None` when profiling is off.
///
/// The returned id is the host's own, at or above [`ScopeId::USER_BASE`], so
/// [`BridgeSink`] forwards it unchanged. Pair it with
/// [`slicer_core::profile::scope`]:
///
/// ```no_run
/// let guard = slicer_sdk::profile::register_scope("my_module::inner_loop")
///     .map(slicer_core::profile::scope);
/// // ... work ...
/// drop(guard); // or let it fall out of scope
/// ```
///
/// Always `None` off `wasm32` and always `None` before [`install_guest_sink`]
/// has run, both of which mean "nothing is recording, skip the scope".
#[must_use]
pub fn register_scope(name: &str) -> Option<ScopeId> {
    #[cfg(target_arch = "wasm32")]
    {
        GUEST_SINK.register_scope(name)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A [`ProfilingHost`] that records what the bridge asked it, and hands out
    /// ids that deliberately do NOT match the core raw ids — so a bridge that
    /// forwarded `ScopeId::raw()` straight through would fail these tests even
    /// though the real host happens to answer with the reserved id.
    struct FakeHost {
        enabled: bool,
    }

    thread_local! {
        static REGISTERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static MARKS: RefCell<Vec<(u32, MarkEdge)>> = const { RefCell::new(Vec::new()) };
        static ENABLED_QUERIES: RefCell<u32> = const { RefCell::new(0) };
    }

    /// Ids the fake host mints, offset far from both the core ids and
    /// `USER_BASE` so a pass-through bug is unambiguous.
    const FAKE_ID_BASE: u32 = 7000;

    impl ProfilingHost for FakeHost {
        fn profile_enabled(&self) -> bool {
            ENABLED_QUERIES.with(|n| *n.borrow_mut() += 1);
            self.enabled
        }

        fn profile_register(&self, name: &str) -> u32 {
            REGISTERED.with(|reg| {
                let mut reg = reg.borrow_mut();
                if let Some(index) = reg.iter().position(|seen| seen == name) {
                    return FAKE_ID_BASE + index as u32;
                }
                reg.push(name.to_string());
                FAKE_ID_BASE + (reg.len() - 1) as u32
            })
        }

        fn profile_mark(&self, scope: u32, edge: MarkEdge) {
            MARKS.with(|marks| marks.borrow_mut().push((scope, edge)));
        }
    }

    fn reset() {
        REGISTERED.with(|reg| reg.borrow_mut().clear());
        MARKS.with(|marks| marks.borrow_mut().clear());
        ENABLED_QUERIES.with(|n| *n.borrow_mut() = 0);
    }

    fn registered() -> Vec<String> {
        REGISTERED.with(|reg| reg.borrow().clone())
    }

    fn marks() -> Vec<(u32, MarkEdge)> {
        MARKS.with(|marks| marks.borrow().clone())
    }

    #[test]
    fn enabled_bridge_registers_core_names_and_marks_under_host_ids() {
        reset();
        let sink = BridgeSink::new(FakeHost { enabled: true });
        assert!(sink.prime());
        assert!(sink.is_enabled());

        // Every core scope was registered by name, in `CORE_SCOPES` order.
        let expected: Vec<String> = ScopeId::CORE_SCOPES
            .iter()
            .map(|s| s.name().expect("core scope must be named").to_string())
            .collect();
        assert_eq!(registered(), expected);

        // A mark carries the id the host handed out, not the guest's raw id.
        sink.enter(ScopeId::OFFSET);
        sink.exit(ScopeId::OFFSET);
        let offset_index = ScopeId::CORE_SCOPES
            .iter()
            .position(|s| *s == ScopeId::OFFSET)
            .expect("OFFSET is a core scope");
        let offset_host_id = FAKE_ID_BASE + offset_index as u32;
        assert_ne!(
            offset_host_id,
            ScopeId::OFFSET.raw(),
            "the fake host must not agree with the raw id, or this proves nothing"
        );
        assert_eq!(
            marks(),
            vec![
                (offset_host_id, MarkEdge::Enter),
                (offset_host_id, MarkEdge::Exit)
            ]
        );

        // Priming is once per instance: no second `profile-enabled`, no
        // re-registration.
        assert!(sink.prime());
        assert_eq!(ENABLED_QUERIES.with(|n| *n.borrow()), 1);
        assert_eq!(registered(), expected);
    }

    #[test]
    fn disabled_bridge_asks_once_and_never_calls_the_host_again() {
        reset();
        let sink = BridgeSink::new(FakeHost { enabled: false });
        assert!(!sink.prime());
        assert!(!sink.is_enabled());

        for scope in ScopeId::CORE_SCOPES {
            sink.enter(*scope);
            sink.exit(*scope);
        }
        sink.enter(ScopeId::new(ScopeId::USER_BASE));
        assert_eq!(sink.register_scope("never_registered"), None);

        assert_eq!(
            ENABLED_QUERIES.with(|n| *n.borrow()),
            1,
            "profile-enabled is cached once per instance"
        );
        assert!(
            registered().is_empty(),
            "nothing may be registered while profiling is off"
        );
        assert!(marks().is_empty(), "no mark may reach the host");
    }

    #[test]
    fn unprimed_bridge_is_inert() {
        reset();
        let sink = BridgeSink::new(FakeHost { enabled: true });
        sink.enter(ScopeId::CLIP_POLYGONS);
        sink.exit(ScopeId::CLIP_POLYGONS);
        assert_eq!(sink.register_scope("too_early"), None);
        assert!(!sink.is_enabled());
        assert_eq!(ENABLED_QUERIES.with(|n| *n.borrow()), 0);
        assert!(marks().is_empty());
    }

    #[test]
    fn user_scope_ids_pass_through_and_core_ids_are_translated() {
        reset();
        let sink = BridgeSink::new(FakeHost { enabled: true });
        sink.prime();

        // `register_scope` returns the host's id, at or above USER_BASE for a
        // name the host has not reserved.
        let user = sink
            .register_scope("my_module::inner_loop")
            .expect("profiling is on");
        assert!(user.raw() >= FAKE_ID_BASE);
        MARKS.with(|marks| marks.borrow_mut().clear());

        sink.enter(user);
        sink.exit(user);
        assert_eq!(
            marks(),
            vec![(user.raw(), MarkEdge::Enter), (user.raw(), MarkEdge::Exit)],
            "an id already minted by the host must cross unchanged"
        );

        // An id below USER_BASE that no core scope claims cannot be translated,
        // so it is dropped rather than misattributed.
        MARKS.with(|marks| marks.borrow_mut().clear());
        let unknown = ScopeId::new(ScopeId::USER_BASE - 1);
        assert_eq!(unknown.name(), None, "this id must be unnamed to test drop");
        sink.enter(unknown);
        sink.exit(unknown);
        assert!(marks().is_empty());
    }

    #[test]
    fn install_guest_sink_is_a_native_no_op() {
        // On this target there is no host to bridge to, so the `slicer-core`
        // slot must be left exactly as it was — module unit tests depend on it.
        let before = slicer_core::profile::sink().is_some();
        install_guest_sink();
        assert_eq!(slicer_core::profile::sink().is_some(), before);
        assert_eq!(register_scope("native"), None);
    }
}
