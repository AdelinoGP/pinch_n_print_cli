//! Zero-cost-when-unused profiling scopes for `slicer-core` primitives.
//!
//! # Why this lives in `slicer-core`
//!
//! The dependency edge runs `slicer-sdk -> slicer-core`, never the reverse, so
//! [`crate::polygon_ops`] cannot call into the SDK to emit a scope mark. Instead
//! `slicer-core` owns the *seam*: it declares a [`Sink`] trait and a
//! process-global slot, and whoever is in a position to record marks installs an
//! implementation:
//!
//! - Guest side: `slicer-sdk` installs a sink that forwards to the WIT
//!   `profile-mark` host function.
//! - Host side: `slicer-wasm-host` installs a native sink.
//!
//! # Two independent copies of the global state
//!
//! `slicer-core` is compiled into every wasm32 guest *and* into the native host.
//! Each of those artifacts links its **own** copy of [`static@SINK`]: the guest's copy
//! lives inside the wasm instance's linear memory, the host's copy lives in the
//! host process. They are separate objects with separate lifetimes and cannot
//! observe or interfere with each other. Installing a sink on the host does not
//! make guest calls to [`scope`] emit anything, and vice versa. Each side must
//! install its own sink, and a guest's marks reach the host only by way of the
//! guest sink's explicit WIT call.
//!
//! # Cost when no sink is installed
//!
//! [`scope`] performs exactly one `OnceLock::get` (an acquire load of a single
//! atomic) and one branch. No allocation, no locking, no thread-local access.
//! The returned [`ScopeGuard`] is a plain value holding the scope id and a
//! copied `Option<&'static dyn Sink>`, so `Drop` is another branch.
//!
//! # wasm32 compatibility
//!
//! This module uses only `core`/`std` primitives that exist on
//! `wasm32-unknown-unknown`: `OnceLock`, references, and `Drop`. No clock, no
//! threads, no WASI.
//!
//! # Example
//!
//! ```
//! use slicer_core::profile::{self, ScopeId, Sink};
//!
//! struct CountingSink;
//! impl Sink for CountingSink {
//!     fn enter(&self, _scope: ScopeId) {}
//!     fn exit(&self, _scope: ScopeId) {}
//! }
//!
//! // Installers hand over a `&'static dyn Sink`; `install` is once-per-process.
//! let _ = profile::install(&CountingSink);
//!
//! {
//!     let _guard = profile::scope(ScopeId::CLIP_POLYGONS);
//!     // ... work ...
//! } // `exit` fires here, on drop.
//! ```

use std::sync::OnceLock;

/// Identity of a profiling scope.
///
/// A scope id is an opaque `u32`. Ids below [`ScopeId::USER_BASE`] are reserved
/// for `slicer-core` itself and are enumerated by [`ScopeId::CORE_SCOPES`];
/// everything at or above `USER_BASE` belongs to whoever installed the
/// [`Sink`] and is never minted by this crate.
///
/// The type is deliberately `Copy` and `u32`-sized so a mark can cross the WIT
/// boundary as a scalar without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(u32);

impl ScopeId {
    /// First scope id NOT reserved by `slicer-core`.
    ///
    /// Sink installers may mint their own ids at or above this value without
    /// risking a collision with a future core scope.
    pub const USER_BASE: u32 = 1024;

    /// `polygon_ops::clip_polygons` — the single clipper2 boolean primitive.
    pub const CLIP_POLYGONS: ScopeId = ScopeId(1);

    /// `polygon_ops::offset` — the single-pass clipper2 inflate primitive.
    pub const OFFSET: ScopeId = ScopeId(2);

    /// `polygon_ops::offset2_ex` — the two-pass clipper2 inflate primitive.
    pub const OFFSET2_EX: ScopeId = ScopeId(3);

    /// `polygon_ops::opening` — erode-then-dilate.
    ///
    /// Marked at the public function rather than at the shared `morph_pass`
    /// helper it calls twice: `morph_pass` reaches `inflate_once` directly,
    /// bypassing [`ScopeId::OFFSET`], so without this the op would report
    /// nothing. Marking `morph_pass` instead would emit two marks per call
    /// under a name no reader recognises.
    pub const OPENING: ScopeId = ScopeId(4);

    /// `polygon_ops::closing_ex` — dilate-then-erode. Same `morph_pass`
    /// bypass as [`ScopeId::OPENING`].
    pub const CLOSING_EX: ScopeId = ScopeId(5);

