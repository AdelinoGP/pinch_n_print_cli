//! Independent territory model for `H-SUPPORT-TERRITORY`.
//!
//! `TerritoryClipper` (`crates/slicer-wasm-host/src/support_territory.rs`) has
//! two rules:
//!
//! * **Own territory** — a query naming an exact minted sub-region origin
//!   `(object, layer, region_id, ordered chain)` keeps `polygons ∩ own`.
//! * **Base region** — an origin with no territory entry keeps
//!   `polygons − inflate(foreign, clearance)`, and `polygons` unchanged when no
//!   foreign territory exists.
//!
//! The model builds axis-aligned rectangles on a millimetre lattice, so both
//! operations are exact rectilinear sets, and classifies the result per sample
//! point with `oracles::point_in_region`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_ir::slice_ir::SupportSourceKey;
use slicer_ir::{ExPolygon, Point2, Polygon, SupportAnalysisIR, SupportGeometryKey};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};
use slicer_wasm_host::support_territory::{TerritoryClipper, SUPPORT_TERRITORY_CLEARANCE_KEY};

/// Canonical units per millimetre.
const MM: i64 = 10_000;
/// The single fixture object.
const OBJECT: &str = "obj_territory";

/// An axis-aligned rectangle in canonical units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Lower-left X.
    pub x0: i64,
    /// Lower-left Y.
    pub y0: i64,
    /// Upper-right X.
    pub x1: i64,
    /// Upper-right Y.
    pub y1: i64,
}

impl Rect {
    fn expolygon(self) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: self.x0,
                        y: self.y0,
                    },
                    Point2 {
                        x: self.x1,
                        y: self.y0,
                    },
                    Point2 {
                        x: self.x1,
                        y: self.y1,
                    },
                    Point2 {
                        x: self.x0,
                        y: self.y1,
                    },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn inflated(self, by: i64) -> Self {
        Self {
            x0: self.x0 - by,
            y0: self.y0 - by,
            x1: self.x1 + by,
            y1: self.y1 + by,
        }
    }

    fn contains(self, x: i64, y: i64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

/// One declared territory entry.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// Minted sub-region id (the IR integer).
    pub region_id: u64,
    /// Owning family index.
    pub family: u8,
    /// Territory rectangle.
    pub rect: Rect,
}

/// One generated territory scenario.
#[derive(Debug, Clone)]
pub struct TerritoryCase {
    /// Declared territory entries at layer 0.
    pub entries: Vec<Entry>,
    /// The query polygons.
    pub body: Rect,
    /// Clearance in millimetres.
    pub clearance_mm: f32,
    /// Family index the querying side belongs to.
    pub family: u8,
    /// The queried region id.
    pub query_region: u64,
    /// Whether the query names a minted sub-region rather than a base region.
    pub sub_region_query: bool,
}

impl TerritoryCase {
    /// Build the committed analysis IR for this scenario.
    pub fn analysis(&self) -> SupportAnalysisIR {
        let mut support_territory: HashMap<SupportGeometryKey, Vec<ExPolygon>> = HashMap::new();
        let mut family_assignments: BTreeMap<SupportSourceKey, String> = BTreeMap::new();
        for entry in &self.entries {
            support_territory.insert(
                SupportGeometryKey {
                    global_support_layer_index: 0,
                    object_id: OBJECT.to_owned(),
                    region_id: entry.region_id,
                    variant_chain: Vec::new(),
                },
                vec![entry.rect.expolygon()],
            );
            family_assignments.insert(
                SupportSourceKey {
                    object_id: OBJECT.to_owned(),
                    region_id: entry.region_id,
                    variant_chain: Vec::new(),
                },
                format!("family_{}", entry.family),
            );
        }
        let mut shared_settings = BTreeMap::new();
        shared_settings.insert(
            SUPPORT_TERRITORY_CLEARANCE_KEY.to_owned(),
            format!("{}", self.clearance_mm),
        );
        SupportAnalysisIR {
            support_territory,
            family_assignments,
            shared_settings,
            ..SupportAnalysisIR::default()
        }
    }

