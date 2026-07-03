// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/BeadingStrategyFactory.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `BeadingStrategyFactory`: assembles the full Arachne beading-strategy
//! decorator stack.
//!
//! Composes the five strategies added in earlier steps of this packet, in
//! the canonical order (innermost/base first): `Distributed → Redistribute →
//! [Widening] → [OuterWallInset] → Limited`, with both middle layers
//! conditional (see below). `Distributed` has no parent; each subsequent
//! strategy wraps the previous one; `Limited` is the outermost decorator and
//! is the type the call site actually holds and dispatches against — calls
//! flow inward through each layer to `Distributed`.
//!
//! `OuterWallInsetBeadingStrategy` wrapping faithfully matches upstream's
//! `BeadingStrategyFactory::makeStrategy` gate (`BeadingStrategyFactory.cpp:
//! 50-97`): it is wrapped **only when `outer_wall_offset != 0.0`**; when it
//! is exactly `0.0` (the default), the layer is literally absent from the
//! composition chain — not merely a runtime no-op.
//!
//! `WideningBeadingStrategy` wrapping likewise now matches upstream: it is
//! wrapped **only when `print_thin_walls` (the `detect_thin_wall` config key)
//! is `true`**; when `false` (the default), it too is literally absent from
//! the composition chain. With both optional layers absent, `type_chain()`
//! reads `"Limited(Redistribute(Distributed))"` (three layers).
//!
//! The `preferred_bead_width_outer`/`preferred_bead_width_inner` split is
//! also now implemented (`BeadingStrategyFactory.cpp:50-97`): `optimal_width`
//! serves as upstream's `preferred_bead_width_inner`, and the new
//! `preferred_bead_width_outer` field supplies upstream's
//! `preferred_bead_width_outer`. `DistributedBeadingStrategy`'s (and, when
//! present, `WideningBeadingStrategy`'s) base width is *conditionally*
//! selected — `preferred_bead_width_outer` when `max_bead_count <= 2`, else
//! `optimal_width` — via `effective_optimal_width` below.
//! `RedistributeBeadingStrategy`'s `optimal_width_outer` parameter always
//! uses `preferred_bead_width_outer`, unconditionally.
//!
//! All values are in slicer units (1 unit = 100 nm) — see
//! `docs/08_coordinate_system.md`.

use serde::{Deserialize, Serialize};

use super::distributed::DistributedBeadingStrategy;
use super::limited::LimitedBeadingStrategy;
use super::outer_wall_inset::OuterWallInsetBeadingStrategy;
use super::redistribute::RedistributeBeadingStrategy;
use super::widening::WideningBeadingStrategy;
use super::BeadingStrategy;

/// Bundles the parameters each of the five composed strategies' constructors
/// need to build the full Arachne beading-strategy stack via
/// [`BeadingStrategyFactory::create_stack`].
///
/// All width/length fields are in slicer units (1 unit = 100 nm).
///
/// Derives `Serialize`/`Deserialize` directly: `serde` is now a real
/// dependency of `slicer-core` (promoted from `[dev-dependencies]`), so the
/// JSON golden fixture in `tests/beading/factory.rs` deserializes straight
/// into this type — no local mirror struct is needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeadingFactoryParams {
    /// The bead width used when there is no surplus/deficit to redistribute
    /// (`DistributedBeadingStrategy`), and the width forced onto the
    /// outermost/innermost beads (`RedistributeBeadingStrategy`).
    pub optimal_width: f64,
    /// Reserved transition-ramp-length parameter passed to
    /// `DistributedBeadingStrategy::new`; not read by its `compute`.
    pub default_transition_length: f64,
    /// Reserved transition-filter-distance parameter passed to
    /// `DistributedBeadingStrategy::new`; not read by its `compute`.
    pub transition_filter_dist: f64,
    /// Gaussian decay radius (bead-count units) for
    /// `DistributedBeadingStrategy`.
    pub distribution_count: usize,
    /// Threshold below which `WideningBeadingStrategy` reports a single bead
    /// instead of delegating (or nothing at all, below this).
    pub min_input_width: f64,
    /// Minimum bead width `WideningBeadingStrategy` clamps its emitted bead
    /// up to, in the `min_input_width <= thickness < optimal_width` regime.
    pub min_output_width: f64,
    /// Inward offset `OuterWallInsetBeadingStrategy` applies to the outer
    /// wall's toolpath location. Wrapped into the stack only when nonzero
    /// (see the module doc comment) — `0.0` means the layer is absent
    /// entirely, not merely a no-op.
    pub outer_wall_offset: f64,
    /// The maximum number of (non-sentinel) beads `LimitedBeadingStrategy`
    /// will ever request from its parent.
    pub max_bead_count: usize,
    /// Below `minimum_variable_line_ratio * optimal_width`, total thickness
    /// produces no beads at all (`RedistributeBeadingStrategy`). Internal
    /// Arachne parameter with no corresponding registered config key in this
    /// packet's T-218 scope (like `default_transition_length` /
    /// `transition_filter_dist` above), matching upstream OrcaSlicer's
    /// factory default `minimum_variable_line_width = 0.5`.
    pub minimum_variable_line_ratio: f64,
    /// Gates whether `WideningBeadingStrategy` is wrapped into the stack at
    /// all. Named after the internal Arachne/C++ parameter
    /// (`BeadingStrategyFactory::makeStrategy`'s `print_thin_walls`); maps to
    /// the `detect_thin_wall` config key.
    pub print_thin_walls: bool,
    /// Target width (slicer units) for the outermost/innermost bead. Used
    /// unconditionally as `RedistributeBeadingStrategy`'s
    /// `optimal_width_outer` parameter, and conditionally (when
    /// `max_bead_count <= 2`) as `DistributedBeadingStrategy`'s/
    /// `WideningBeadingStrategy`'s base width instead of `optimal_width`.
    /// Maps to the `preferred_bead_width_outer` config key.
    pub preferred_bead_width_outer: f64,
}