    /// Every scope id minted by `slicer-core`, in ascending order.
    ///
    /// A sink implementation can walk this to build an id-to-name table up
    /// front instead of calling [`ScopeId::name`] on the hot path.
    pub const CORE_SCOPES: &'static [ScopeId] = &[
        ScopeId::CLIP_POLYGONS,
        ScopeId::OFFSET,
        ScopeId::OFFSET2_EX,
        ScopeId::OPENING,
        ScopeId::CLOSING_EX,
    ];

    /// Constructs a scope id from its raw representation.
    ///
    /// Callers outside `slicer-core` should pass a value at or above
    /// [`ScopeId::USER_BASE`].
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        ScopeId(raw)
    }

    /// Returns the raw representation, for transport across the WIT boundary.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Returns the stable human-readable name of a core scope, or `None` for an
    /// id this crate did not mint.
    ///
    /// The names are part of the profiling contract: they are what a report
    /// shows, so they match the Rust symbol they wrap.
    #[must_use]
    pub const fn name(self) -> Option<&'static str> {
        match self.0 {
            1 => Some("polygon_ops::clip_polygons"),
            2 => Some("polygon_ops::offset"),
            3 => Some("polygon_ops::offset2_ex"),
            4 => Some("polygon_ops::opening"),
            5 => Some("polygon_ops::closing_ex"),
            _ => None,
        }
    }
}

/// Receiver of profiling scope transitions.
///
/// Object-safe by construction: both methods take `&self` and only `Copy`
/// scalars, so a `&'static dyn Sink` is all [`install`] needs. Implementations
/// must be `Send + Sync` because `slicer-core` algorithms may run on a rayon
/// pool under the `host-algos` feature.
///
/// Implementations MUST NOT panic and MUST NOT re-enter `slicer-core` geometry
/// primitives: a sink that calls [`crate::polygon_ops::clip_polygons`] would
/// recurse forever through [`scope`].
pub trait Sink: Send + Sync + 'static {
    /// Called when a scope is entered, before the work begins.
    fn enter(&self, scope: ScopeId);

    /// Called when a scope is left, after the work completes — including on an
    /// early return or an unwind, because [`ScopeGuard`] fires it from `Drop`.
    fn exit(&self, scope: ScopeId);
}

/// The process-global sink.
///
/// See the module docs: the wasm guest and the native host each link their own
/// copy of this static, so they are independent slots, not a shared one.
static SINK: OnceLock<&'static dyn Sink> = OnceLock::new();

/// Installs the process-global profiling sink.
///
/// Returns `true` if this call installed `sink`, `false` if a sink was already
/// installed (in which case `sink` is discarded and the incumbent is kept).
/// Install-once is deliberate: it keeps [`scope`] down to a single relaxed-cost
/// atomic load with no lock, and it means a mid-run swap can never split a
/// scope's `enter` and `exit` across two different sinks.
pub fn install(sink: &'static dyn Sink) -> bool {
    SINK.set(sink).is_ok()
}

/// Returns the installed sink, or `None` if profiling is off.
///
/// Exposed so an installer can check whether it (or someone else) already won
/// the [`install`] race without attempting a second install.
#[must_use]
pub fn sink() -> Option<&'static dyn Sink> {
    SINK.get().copied()
}

/// RAII guard for one profiling scope.
///
/// Created by [`scope`]. Emits [`Sink::exit`] from `Drop`, so scopes nest
/// correctly and are safe across early returns, `?`, and unwinding panics.
#[must_use = "a ScopeGuard measures nothing if dropped immediately; bind it to `_guard`"]
pub struct ScopeGuard {
    id: ScopeId,
    /// Captured at `enter` time so `exit` can never be delivered to a different
    /// sink than the matching `enter` was, and so `Drop` needs no second load.
    sink: Option<&'static dyn Sink>,
}

impl ScopeGuard {
    /// The scope this guard will close on drop.
    #[must_use]
    pub const fn id(&self) -> ScopeId {
        self.id
    }
}

impl Drop for ScopeGuard {
    #[inline]
    fn drop(&mut self) {
        if let Some(sink) = self.sink {
            sink.exit(self.id);
        }
    }
}

