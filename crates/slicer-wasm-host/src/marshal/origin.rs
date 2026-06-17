use std::fmt;

// ---------------------------------------------------------------------------
// OriginId
// ---------------------------------------------------------------------------

/// Identifies the mesh object and region that produced a particular WIT output payload.
///
/// Used as the grouping key inside [`OriginBucket`] to route per-payload outputs
/// back to the correct region accumulator when a stage runs in tagged (multi-region) mode.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OriginId {
    /// The object identifier string emitted by the WASM guest (matches `MeshObjectView.id`).
    pub object_id: String,
    /// The region index emitted by the WASM guest for this payload.
    pub region_id: u64,
}

// ---------------------------------------------------------------------------
// MarshalError
// ---------------------------------------------------------------------------

/// Errors produced by marshal helpers when converting WIT guest output to host IR.
#[derive(Debug, PartialEq, Eq)]
pub enum MarshalError {
    /// A payload at `index` carried no origin tag while the bucket is in tagged mode.
    UntaggedPayload {
        /// The collection name used for diagnostics (e.g. `"perimeter_paths"`).
        kind: &'static str,
        /// Zero-based index of the offending payload in the collection.
        index: usize,
    },
    /// The `origins` slice and the `payloads` vec have different lengths.
    OriginLengthMismatch {
        /// The collection name used for diagnostics.
        kind: &'static str,
        /// Number of origin entries supplied.
        origins: usize,
        /// Number of payload entries supplied.
        payloads: usize,
    },
    /// A floating-point value that must be finite was NaN or ±infinity.
    NonFiniteFloat {
        /// Field name used for diagnostics (e.g. `"width"`).
        field: &'static str,
        /// Zero-based index of the offending entry.
        index: usize,
    },
}

impl fmt::Display for MarshalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarshalError::UntaggedPayload { kind, index } => {
                write!(
                    f,
                    "untagged payload in tagged mode: kind={kind}, index={index}"
                )
            }
            MarshalError::OriginLengthMismatch {
                kind,
                origins,
                payloads,
            } => {
                write!(
                    f,
                    "origin/payload length mismatch: kind={kind}, origins={origins}, payloads={payloads}"
                )
            }
            MarshalError::NonFiniteFloat { field, index } => {
                write!(f, "non-finite float: field={field}, index={index}")
            }
        }
    }
}

impl From<MarshalError> for String {
    fn from(e: MarshalError) -> String {
        e.to_string()
    }
}

// ---------------------------------------------------------------------------
// OriginBucket
// ---------------------------------------------------------------------------

/// Accumulates WIT output payloads into per-origin region accumulators.
///
/// In *tagged* mode (when `any_tagged` is `true`) each payload carries an [`OriginId`]
/// and is routed to the matching accumulator, creating a new one on first sight.
/// In *anonymous* mode all payloads collapse into a single accumulator.
pub struct OriginBucket<R> {
    tagged: bool,
    regions: Vec<(OriginId, R)>,
    mint: fn(&OriginId) -> R,
}

impl<R> OriginBucket<R> {
    /// Creates a new bucket.
    ///
    /// `any_tagged` — set to `true` when at least one region in the dispatch carries an
    /// explicit [`OriginId`] (i.e. the stage was invoked with per-region tagging).
    /// `mint` — factory function that constructs a fresh accumulator `R` for a new origin.
    pub fn new(any_tagged: bool, mint: fn(&OriginId) -> R) -> Self {
        if any_tagged {
            OriginBucket {
                tagged: true,
                regions: Vec::new(),
                mint,
            }
        } else {
            let anon_id = OriginId {
                object_id: String::new(),
                region_id: 0,
            };
            let region = mint(&anon_id);
            OriginBucket {
                tagged: false,
                regions: vec![(anon_id, region)],
                mint,
            }
        }
    }