    fn clearance_units(&self) -> i64 {
        (f64::from(self.clearance_mm) * MM as f64).round() as i64
    }

    /// Independent membership oracle for one sample point.
    fn expected_contains(&self, x: i64, y: i64) -> bool {
        if self.sub_region_query {
            if let Some(entry) = self
                .entries
                .iter()
                .find(|entry| entry.region_id == self.query_region)
            {
                return self.body.contains(x, y) && entry.rect.contains(x, y);
            }
        }
        if !self.body.contains(x, y) {
            return false;
        }
        let querying = format!("family_{}", self.family);
        let clearance = self.clearance_units();
        for entry in &self.entries {
            if format!("family_{}", entry.family) == querying {
                continue;
            }
            if entry.rect.inflated(clearance).contains(x, y) {
                return false;
            }
        }
        true
    }
}

/// Verify the real clipper against the model.
pub fn verify(case: &TerritoryCase) -> Result<(), String> {
    let analysis = case.analysis();
    let clipper = TerritoryClipper::from_ir(&analysis)
        .ok_or_else(|| String::from("a territory-bearing analysis must clip"))?;

    let region_id = if case.sub_region_query {
        case.query_region.to_string()
    } else {
        "0".to_owned()
    };
    let clipped = clipper
        .clip(
            OBJECT,
            0,
            &region_id,
            &[],
            &format!("family_{}", case.family),
            &[case.body.expolygon()],
        )
        .ok_or_else(|| String::from("a territory-bearing layer must clip"))?;

    let clearance = case.clearance_units();
    let mut xs: BTreeSet<i64> = BTreeSet::new();
    let mut ys: BTreeSet<i64> = BTreeSet::new();
    let rects: Vec<Rect> = std::iter::once(case.body)
        .chain(case.entries.iter().map(|entry| entry.rect))
        .chain(
            case.entries
                .iter()
                .map(|entry| entry.rect.inflated(clearance)),
        )
        .collect();
    // Every fixture edge is a multiple of 1000 units (whole millimetres plus a
    // 0.2 or 0.4 mm clearance), so a 2500-unit offset always lands strictly
    // between edges and can never sit on a boundary.
    for rect in rects {
        for x in [rect.x0, rect.x1] {
            xs.insert(x - 2500);
            xs.insert(x + 2500);
        }
        for y in [rect.y0, rect.y1] {
            ys.insert(y - 2500);
            ys.insert(y + 2500);
        }
    }
    let xs: Vec<i64> = xs.into_iter().collect();
    let ys: Vec<i64> = ys.into_iter().collect();
    let mut checked = 0usize;
    for x in &xs {
        for y in &ys {
            let (x, y) = (*x, *y);
            let point = Point2 { x, y };
            let inside = match point_in_region(point, &clipped) {
                Ok(PointLocation::Inside) => true,
                Ok(PointLocation::Outside) => false,
                Ok(PointLocation::Boundary) => {
                    return Err(format!("sample {point:?} landed on a boundary"))
                }
                Err(error) => return Err(format!("point_in_region failed: {error:?}")),
            };
            let want = case.expected_contains(x, y);
            if inside != want {
                return Err(format!("sample {point:?} inside={inside}, expected {want}"));
            }
            checked += 1;
        }
    }
    if checked == 0 {
        return Err(String::from("the sample grid produced no points"));
    }
    Ok(())
}