/// Opens profiling scope `id`, returning a guard that closes it on drop.
///
/// With no sink installed this is one atomic load plus one branch and allocates
/// nothing; the returned guard's `Drop` is a second branch.
#[inline]
pub fn scope(id: ScopeId) -> ScopeGuard {
    let sink = SINK.get().copied();
    if let Some(sink) = sink {
        sink.enter(id);
    }
    ScopeGuard { id, sink }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_ops::{
        clip_polygons, closing_ex, difference_ex, offset, offset2_ex, opening, opening_ex,
        union_ex, ClipOperation, OffsetJoinType,
    };
    use slicer_ir::{ExPolygon, Point2, Polygon};
    use std::cell::RefCell;

    /// One observed sink callback.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Enter(ScopeId),
        Exit(ScopeId),
    }

    thread_local! {
        /// `Some` only while the current thread is inside [`record`]. Keeping the
        /// buffer thread-local means the once-installed global sink cannot be
        /// polluted by, or pollute, any other test running concurrently in this
        /// same test binary.
        static EVENTS: RefCell<Option<Vec<Event>>> = const { RefCell::new(None) };
    }

    struct ThreadLocalRecorder;

    impl Sink for ThreadLocalRecorder {
        fn enter(&self, scope: ScopeId) {
            push(Event::Enter(scope));
        }
        fn exit(&self, scope: ScopeId) {
            push(Event::Exit(scope));
        }
    }

    fn push(event: Event) {
        EVENTS.with(|slot| {
            if let Some(buffer) = slot.borrow_mut().as_mut() {
                buffer.push(event);
            }
        });
    }

    /// Runs `body` with recording enabled on this thread and returns what the
    /// installed sink observed. Returns an empty vec when no sink is installed.
    fn record<R>(body: impl FnOnce() -> R) -> (R, Vec<Event>) {
        EVENTS.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
        let out = body();
        let events = EVENTS.with(|slot| slot.borrow_mut().take().unwrap_or_default());
        (out, events)
    }

    fn square(min: f32, max: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min, min),
                    Point2::from_mm(max, min),
                    Point2::from_mm(max, max),
                    Point2::from_mm(min, max),
                ],
            },
            holes: Vec::new(),
        }
    }

    /// Opens a scope and returns early without ever reaching the end of the
    /// function body — the guard must still emit `exit`.
    fn early_return_under_scope(id: ScopeId, bail: bool) -> &'static str {
        let _guard = scope(id);
        if bail {
            return "bailed";
        }
        "fell through"
    }

    /// A single test owns the whole lifecycle because [`static@SINK`] is a
    /// `OnceLock`: install-once means two tests racing to install would be
    /// order-dependent. Phase 1 runs before any install, phases 2+ after.
    #[test]
    fn profiling_scopes_are_inert_until_installed_then_record_nested_marks() {
        let subject = vec![square(0.0, 10.0)];
        let clip = vec![square(4.0, 12.0)];

        // ---- Phase 1: no sink installed -------------------------------------
        // The marked ops must not panic and must not produce any observable
        // effect: the recorder is armed, yet nothing reaches it.
        assert!(sink().is_none(), "no sink may be installed before phase 1");

        let (uninstalled_results, uninstalled_events) = record(|| {
            (
                clip_polygons(&subject, &clip, ClipOperation::Difference),
                offset(&subject, 0.5, OffsetJoinType::Miter, 0.0),
                offset2_ex(&subject, -0.5, 0.5, OffsetJoinType::Miter, 3.0),
                difference_ex(&subject, &clip),
                opening_ex(&subject, 0.5, OffsetJoinType::Miter, 3.0),
                union_ex(&subject),
            )
        });
        assert!(
            uninstalled_events.is_empty(),
            "uninstalled sink must be a true no-op, saw {uninstalled_events:?}"
        );

        // The guard is also inert with no sink, including on the early-return path.
        let (early, early_events) =
            record(|| early_return_under_scope(ScopeId::CLIP_POLYGONS, true));
        assert_eq!(early, "bailed");
        assert!(early_events.is_empty());

        // ---- Phase 2: install ----------------------------------------------
        static RECORDER: ThreadLocalRecorder = ThreadLocalRecorder;
        assert!(install(&RECORDER), "this test must win the install race");
        assert!(!install(&RECORDER), "install is once-per-process");
        assert!(sink().is_some());

        // ---- Phase 3: the three marked primitives ---------------------------
        let (clip_out, events) =
            record(|| clip_polygons(&subject, &clip, ClipOperation::Difference));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::CLIP_POLYGONS),
                Event::Exit(ScopeId::CLIP_POLYGONS)
            ]
        );

        let (offset_out, events) = record(|| offset(&subject, 0.5, OffsetJoinType::Miter, 0.0));
        assert_eq!(
            events,
            vec![Event::Enter(ScopeId::OFFSET), Event::Exit(ScopeId::OFFSET)]
        );

        let (offset2_out, events) =
            record(|| offset2_ex(&subject, -0.5, 0.5, OffsetJoinType::Miter, 3.0));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::OFFSET2_EX),
                Event::Exit(ScopeId::OFFSET2_EX)
            ]
        );

        // ---- Phase 4: delegating wrappers inherit the primitive's scope -----
        // `difference_ex` -> `difference` -> `clip_polygons`; only the primitive
        // is marked, so exactly one enter/exit pair appears.
        let (difference_out, events) = record(|| difference_ex(&subject, &clip));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::CLIP_POLYGONS),
                Event::Exit(ScopeId::CLIP_POLYGONS)
            ]
        );

        // `opening_ex` -> `offset2_ex`.
        let (opening_out, events) =
            record(|| opening_ex(&subject, 0.5, OffsetJoinType::Miter, 3.0));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::OFFSET2_EX),
                Event::Exit(ScopeId::OFFSET2_EX)
            ]
        );

        // `union_ex` -> `union` -> `clip_polygons`.
        let (union_out, events) = record(|| union_ex(&subject));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::CLIP_POLYGONS),
                Event::Exit(ScopeId::CLIP_POLYGONS)
            ]
        );

        // `opening` and `closing_ex` reach `inflate_once` through `morph_pass`,
        // bypassing `offset` entirely — so each must carry its own mark, and
        // must emit exactly one pair despite calling `morph_pass` twice.
        // Regression guard: with the mark on `offset` alone these were silent.
        let (_, events) = record(|| opening(&subject, 0.5, OffsetJoinType::Miter));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::OPENING),
                Event::Exit(ScopeId::OPENING)
            ],
            "opening must emit exactly one pair, not one per morph_pass"
        );

        let (_, events) = record(|| closing_ex(&subject, 0.5, OffsetJoinType::Miter));
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::CLOSING_EX),
                Event::Exit(ScopeId::CLOSING_EX)
            ],
            "closing_ex must emit exactly one pair, not one per morph_pass"
        );

        // ---- Phase 5: installing a sink must not change any result ----------
        assert_eq!(
            (
                clip_out,
                offset_out,
                offset2_out,
                difference_out,
                opening_out,
                union_out
            ),
            uninstalled_results,
            "profiling marks must not perturb geometry"
        );

        // ---- Phase 6: manual nesting ----------------------------------------
        // Guards close in reverse creation order, so the event stream is a
        // well-formed bracket sequence.
        let user = ScopeId::new(ScopeId::USER_BASE);
        let ((), events) = record(|| {
            let _outer = scope(user);
            let _inner = scope(ScopeId::OFFSET);
            drop(_inner);
            let _sibling = scope(ScopeId::CLIP_POLYGONS);
        });
        assert_eq!(
            events,
            vec![
                Event::Enter(user),
                Event::Enter(ScopeId::OFFSET),
                Event::Exit(ScopeId::OFFSET),
                Event::Enter(ScopeId::CLIP_POLYGONS),
                Event::Exit(ScopeId::CLIP_POLYGONS),
                Event::Exit(user),
            ]
        );

        // ---- Phase 7: exit fires on an early return -------------------------
        let (bailed, events) = record(|| early_return_under_scope(user, true));
        assert_eq!(bailed, "bailed");
        assert_eq!(events, vec![Event::Enter(user), Event::Exit(user)]);

        let (fell_through, events) = record(|| early_return_under_scope(user, false));
        assert_eq!(fell_through, "fell through");
        assert_eq!(events, vec![Event::Enter(user), Event::Exit(user)]);

        // A guard dropped without being bound to a name still brackets nothing
        // but itself, and `ScopeGuard::id` reports what it will close.
        let ((), events) = record(|| {
            let guard = scope(ScopeId::OFFSET2_EX);
            assert_eq!(guard.id(), ScopeId::OFFSET2_EX);
        });
        assert_eq!(
            events,
            vec![
                Event::Enter(ScopeId::OFFSET2_EX),
                Event::Exit(ScopeId::OFFSET2_EX)
            ]
        );
    }

    /// Independent of install state, so it is safe to run concurrently with the
    /// lifecycle test above.
    #[test]
    fn core_scope_ids_are_distinct_named_and_below_user_base() {
        let mut seen = Vec::new();
        for id in ScopeId::CORE_SCOPES {
            assert!(
                id.raw() < ScopeId::USER_BASE,
                "core scope {id:?} must be reserved"
            );
            assert!(id.name().is_some(), "core scope {id:?} must be named");
            assert!(!seen.contains(id), "duplicate core scope {id:?}");
            seen.push(*id);
        }
        // Not a pinned count: the invariant is that `CORE_SCOPES` and `name()`
        // cannot drift apart. Every named reserved id must be listed, so adding
        // a constant and its name while forgetting `CORE_SCOPES` fails here.
        let named: Vec<ScopeId> = (0..ScopeId::USER_BASE)
            .map(ScopeId::new)
            .filter(|id| id.name().is_some())
            .collect();
        assert_eq!(
            seen, named,
            "CORE_SCOPES must list exactly the named reserved ids, in ascending order"
        );
        assert_eq!(
            ScopeId::CLIP_POLYGONS.name(),
            Some("polygon_ops::clip_polygons")
        );
        assert_eq!(ScopeId::new(ScopeId::USER_BASE).name(), None);
        assert_eq!(ScopeId::new(7).raw(), 7);
    }
}