    /// Routes each element of `payloads` into the accumulator identified by the
    /// corresponding `origins` entry, then invokes `place` to insert the payload.
    ///
    /// Returns [`MarshalError::OriginLengthMismatch`] when `payloads.len() != origins.len()`
    /// in tagged mode, or [`MarshalError::UntaggedPayload`] when an origin slot is `None`
    /// in tagged mode.  Anonymous mode ignores `origins` and accepts any length.
    pub fn drain<T>(
        &mut self,
        kind: &'static str,
        payloads: Vec<T>,
        origins: &[Option<OriginId>],
        mut place: impl FnMut(&mut R, T),
    ) -> Result<(), MarshalError> {
        if self.tagged {
            // Check lengths first, before any mutation.
            if origins.len() != payloads.len() {
                return Err(MarshalError::OriginLengthMismatch {
                    kind,
                    origins: origins.len(),
                    payloads: payloads.len(),
                });
            }

            for (index, (payload, origin_opt)) in
                payloads.into_iter().zip(origins.iter()).enumerate()
            {
                let origin = match origin_opt {
                    Some(o) => o,
                    None => return Err(MarshalError::UntaggedPayload { kind, index }),
                };

                // First-seen lookup via Vec::iter().position(...)
                let pos = self.regions.iter().position(|(id, _)| id == origin);
                let idx = match pos {
                    Some(i) => i,
                    None => {
                        let new_region = (self.mint)(origin);
                        self.regions.push((origin.clone(), new_region));
                        self.regions.len() - 1
                    }
                };

                place(&mut self.regions[idx].1, payload);
            }
        } else {
            // Anonymous mode: push everything into the single region.
            let region = &mut self.regions[0].1;
            for payload in payloads {
                place(region, payload);
            }
        }

        Ok(())
    }