impl Default for BeadingFactoryParams {
    /// Default parameters. These now mirror this packet's own registered
    /// `docs/15_config_keys_reference.md` defaults exactly: `optimal_width` =
    /// `optimal_width` (4000), `min_input_width` = `min_feature_size` (1000),
    /// `min_output_width` = `min_bead_width` (4000), `distribution_count` =
    /// `wall_distribution_count` (1), `default_transition_length` =
    /// `wall_transition_length` (4000), `transition_filter_dist` =
    /// `wall_transition_filter_deviation` (1000), `outer_wall_offset` =
    /// `outer_wall_offset` (0, meaning `OuterWallInsetBeadingStrategy` is
    /// correctly ABSENT by default — see the module doc comment),
    /// `max_bead_count` = `max_bead_count` (9). Previously these values
    /// silently diverged from the registered config defaults (a real,
    /// confirmed bug from a prior review); this is a bug fix, not an
    /// intentional behavior change. `minimum_variable_line_ratio` (0.5) has
    /// no registered config key — see its own field doc. `print_thin_walls` =
    /// `detect_thin_wall` (`false`) and `preferred_bead_width_outer` =
    /// `preferred_bead_width_outer` (4000) mirror those two keys' registered
    /// defaults exactly.
    fn default() -> Self {
        Self {
            optimal_width: 4000.0,
            default_transition_length: 4000.0,
            transition_filter_dist: 1000.0,
            distribution_count: 1,
            min_input_width: 1000.0,
            min_output_width: 4000.0,
            outer_wall_offset: 0.0,
            max_bead_count: 9,
            minimum_variable_line_ratio: 0.5,
            print_thin_walls: false,
            preferred_bead_width_outer: 4000.0,
        }
    }
}

/// Assembles the full Arachne beading-strategy decorator stack.
pub struct BeadingStrategyFactory;

impl BeadingStrategyFactory {
    /// Builds the canonical Arachne beading-strategy stack: `Distributed`
    /// (base) wrapped by `Redistribute`, then — only when
    /// `params.print_thin_walls` — `Widening`, then — only when
    /// `params.outer_wall_offset != 0.0` — `OuterWallInset`, then `Limited`
    /// (outermost — the returned trait object). See the module doc comment
    /// for why both `Widening` and `OuterWallInset` are conditional, and for
    /// `effective_optimal_width`'s `max_bead_count <= 2` selection.
    pub fn create_stack(params: &BeadingFactoryParams) -> Box<dyn BeadingStrategy> {
        // Upstream's `preferred_bead_width_outer`/`preferred_bead_width_inner`
        // split (`BeadingStrategyFactory.cpp:50-97`): the base width fed to
        // `DistributedBeadingStrategy` (and, when present,
        // `WideningBeadingStrategy`) is `preferred_bead_width_outer` when
        // `max_bead_count <= 2`, else `optimal_width` (upstream's
        // `preferred_bead_width_inner`).
        let effective_optimal_width = if params.max_bead_count <= 2 {
            params.preferred_bead_width_outer
        } else {
            params.optimal_width
        };

        let distributed: Box<dyn BeadingStrategy> = Box::new(DistributedBeadingStrategy::new(
            effective_optimal_width,
            params.default_transition_length,
            params.transition_filter_dist,
            params.distribution_count,
        ));

        let redistribute: Box<dyn BeadingStrategy> = Box::new(RedistributeBeadingStrategy::new(
            distributed,
            params.preferred_bead_width_outer,
            params.minimum_variable_line_ratio,
        ));

        let pre_outer_wall: Box<dyn BeadingStrategy> = if params.print_thin_walls {
            Box::new(WideningBeadingStrategy::new(
                redistribute,
                effective_optimal_width,
                params.min_input_width,
                params.min_output_width,
            ))
        } else {
            redistribute
        };

        let pre_limited: Box<dyn BeadingStrategy> = if params.outer_wall_offset != 0.0 {
            Box::new(OuterWallInsetBeadingStrategy::new(
                pre_outer_wall,
                params.outer_wall_offset,
            ))
        } else {
            pre_outer_wall
        };

        Box::new(LimitedBeadingStrategy::new(
            pre_limited,
            params.max_bead_count,
        ))
    }
}
