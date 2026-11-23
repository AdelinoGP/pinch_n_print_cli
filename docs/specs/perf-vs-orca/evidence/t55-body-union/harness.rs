//! Exact recorded full-role replay and isolated native-union internals diagnosis.
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;
use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon, SupportPlanRole, SupportPlanRoleRegion};
pub use slicer_core::profile;
mod measured_polygon_ops;

struct Numbers<R: BufRead>(R);
impl<R: BufRead> Numbers<R> {
    fn next(&mut self) -> i64 {
        let mut token=Vec::new();
        loop {
            let buffer=self.0.fill_buf().expect("specimen read");
            assert!(!buffer.is_empty(),"truncated exact specimen");
            let ch=buffer[0];self.0.consume(1);
            if ch.is_ascii_whitespace() { if !token.is_empty() {break;} } else { token.push(ch); }
        }
        std::str::from_utf8(&token).unwrap().parse().expect("integer/bits token")
    }
    fn end(&mut self) {
        loop {
            let buffer=self.0.fill_buf().unwrap();
            if buffer.is_empty() {break;}
            assert!(buffer.iter().all(u8::is_ascii_whitespace),"extra specimen data");
            let len=buffer.len();self.0.consume(len);
        }
    }
    fn polys(&mut self) -> Vec<ExPolygon> {
        (0..self.next()).map(|_| {
            let mut rings=Vec::new();
            for _ in 0..self.next() {
                let points=(0..self.next()).map(|_| Point2{x:self.next(),y:self.next()}).collect();
                rings.push(Polygon{points});
            }
            let mut rings=rings.into_iter();
            ExPolygon{contour:rings.next().expect("contour"),holes:rings.collect()}
        }).collect()
    }
    fn segments(&mut self) -> Vec<Vec<Point3WithWidth>> {
        (0..self.next()).map(|_| (0..self.next()).map(|_| {
            let x=f32::from_bits(self.next() as u32);let y=f32::from_bits(self.next() as u32);
            let z=f32::from_bits(self.next() as u32);let width=f32::from_bits(self.next() as u32);
            let flow_factor=f32::from_bits(self.next() as u32);let quartile=self.next();
            let dist_to_top_mm=f32::from_bits(self.next() as u32);let distance=self.next();
            Point3WithWidth{x,y,z,width,flow_factor,overhang_quartile:(quartile>=0).then_some(quartile as u8),
                dist_to_top_mm,overhang_distance_mm:(distance>=0).then(||f32::from_bits(distance as u32)),
                ..Point3WithWidth::default()}
        }).collect()).collect()
    }
}

fn role(index: i64) -> SupportPlanRole {
    match index {0=>SupportPlanRole::SupportBody,1=>SupportPlanRole::TopInterface,
        2=>SupportPlanRole::BaseInterface,3=>SupportPlanRole::BottomInterface,_=>panic!("unexpected role")}
}
fn shape(polys:&[ExPolygon]) -> [usize;3] {
    [polys.len(),polys.iter().map(|p|p.holes.len()).sum(),polys.iter().map(|p|
        p.contour.points.len()+p.holes.iter().map(|h|h.points.len()).sum::<usize>()).sum()]
}

