"""Reuse the audited streaming parser, never read the large specimens into memory."""

import shutil
import manage

harness = manage.RAW / "replay"
assert not harness.exists(), "one-shot replay setup"
(harness / "src").mkdir(parents=True)
template = (manage.HERE.parent / "t55-body-union/harness.rs").read_text()
prefix = template.split("fn run() {", 1)[0]
prefix = (
    prefix.replace("use std::time::Instant;\n", "")
    .replace("pub use slicer_core::profile;\n", "")
    .replace("mod measured_polygon_ops;\n", "")
)
body = r"""
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
"""
(harness / "src/main.rs").write_text(prefix + body)
shutil.copy2(harness / "src/main.rs", manage.HERE / "replay.rs")
(harness / "Cargo.toml").write_text(f'''[package]
name = "t57-role-replay"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
tree-support-planner = {{ path = "{(manage.SOURCE / "modules/core-modules/tree-support-planner").as_posix()}" }}
slicer-core = {{ path = "{(manage.SOURCE / "crates/slicer-core").as_posix()}", features = ["host-algos"] }}
slicer-ir = {{ path = "{(manage.SOURCE / "crates/slicer-ir").as_posix()}" }}
''')
shutil.copy2(manage.REPO / "Cargo.lock", harness / "Cargo.lock")
print("PASS: candidate full-role replay harness ready")