/// `from_ir(None)` is not an API: no territory means no clipper. A layer with
/// no territory is a no-clipping outcome, and a distinct region id must not
/// match a minted entry.
pub fn absence_modes() -> Result<(), String> {
    if TerritoryClipper::from_ir(&SupportAnalysisIR::default()).is_some() {
        return Err(String::from(
            "an IR with no territory must not produce a clipper",
        ));
    }

    let case = TerritoryCase {
        entries: vec![Entry {
            region_id: 7,
            family: 1,
            rect: Rect {
                x0: MM,
                y0: MM,
                x1: 3 * MM,
                y1: 3 * MM,
            },
        }],
        body: Rect {
            x0: 0,
            y0: 0,
            x1: 4 * MM,
            y1: 4 * MM,
        },
        clearance_mm: 0.4,
        family: 0,
        query_region: 7,
        sub_region_query: true,
    };
    let analysis = case.analysis();
    let clipper = TerritoryClipper::from_ir(&analysis)
        .ok_or_else(|| String::from("the fixture must produce a clipper"))?;

    // A layer the object carries no territory on is a no-clipping outcome.
    if clipper.clip(OBJECT, 5, "0", &[], "family_0", &[]).is_some() {
        return Err(String::from("a layer with no territory must clip to None"));
    }

    // A distinct minted id must not match the entry; the base rule applies.
    verify(&TerritoryCase {
        query_region: 8,
        ..case.clone()
    })?;
    Ok(())
}

/// Strategy for one generated territory scenario.
pub fn strategy() -> impl Strategy<Value = TerritoryCase> {
    let rect = (0i64..3, 0i64..3, 1i64..3, 1i64..3).prop_map(|(x, y, w, h)| Rect {
        x0: x * MM,
        y0: y * MM,
        x1: (x + w) * MM,
        y1: (y + h) * MM,
    });
    (
        proptest::collection::vec(rect.clone(), 1..=3),
        rect,
        proptest::sample::select(vec![0.0f32, 0.2, 0.4]),
        any::<bool>(),
    )
        .prop_map(|(rects, body, clearance_mm, sub_region_query)| {
            let entries: Vec<Entry> = rects
                .into_iter()
                .enumerate()
                .map(|(index, rect)| Entry {
                    region_id: (index as u64) + 1,
                    family: (index % 2) as u8,
                    rect,
                })
                .collect();
            let family = entries[0].family;
            let query_region = entries[0].region_id;
            TerritoryCase {
                entries,
                body,
                clearance_mm,
                family,
                query_region,
                sub_region_query,
            }
        })
}

/// Registry entry point for `H-SUPPORT-TERRITORY`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new(
        "H-SUPPORT-TERRITORY",
        "proptest_support_territory_clearance",
    )
    .map_err(|error| error.to_string())?;
    for bound in [
        "max_objects",
        "max_regions",
        "max_abs_units",
        "max_clearance_units",
        "max_cells",
    ] {
        run.bound(bound).map_err(|error| error.to_string())?;
    }

    absence_modes()?;

    // Own-territory witness: the body keeps the intersection.
    verify(&TerritoryCase {
        entries: vec![Entry {
            region_id: 7,
            family: 0,
            rect: Rect {
                x0: MM,
                y0: MM,
                x1: 3 * MM,
                y1: 3 * MM,
            },
        }],
        body: Rect {
            x0: 0,
            y0: 0,
            x1: 4 * MM,
            y1: 4 * MM,
        },
        clearance_mm: 0.4,
        family: 0,
        query_region: 7,
        sub_region_query: true,
    })?;

    // Foreign-territory witness: the body loses the inflated foreign rect.
    verify(&TerritoryCase {
        entries: vec![Entry {
            region_id: 7,
            family: 1,
            rect: Rect {
                x0: MM,
                y0: MM,
                x1: 3 * MM,
                y1: 3 * MM,
            },
        }],
        body: Rect {
            x0: 0,
            y0: 0,
            x1: 4 * MM,
            y1: 4 * MM,
        },
        clearance_mm: 0.4,
        family: 0,
        query_region: 0,
        sub_region_query: false,
    })?;

    let generated = strategy();
    run.check(&generated, |case: TerritoryCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
