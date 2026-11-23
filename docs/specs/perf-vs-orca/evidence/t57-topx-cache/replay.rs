//! Exact recorded full-role replay and isolated native-union internals diagnosis.
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon, SupportPlanRole, SupportPlanRoleRegion};

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


#[test]
fn candidate_full_original_role_chain_matches_recorded_ordered_results() {
    let here=PathBuf::from(std::env::var("T57_REPLAY_CORPUS").expect("explicit recorded corpus"));
    let mut cases=0; let mut holes=0; let mut changed_cuts=0;
    for job in ["benchy","base"] {
        let mut reader=Numbers(BufReader::new(File::open(here.join(format!("{job}-replay.txt"))).expect("required real specimen")));
        let count=reader.next(); assert!(count>0);
        for _ in 0..count {
            let layer=reader.next(); let radius=f32::from_bits(reader.next() as u32);
            let width=f32::from_bits(reader.next() as u32); let avg=reader.next() as usize;
            let segments:Vec<_>=(0..4).map(|_|reader.segments()).collect();
            let areas:Vec<_>=(0..4).map(|_|reader.polys()).collect(); let collision=reader.polys();
            let carved:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let unions:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let _simplified:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let differences:Vec<_>=(0..4).map(|_|reader.polys()).collect();
            let body_after:Vec<_>=(0..3).map(|_|reader.polys()).collect();
            let expected:Vec<_>=(0..reader.next()).map(|_|SupportPlanRoleRegion { role:role(reader.next()),regions:reader.polys() }).collect();
            let actual=tree_support_planner::build_roles(&segments[0],&segments[1],&segments[2],&segments[3],
                &areas[0],&areas[1],&areas[2],&areas[3],radius,&collision,avg,width);
            assert!(!actual.is_empty()); assert_eq!(actual,expected,"full role chain {job}/{layer}");
            let union=slicer_core::polygon_ops::clip_polygons(&carved[0][..1],&carved[0][1..],slicer_core::polygon_ops::ClipOperation::Union);
            assert_eq!(union,unions[0],"ordered body union {job}/{layer}");
            holes+=usize::from(union.iter().any(|p|!p.holes.is_empty()));
            let mut body=differences[0].clone();
            for (i,cut) in differences.iter().skip(1).enumerate() {
                if !body.is_empty() && !cut.is_empty() {
                    let before=body.clone();
                    body=slicer_core::polygon_ops::clip_polygons(&body,cut,slicer_core::polygon_ops::ClipOperation::Difference);
                    changed_cuts+=usize::from(body!=before);
                }
                assert_eq!(body,body_after[i],"ordered body subtraction {job}/{layer}/{i}");
            }
            cases+=1;
            println!("T57_EXACT_REPLAY {job}/{layer} PASS");
        }
        reader.end();
    }
    assert_eq!(cases,8,"recorded selected-case contract");
    assert_eq!(holes,5,"recorded hole-bearing selected unions");
    assert_eq!(changed_cuts,4,"recorded output-changing body cuts");
}
fn main() {}
