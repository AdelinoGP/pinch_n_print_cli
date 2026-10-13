//! PERF-T55-PROBE: selected real full-role argument/result capture only.
use super::*;

std::thread_local! {
    static ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) fn active() -> bool {
    ACTIVE.with(|v| v.get())
}

pub(super) fn begin(layer: usize, object: &str, region: &str) {
    // Targets are derived from the t53 census, not invented fixture layers.
    let selected = match object {
        "2928b983-cbab-510e-a064-062104f39dd2" => [0, 152, 43, 168].contains(&layer),
        "78620494-ecb8-5aca-8142-5d15997bb6e6" => [9, 21, 38, 232].contains(&layer),
        _ => false,
    };
    ACTIVE.with(|v| v.set(selected));
    if selected {
        host::log_warn(&format!(
            "PERF-T55-BEGIN {{\"layer\":{},\"object\":{:?},\"region\":{:?}}}",
            layer, object, region
        ));
    }
}

fn geometry(ex: &ExPolygon) -> String {
    let ring = |p: &Polygon| -> String {
        format!(
            "[{}]",
            p.points
                .iter()
                .map(|p| format!("[{},{}]", p.x, p.y))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    format!(
        "[{}]",
        std::iter::once(&ex.contour)
            .chain(ex.holes.iter())
            .map(ring)
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn polys(label: &str, role: usize, polys: &[ExPolygon]) {
    if !active() {
        return;
    }
    host::log_warn(&format!(
        "PERF-T55-SET {{\"label\":{:?},\"role\":{},\"count\":{}}}",
        label,
        role,
        polys.len()
    ));
    for (index, ex) in polys.iter().enumerate() {
        host::log_warn(&format!(
            "PERF-T55-DATA {{\"label\":{:?},\"role\":{},\"index\":{},\"polygon\":{}}}",
            label,
            role,
            index,
            geometry(ex)
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn args(
    segments: [&[Vec<Point3WithWidth>]; 4],
    areas: [&[ExPolygon]; 4],
    radius: f32,
    collision: &[ExPolygon],
    avg: usize,
    width: f32,
) {
    if !active() {
        return;
    }
    host::log_warn(&format!(
        "PERF-T55-ARGS {{\"radius_bits\":{},\"width_bits\":{},\"avg_nodes\":{}}}",
        radius.to_bits(),
        width.to_bits(),
        avg
    ));
    for role in 0..4 {
        polys("areas", role, areas[role]);
        host::log_warn(&format!(
            "PERF-T55-SEGMENTS {{\"role\":{},\"count\":{}}}",
            role,
            segments[role].len()
        ));
        for (index, segment) in segments[role].iter().enumerate() {
            let points = segment
                .iter()
                .map(|p| {
                    format!(
                        "[{},{},{},{},{},{},{},{}]",
                        p.x.to_bits(),
                        p.y.to_bits(),
                        p.z.to_bits(),
                        p.width.to_bits(),
                        p.flow_factor.to_bits(),
                        p.overhang_quartile.map(i64::from).unwrap_or(-1),
                        p.dist_to_top_mm.to_bits(),
                        p.overhang_distance_mm
                            .map(|v| i64::from(v.to_bits()))
                            .unwrap_or(-1)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            host::log_warn(&format!(
                "PERF-T55-SEGMENT {{\"role\":{},\"index\":{},\"points\":[{}]}}",
                role, index, points
            ));
        }
    }
    polys("collision", 0, collision);
}

pub(super) fn cut(index: usize, before: &[ExPolygon], after: &[ExPolygon], ran: bool) {
    if active() {
        host::log_warn(&format!(
            "PERF-T55-CUT {{\"index\":{},\"ran\":{},\"changed\":{}}}",
            index,
            ran,
            before != after
        ));
        polys(&format!("body_after_{}", index), 0, after);
    }
}

pub(super) fn finish(roles: &[slicer_ir::SupportPlanRoleRegion]) {
    if active() {
        host::log_warn(&format!("PERF-T55-RETURN {{\"count\":{}}}", roles.len()));
        for (index, role) in roles.iter().enumerate() {
            host::log_warn(&format!(
                "PERF-T55-ROLE {{\"index\":{},\"name\":\"{:?}\"}}",
                index, role.role
            ));
            polys("return", index, &role.regions);
        }
        host::log_warn("PERF-T55-END {}");
    }
    ACTIVE.with(|v| v.set(false));
}
