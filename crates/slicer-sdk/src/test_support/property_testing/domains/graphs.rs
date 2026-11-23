//! Construct graph invariants rather than filtering arbitrary graphs.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

use super::{DomainError, GraphKind, MalformedGraphKind, SmallGraph};

const MAX_NODES: u16 = 32;

fn check_max_nodes(max_nodes: u16) -> Result<(), DomainError> {
    if !(1..=MAX_NODES).contains(&max_nodes) {
        Err(DomainError::InvalidBounds(
            "graph limit must be 1..=32".into(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn graphs(
    kind: GraphKind,
    max_nodes: u16,
) -> Result<BoxedStrategy<SmallGraph>, DomainError> {
    check_max_nodes(max_nodes)?;
    match kind {
        GraphKind::Dag => Ok((
            0_u16..=max_nodes,
            0_u16..=(u16::MAX - max_nodes),
            proptest::collection::vec((any::<u16>(), any::<u16>()), 0..=usize::from(max_nodes) * 2),
        )
            .prop_map(|(count, offset, raw_edges)| {
                let node_ids: Vec<_> = (0..count).map(|i| offset + i).collect();
                let mut edges = Vec::new();
                if count > 1 {
                    for (a, b) in raw_edges {
                        let a = a % count;
                        let b = b % count;
                        if a != b {
                            edges.push((offset + a.min(b), offset + a.max(b)));
                        }
                    }
                    edges.sort_unstable();
                    edges.dedup();
                }
                SmallGraph { node_ids, edges }
            })
            .boxed()),
        GraphKind::CycleWithTail => {
            if max_nodes < 3 {
                return Err(DomainError::InvalidBounds(
                    "cycle and tail require at least three nodes".into(),
                ));
            }
            Ok((2_u16..max_nodes, 1_u16..max_nodes)
                .prop_map(move |(cycle_len, raw_tail_len)| {
                    let tail_len = raw_tail_len.min(max_nodes - cycle_len);
                    let node_ids = (0..cycle_len + tail_len).collect();
                    let mut edges: Vec<_> =
                        (0..cycle_len).map(|i| (i, (i + 1) % cycle_len)).collect();
                    edges.extend((cycle_len - 1..cycle_len + tail_len - 1).map(|i| (i, i + 1)));
                    SmallGraph { node_ids, edges }
                })
                .boxed())
        }
    }
}

pub(super) fn malformed_graphs(
    kind: MalformedGraphKind,
    max_nodes: u16,
) -> Result<BoxedStrategy<SmallGraph>, DomainError> {
    check_max_nodes(max_nodes)?;
    match kind {
        MalformedGraphKind::DuplicateNode => {
            if max_nodes < 2 {
                return Err(DomainError::InvalidBounds(
                    "duplicate witness needs two node entries".into(),
                ));
            }
            Ok((1_u16..max_nodes)
                .prop_map(|count| {
                    let mut node_ids: Vec<_> = (0..count).collect();
                    node_ids.push(0);
                    SmallGraph {
                        node_ids,
                        edges: Vec::new(),
                    }
                })
                .boxed())
        }
        MalformedGraphKind::UnknownEndpoint => Ok((1_u16..=max_nodes)
            .prop_map(move |count| SmallGraph {
                node_ids: (0..count).collect(),
                edges: vec![(0, max_nodes)],
            })
            .boxed()),
    }
}