    /// Consumes the bucket and returns the accumulated regions in first-seen origin order.
    pub fn into_regions(self) -> Vec<R> {
        self.regions.into_iter().map(|(_, r)| r).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mint_vec(_: &OriginId) -> Vec<i32> {
        Vec::new()
    }

    fn make_id(obj: &str, region: u64) -> OriginId {
        OriginId {
            object_id: obj.to_string(),
            region_id: region,
        }
    }

    #[test]
    fn buckets_by_first_seen_origin_order() {
        let mut bucket: OriginBucket<Vec<i32>> = OriginBucket::new(true, mint_vec);

        let id_a = make_id("A", 1);
        let id_b = make_id("B", 2);

        // Payloads: 10 -> A, 20 -> B, 30 -> A  (order: A first, then B)
        let payloads = vec![10i32, 20, 30];
        let origins = vec![Some(id_a.clone()), Some(id_b.clone()), Some(id_a.clone())];

        bucket
            .drain("test", payloads, &origins, |region, p| region.push(p))
            .expect("drain must succeed");

        let regions = bucket.into_regions();
        assert_eq!(regions.len(), 2, "must have exactly 2 regions");
        // A is first-seen → index 0, B → index 1
        assert_eq!(
            regions[0],
            vec![10, 30],
            "A's region must contain 1st and 3rd payloads"
        );
        assert_eq!(regions[1], vec![20], "B's region must contain 2nd payload");
    }

    #[test]
    fn untagged_payload_in_tagged_mode_errs() {
        let mut bucket: OriginBucket<Vec<i32>> = OriginBucket::new(true, mint_vec);

        let id_a = make_id("A", 1);
        let payloads = vec![1i32, 2, 3];
        let origins = vec![Some(id_a.clone()), None, Some(id_a.clone())];

        let result = bucket.drain("test", payloads, &origins, |region, p| region.push(p));

        assert_eq!(
            result,
            Err(MarshalError::UntaggedPayload {
                kind: "test",
                index: 1
            }),
            "must error on None origin at index 1"
        );
    }

    #[test]
    fn anonymous_mode_collapses_to_one_region() {
        let mut bucket: OriginBucket<Vec<i32>> = OriginBucket::new(false, mint_vec);

        // Origins slice is ignored in anonymous mode; pass empty or arbitrary.
        let payloads = vec![1i32, 2, 3, 4];
        let origins: Vec<Option<OriginId>> = vec![];

        bucket
            .drain("test", payloads, &origins, |region, p| region.push(p))
            .expect("anonymous drain must not error");

        let regions = bucket.into_regions();
        assert_eq!(
            regions.len(),
            1,
            "anonymous mode must produce exactly one region"
        );
        assert_eq!(
            regions[0],
            vec![1, 2, 3, 4],
            "all payloads must be in the single region"
        );
    }

    /// Regression guard for the AC-6 bug introduced in packet 113 Step 5.
    ///
    /// Three independent `OriginBucket` instances each computed their OWN
    /// first-seen order, so `interface_paths` with origins [B, A] would emit
    /// [B, A] even when A was first-seen globally in `support_paths`.
    ///
    /// This test asserts that draining two collections into the SAME bucket
    /// fixes the ordering: the first drain establishes [A, B]; the second drain
    /// with [B, A] must still emit in the shared order [A, B].
    #[test]
    fn shared_bucket_preserves_first_seen_across_drains() {
        struct TwoVecs {
            col1: Vec<i32>,
            col2: Vec<i32>,
        }

        fn mint(_: &OriginId) -> TwoVecs {
            TwoVecs {
                col1: Vec::new(),
                col2: Vec::new(),
            }
        }

        let id_a = make_id("A", 1);
        let id_b = make_id("B", 2);

        let mut bucket: OriginBucket<TwoVecs> = OriginBucket::new(true, mint);

        // Collection 1: origins [A, B] — A is first-seen globally.
        let col1_payloads = vec![10i32, 20];
        let col1_origins = vec![Some(id_a.clone()), Some(id_b.clone())];
        bucket
            .drain("col1", col1_payloads, &col1_origins, |r, p| r.col1.push(p))
            .expect("col1 drain must succeed");

        // Collection 2: origins [B, A] — reversed from global order.
        // Must still emit in first-seen order [A, B], not [B, A].
        let col2_payloads = vec![30i32, 40];
        let col2_origins = vec![Some(id_b.clone()), Some(id_a.clone())];
        bucket
            .drain("col2", col2_payloads, &col2_origins, |r, p| r.col2.push(p))
            .expect("col2 drain must succeed");

        let regions = bucket.into_regions();

        // Shared order must be [A, B] (first-seen in col1).
        assert_eq!(regions.len(), 2, "must have exactly 2 regions");

        // Region 0 = A: col1 has [10], col2 has [40] (A's payload from col2).
        assert_eq!(
            regions[0].col1,
            vec![10],
            "region A col1 must contain A's payload from col1"
        );
        assert_eq!(
            regions[0].col2,
            vec![40],
            "region A col2 must contain A's payload from col2 (order fixed by first global appearance)"
        );

        // Region 1 = B: col1 has [20], col2 has [30] (B's payload from col2).
        assert_eq!(
            regions[1].col1,
            vec![20],
            "region B col1 must contain B's payload from col1"
        );
        assert_eq!(
            regions[1].col2,
            vec![30],
            "region B col2 must contain B's payload from col2"
        );
    }

    #[test]
    fn length_mismatch_errs() {
        let mut bucket: OriginBucket<Vec<i32>> = OriginBucket::new(true, mint_vec);

        let id_a = make_id("A", 1);
        let payloads = vec![1i32, 2, 3];
        // origins has 2 entries but payloads has 3
        let origins = vec![Some(id_a.clone()), Some(id_a.clone())];

        let result = bucket.drain("test", payloads, &origins, |region, p| region.push(p));

        assert_eq!(
            result,
            Err(MarshalError::OriginLengthMismatch {
                kind: "test",
                origins: 2,
                payloads: 3
            }),
            "must error on length mismatch"
        );
        // No region mutation happened (error returned before any placement).
    }
}