fn run() {
    let here=PathBuf::from(std::env::var("T55_EVIDENCE").expect("explicit evidence directory"));
    let mut result=Vec::new();
    let mut hole_cases=0;let mut positive_cuts=0;let mut negative=[0;3];
    for job in ["benchy","base"] {
        let mut reader=Numbers(BufReader::new(File::open(here.join(format!("{}-replay.txt",job))).expect("required real specimen")));
        let count=reader.next();assert!(count>0,"nonempty captured population");
        for _ in 0..count {
            let layer=reader.next();let radius=f32::from_bits(reader.next() as u32);
            let width=f32::from_bits(reader.next() as u32);let avg=reader.next() as usize;
            let segments:Vec<_>=(0..4).map(|_|reader.segments()).collect();
            let areas:Vec<_>=(0..4).map(|_|reader.polys()).collect();let collision=reader.polys();
            let carved:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let unions:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let simplified:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let differences:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let body_after:Vec<_>=(0..3).map(|_|reader.polys()).collect();
            let expected:Vec<_>=(0..reader.next()).map(|_|SupportPlanRoleRegion {
                role:role(reader.next()),regions:reader.polys(),
            }).collect();
            // Expected values are independently recorded WASM output, never derived from native production.
            let actual=tree_support_planner::build_roles(&segments[0],&segments[1],&segments[2],&segments[3],
                &areas[0],&areas[1],&areas[2],&areas[3],radius,&collision,avg,width);
            assert_eq!(actual,expected,"full original-argument role chain {job} layer {layer}");
            assert!(!actual.is_empty(),"real returned role population");
            let mut cut_body=differences[0].clone();
            for (i,cut) in differences.iter().skip(1).enumerate() {
                if !cut_body.is_empty() && !cut.is_empty() {
                    let before=cut_body.clone();
                    cut_body=slicer_core::polygon_ops::clip_polygons(&cut_body,cut,slicer_core::polygon_ops::ClipOperation::Difference);
                    positive_cuts+=usize::from(cut_body!=before);
                }
                assert_eq!(cut_body,body_after[i],"recorded body cut {i}, {job}/{layer}");
            }
            // Measure only this selected body's exact real union, not the carve/difference stage.
            assert!(carved[0].len()>1,"real multi-polygon request");
            clipper2_rust::t55_probe::reset(true);
            let start=Instant::now();
            let measured=measured_polygon_ops::clip_polygons(&carved[0][..1],&carved[0][1..],measured_polygon_ops::ClipOperation::Union);
            let total=start.elapsed().as_nanos() as u64;
            let clipper=clipper2_rust::t55_probe::take();
            let counts=clipper2_rust::t55_probe::counts();
            assert_eq!(counts[3],0,"unchanged-edge TopX reuse mismatch {job}/{layer}");
            assert_eq!(measured,unions[0],"instrumented exact ordered union {job}/{layer}");
            let phases=measured_polygon_ops::t55_take();
            assert_eq!(unions[0],simplified[0],"these matched jobs have no active simplify");
            hole_cases+=usize::from(shape(&unions[0])[1]>0);
            let mut wrong=actual.clone();wrong[0].regions[0].contour.points[0].x+=1;
            assert_ne!(wrong,expected,"vertex negative control");negative[0]+=1;
            let mut wrong=actual.clone();wrong[0].role=SupportPlanRole::RaftRelated;
            assert_ne!(wrong,expected,"role negative control");negative[2]+=1;
            if let Some(poly)=unions[0].iter().position(|p|!p.holes.is_empty()) {
                let mut wrong=measured.clone();wrong[poly].holes.remove(0);
                assert_ne!(wrong,unions[0],"hole negative control");negative[1]+=1;
            }
            result.push(serde_json::json!({"job":job,"layer":layer,"input":shape(&carved[0]),"union":shape(&unions[0]),
                "full_roles":expected.iter().map(|r|format!("{:?}",r.role)).collect::<Vec<_>>(),
                "total_ns":total,"flatten_ns":phases[0],"boolean_wrapper_ns":phases[1],"reconstruct_ns":phases[2],
                "execute_internal_ns":clipper[0],"build_tree_ns":clipper[1],"cleanup_ns":clipper[2],
                "add_paths_ns":clipper[3],"sweep_reset_ns":clipper[4],"sweep_insert_ns":clipper[5],
                "sweep_horizontal_ns":clipper[6],"sweep_intersections_ns":clipper[7],"sweep_top_ns":clipper[8],
                "sweep_final_horz_joins_ns":clipper[9],"intersection_build_ns":clipper[10],"intersection_process_ns":clipper[11],
                "adjust_copy_ns":clipper[12],"adjust_top_x_calls":counts[0],"top_nonterminal_top_x_calls":counts[1],
                "unchanged_edge_repeated_top_x_calls":counts[2],"repeated_top_x_mismatches":counts[3],"merge_comparisons":counts[4]}));
        }
        reader.end();
    }
    assert!(hole_cases>0 && positive_cuts>0 && negative.iter().all(|n|*n>0),"nonvacuous coverage");
    let report=serde_json::json!({"cases":result,"hole_cases":hole_cases,"positive_body_cuts":positive_cuts,
        "negative_controls":{"vertex":negative[0],"hole":negative[1],"role":negative[2]}});
    std::fs::write(here.join("native-internals.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("PERF-T55-REPLAY {}",report);
}

fn main() {run();}

#[test]
fn t55_full_original_role_chain_and_union_internals() {run();}
