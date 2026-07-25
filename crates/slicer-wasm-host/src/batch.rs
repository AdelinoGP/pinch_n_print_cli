//! Fan-out policy for the batched host services in `slicer:common/host-services`.
//!
//! A batched import hands the host a whole list of independent requests in one
//! call. The guest is blocked for the duration, so nothing re-enters the host
//! concurrently — only the *algorithm* behind each request has to be thread
//! safe, not the call path. That is the whole reason batching is cheap where
//! guest-internal threading is not (ADR-0049).
//!
//! # Ordering
//!
//! Results are always returned in input order. `rayon`'s indexed
//! `par_iter().map(..).collect()` preserves it, so the i-th result belongs to
//! the i-th request whether the batch ran serially or in parallel. Output
//! cannot depend on worker count or scheduling.
//!
//! # When to fan out
//!
//! On estimated **work**, not item count. Per-item cost across the batched
//! services spans about three orders of magnitude — one `offset-polygons` over
//! a benchy support outline measured around 7 ms, while one `raycast-z-down` is
//! a single ray. A threshold in items would either serialize batches worth
//! parallelizing or fan out batches that are pure overhead.
//!
//! Each service supplies a cost function in a common unit: **input geometric
//! primitives touched** — polygon vertices for the clipper-backed services, mesh
//! triangles for the mesh-query services, since that is what each one's inner
//! loop actually walks.

use rayon::prelude::*;

/// Estimated primitive count at or above which a batch is fanned out.
///
/// Not measured per service — chosen from the shape of the costs involved.
/// Rayon's per-task overhead is on the order of a microsecond; primitive-level
/// work in these services is tens of nanoseconds upward. At this threshold the
/// scheduling cost is well under a percent of the estimated work, with margin
/// for the cheapest primitive. It is deliberately a single constant: the cost
/// functions do the per-service normalizing, so the threshold does not have to.
pub const FANOUT_WORK_THRESHOLD: u64 = 4_096;

/// Whether a batch ran serially or fanned out. Returned so callers can record
/// it and tests can assert both paths agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchMode {
    /// Ran on the calling thread.
    Serial,
    /// Fanned out over the rayon pool.
    Parallel,
}

impl BatchMode {
    /// Pick a mode from the batch's estimated work.
    ///
    /// A single-item batch is always serial: there is nothing to overlap, and
    /// fanning out one task is pure overhead regardless of how large it is.
    pub fn for_work(item_count: usize, estimated_work: u64) -> Self {
        if item_count < 2 || estimated_work < FANOUT_WORK_THRESHOLD {
            Self::Serial
        } else {
            Self::Parallel
        }
    }
}

/// Map `f` over `requests`, fanning out when `mode` says so.
///
/// Split out from [`map_batch`] so tests can drive both paths over identical
/// input and byte-compare the results.
pub fn map_batch_with_mode<T, R, F>(requests: &[T], mode: BatchMode, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    match mode {
        BatchMode::Serial => requests.iter().map(f).collect(),
        BatchMode::Parallel => requests.par_iter().map(f).collect(),
    }
}

/// Map `f` over `requests`, choosing serial or parallel from `cost`.
///
/// `cost` is evaluated on the calling thread before any fan-out, so it may
/// borrow host state (for example to size a mesh query by triangle count).
pub fn map_batch<T, R, C, F>(requests: &[T], cost: C, f: F) -> (Vec<R>, BatchMode)
where
    T: Sync,
    R: Send,
    C: Fn(&T) -> u64,
    F: Fn(&T) -> R + Sync + Send,
{
    let estimated_work: u64 = requests.iter().map(cost).sum();
    let mode = BatchMode::for_work(requests.len(), estimated_work);
    (map_batch_with_mode(requests, mode, f), mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_item_batches_never_fan_out() {
        assert_eq!(BatchMode::for_work(1, u64::MAX), BatchMode::Serial);
        assert_eq!(BatchMode::for_work(0, u64::MAX), BatchMode::Serial);
    }

    #[test]
    fn threshold_is_on_work_not_item_count() {
        // Many trivial items stay serial...
        assert_eq!(BatchMode::for_work(10_000, 10), BatchMode::Serial);
        // ...while a couple of heavy ones fan out.
        assert_eq!(
            BatchMode::for_work(2, FANOUT_WORK_THRESHOLD),
            BatchMode::Parallel
        );
    }

    #[test]
    fn both_modes_produce_identical_results_in_input_order() {
        let requests: Vec<u64> = (0..500).collect();
        let f = |n: &u64| n.wrapping_mul(2_654_435_761);

        let serial = map_batch_with_mode(&requests, BatchMode::Serial, f);
        let parallel = map_batch_with_mode(&requests, BatchMode::Parallel, f);

        assert_eq!(serial, parallel);
        assert_eq!(serial.len(), requests.len());
        assert_eq!(serial[0], f(&0));
        assert_eq!(serial[499], f(&499));
    }
}
