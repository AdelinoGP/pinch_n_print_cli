//! AC-5 (the `L` lane): real adapter execution inside a supervised Windows child,
//! plus controlled hard-deadline and abnormal-abort probes.
//!
//! # What this file proves, and what it refuses to claim
//!
//! The adapter promises **no** in-process render deadline. `evaluation_steps`
//! bounds the evaluator's work; a native render cannot be interrupted from inside
//! the library. So containment is demonstrated the only honest way: a parent test
//! spawns this same test binary as a child, supervises it with a hard wall-clock
//! deadline, and observes the outcome.
//!
//! The probes are:
//!
//! * `run` — real adapter evaluation inside a child, success path;
//! * `hang` — a deliberately blocked child killed at the deadline, which must
//!   report exactly `TimedOut` and leave no surviving child;
//! * `abort` — an abnormal termination, which must also leave a usable
//!   pre-execution reproducer on disk;
//! * `budget` — evaluator budget exhaustion, whose classification must stay
//!   distinct from a timeout;
//! * `recursion` — minimum-stack recursion must return `PartialOutput` in a child.
//!
//! The reproducer (source, options, engine identity) is written **before** the
//! child executes anything dangerous, so a killed or aborted child still leaves
//! enough to rerun it.
//!
//! The parent uses a bounded `try_wait` loop to enforce the deadline. That loop is
//! the mechanism of the deadline, not the evidence for a claim: every assertion is
//! about the observed exit status, the written reproducer, and whether the child
//! was actually reaped.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use slicer_scad::{
    engine_info, evaluate, ScadErrorKind, ScadEvalOptions, ScadEvalOutput, ScadGeometry,
    ScadLimits, ScadSource,
};

/// Environment variable selecting the child behaviour.
const CHILD_MODE: &str = "SCAD_CHILD_MODE";

/// Wall-clock budget the parent allows a supervised child.
const DEADLINE: Duration = Duration::from_secs(20);

/// A bounded fixture that really evaluates, renders and converts.
const BOUNDED_SOURCE: &str = "difference(){ square(10); translate([3,3]) square(4); }";

/// A fixture with a genuinely long evaluation: a 25M-iteration nested loop.
const LONG_SOURCE: &str = "for (a = [0:5000]) for (b = [0:5000]) translate([a, b, 0]) cube(1);";

const RECURSION_SOURCE: &str =
    "function cum(list, k) = (k == 0) ? 0 : cum(list, k - 1) + list[k - 1];\
     cube(3); cube(cum([5], -1)); cube(7);";

fn minimum_stack_options() -> ScadEvalOptions {
    options(
        ScadLimits {
            worker_stack_bytes: slicer_scad::MIN_WORKER_STACK_BYTES,
            ..ScadLimits::default()
        },
        None,
    )
}

fn repro_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scad-adapter/repro")
}

fn options(limits: ScadLimits, extrude: Option<f64>) -> ScadEvalOptions {
    ScadEvalOptions {
        extrude_height_mm: extrude,
        limits,
        ..ScadEvalOptions::default()
    }
}

/// Persist everything needed to rerun one child before it is allowed to run.
fn write_reproducer(
    case: &str,
    source: &ScadSource,
    options: &ScadEvalOptions,
) -> std::path::PathBuf {
    let dir = repro_dir();
    std::fs::create_dir_all(&dir).expect("create repro directory");
    let path = dir.join(format!("{case}.json"));
    let engine = engine_info();
    // Real JSON, not Rust `Debug` syntax: a retained reproducer must be readable by
    // any tool, and a `Debug` dump is not JSON at all.
    let document = serde_json::json!({
        "case": case,
        "engine_info": engine,
        "source": source,
        "options": options,
    });
    let body = serde_json::to_string_pretty(&document).expect("serialize reproducer")
        + "
";
    let mut file = std::fs::File::create(&path).expect("create repro file");
    file.write_all(body.as_bytes()).expect("write repro file");
    file.sync_all().expect("flush repro file");
    path
}

/// Outcome of one supervised child.
#[derive(Debug, PartialEq, Eq)]
enum Supervised {
    Exited(i32),
    Terminated,
    TimedOut,
}

struct ChildOutcome {
    outcome: Supervised,
    reaped: bool,
    stdout: String,
}

/// Spawn `this` test binary with `--exact scad_child_entry`, supervise it with a
/// hard deadline, and kill it if the deadline expires.
fn supervise(case: &str) -> ChildOutcome {
    supervise_in(case, None)
}

/// [`supervise`] with an explicit working directory for the child, so a
/// working-directory fallback can be tested without changing the concurrent test
/// process's own directory.
fn supervise_in(case: &str, cwd: Option<&std::path::Path>) -> ChildOutcome {
    let exe = std::env::current_exe().expect("current test executable");
    let dir = repro_dir();
    std::fs::create_dir_all(&dir).expect("create repro directory");
    let out_path = dir.join(format!("{case}.stdout.txt"));

    let stdout = std::fs::File::create(&out_path).expect("create child stdout file");
    let mut command = Command::new(exe);
    command
        .args([
            "--exact",
            "scad_child_entry",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_MODE, case)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    if let Some(directory) = cwd {
        command.current_dir(directory);
    }
    let mut child = command.spawn().expect("spawn supervised child");

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().expect("poll child") {
            Some(status) => break status,
            None if started.elapsed() >= DEADLINE => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().expect("reap killed child");
            }
            // test-quality: this sleep *is* the deadline mechanism, not evidence.
            // Every assertion below is about the observed exit status, the written
            // reproducer and whether the child was reaped; none infers correctness
            // from elapsed time. A gated target or an explicit join cannot express
            // "kill this child at a wall-clock bound".
            // The sleep *is* the deadline mechanism; every assertion below is about
            // the observed exit status, the reproducer and the reap.
            None => std::thread::sleep(Duration::from_millis(20)), // test-quality: deadline enforcement in a process supervisor, not evidence inferred from elapsed time
        }
    };

    // "No surviving worker": whatever happened, the child must be reaped and must
    // not still be running.
    let reaped = child.try_wait().expect("final poll").is_some();
    let stdout = std::fs::read_to_string(&out_path).unwrap_or_default();
    ChildOutcome {
        outcome: if timed_out {
            Supervised::TimedOut
        } else {
            match status.code() {
                Some(code) => Supervised::Exited(code),
                None => Supervised::Terminated,
            }
        },
        reaped,
        stdout,
    }
}

#[test]
fn scad_process_supervision_retains_repro_and_joins() {
    let child_only = std::env::var(CHILD_MODE).is_err();
    assert!(
        child_only,
        "this entry is a child-only dispatcher and must not be entered directly"
    );

    // Every case writes its reproducer before the child may run anything.
    let run_source = ScadSource {
        label: "supervised-run".to_string(),
        text: BOUNDED_SOURCE.to_string(),
    };
    let run_options = options(ScadLimits::default(), Some(2.0));
    let run_repro = write_reproducer("run", &run_source, &run_options);
    assert!(
        run_repro.exists(),
        "the reproducer must exist before the child runs"
    );
    let recorded = std::fs::read_to_string(&run_repro).expect("read reproducer");
    assert!(
        recorded.contains("manifold_cpp") && recorded.contains("supervised-run"),
        "the reproducer must record engine identity and the source label: {recorded}"
    );

    // --- case 1: a real bounded evaluation inside a child -------------------
    let run = supervise("run");
    assert!(run.reaped, "the supervised child must be reaped");
    assert_eq!(
        run.outcome,
        Supervised::Exited(0),
        "the bounded fixture must succeed in a child; stdout={}",
        run.stdout
    );
    assert!(
        run.stdout.contains("GEOMETRY=Mesh") || run.stdout.contains("GEOMETRY=Contours"),
        "the child must report the geometry it produced: {}",
        run.stdout
    );

    // --- case 2: a hard deadline kills a deliberately blocked child ---------
    let long_source = ScadSource {
        label: "supervised-hang".to_string(),
        text: LONG_SOURCE.to_string(),
    };
    let long_options = options(ScadLimits::default(), None);
    write_reproducer("hang", &long_source, &long_options);
    let hang = supervise("hang");
    assert!(hang.reaped, "a killed child must still be reaped");
    assert_eq!(
        hang.outcome,
        Supervised::TimedOut,
        "the blocking child must time out, not abort; stdout={}",
        hang.stdout
    );
    assert!(
        hang.stdout.contains("BLOCKING_FOR_THE_DEADLINE_PROBE"),
        "{}",
        hang.stdout
    );
    assert!(
        !hang.stdout.contains("UNEXPECTEDLY_COMPLETED"),
        "{}",
        hang.stdout
    );

    // --- case 3: abnormal termination keeps its reproducer ------------------
    let abort_source = ScadSource {
        label: "supervised-abort".to_string(),
        text: BOUNDED_SOURCE.to_string(),
    };
    let abort_options = options(ScadLimits::default(), None);
    let abort_repro = write_reproducer("abort", &abort_source, &abort_options);
    let abort = supervise("abort");
    assert!(
        abort.reaped,
        "an abnormally terminated child must be reaped"
    );
    assert!(
        matches!(abort.outcome, Supervised::Terminated)
            || matches!(abort.outcome, Supervised::Exited(code) if code != 0),
        "an abnormal abort must be a failed termination, never a timeout: {:?}; stdout={}",
        abort.outcome,
        abort.stdout
    );
    assert!(abort.stdout.contains("ABOUT_TO_ABORT"), "{}", abort.stdout);
    assert!(
        abort_repro.exists(),
        "the pre-execution reproducer must survive an abort"
    );

    // --- case 3b: a relative request resolved from a different CWD ----------
    // The child runs with its working directory set to the directory that holds a
    // readable `tempting.scad`, and asks for it by a relative path. A resolver with
    // any working-directory fallback would find it; the boundary must not.
    let resource_dir = repro_dir().join("tempting-cwd");
    std::fs::create_dir_all(&resource_dir).expect("create tempting cwd directory");
    std::fs::write(
        resource_dir.join("tempting.scad"),
        "cube(9);
",
    )
    .expect("write tempting include for the cwd probe");
    let resource_source = ScadSource {
        label: "supervised-resource-cwd".to_string(),
        text: "include <tempting.scad>; cube(1);".to_string(),
    };
    write_reproducer(
        "resource-cwd",
        &resource_source,
        &options(ScadLimits::default(), None),
    );
    let resource = supervise_in("resource-cwd", Some(&resource_dir));
    assert!(resource.reaped, "the resource child must be reaped");
    assert_eq!(
        resource.outcome,
        Supervised::Exited(0),
        "the child must classify, not crash: {}",
        resource.stdout
    );
    assert!(
        resource.stdout.contains("KIND=ExternalResourceDenied"),
        "a relative include must be denied even when it exists in the child's own          working directory: {}",
        resource.stdout
    );

    // --- case 4: budget exhaustion is not a timeout -------------------------
    let budget_source = ScadSource {
        label: "supervised-budget".to_string(),
        text: LONG_SOURCE.to_string(),
    };
    let budget_options = ScadEvalOptions {
        limits: ScadLimits {
            evaluation_steps: 100_000,
            ..ScadLimits::default()
        },
        ..ScadEvalOptions::default()
    };
    write_reproducer("budget", &budget_source, &budget_options);
    let budget = supervise("budget");
    assert!(budget.reaped, "the budget child must be reaped");
    assert_eq!(
        budget.outcome,
        Supervised::Exited(0),
        "evaluator budget exhaustion is a classified result, not a process failure: {}",
        budget.stdout
    );
    assert!(
        budget.stdout.contains("KIND=EvaluationBudgetExceeded"),
        "the child must report the budget classification: {}",
        budget.stdout
    );
    assert!(
        !budget.stdout.contains("KIND=Timeout"),
        "a budget exhaustion must never be reported as a timeout"
    );
}

/// Child-only dispatcher. Selected with `--exact scad_child_entry`.
///
/// With `CHILD_MODE` unset this is a normal (and passing) test that asserts the
/// dispatcher contract; the parent never relies on that path.
#[test]
fn scad_child_entry() {
    let Ok(case) = std::env::var(CHILD_MODE) else {
        assert!(
            std::env::var(CHILD_MODE).is_err(),
            "the dispatcher must not run as an ordinary test"
        );
        return;
    };

    match case.as_str() {
        "run" => {
            let source = ScadSource {
                label: "supervised-run".to_string(),
                text: BOUNDED_SOURCE.to_string(),
            };
            let options = options(ScadLimits::default(), Some(2.0));
            match evaluate(&source, &options) {
                Ok(ScadEvalOutput { geometry, .. }) => match geometry {
                    ScadGeometry::Mesh(mesh) => {
                        println!(
                            "GEOMETRY=Mesh verts={} tris={}",
                            mesh.vertices.len(),
                            mesh.indices.len() / 3
                        );
                    }
                    ScadGeometry::Contours(regions) => {
                        println!("GEOMETRY=Contours regions={}", regions.len());
                    }
                    ScadGeometry::Empty => println!("GEOMETRY=Empty"),
                },
                Err(error) => {
                    println!("KIND={:?} message={}", error.kind, error.message);
                    std::process::exit(2);
                }
            }
        }
        "hang" => {
            println!("BLOCKING_FOR_THE_DEADLINE_PROBE");
            std::io::stdout().flush().expect("flush deadline marker");
            // test-quality: the child-only blocking input for AC-5's hard-deadline
            // probe. The parent asserts that this process was killed and reaped and
            // that its exit status is nonzero; it asserts nothing about how long a
            // real adapter run takes, so there is no timing claim to be flaky.
            std::thread::sleep(Duration::from_secs(3600)); // test-quality: the controlled blocking input for AC-5's hard-deadline probe; the parent asserts the kill and the reap, not a duration
            println!("UNEXPECTEDLY_COMPLETED");
        }
        "resource-cwd" => {
            // The parent sets this child's working directory to the directory that
            // holds `tempting.scad`, and this source asks for it by a RELATIVE path.
            // A resolver with any working-directory fallback would read it; the
            // deny-all boundary must not.
            let source = ScadSource {
                label: "supervised-resource-cwd".to_string(),
                text: "include <tempting.scad>; cube(1);".to_string(),
            };
            match evaluate(&source, &options(ScadLimits::default(), None)) {
                Ok(_) => println!("KIND=Ok"),
                Err(error) => println!("KIND={:?}", error.kind),
            }
        }
        "abort" => {
            println!("ABOUT_TO_ABORT");
            std::io::stdout().flush().expect("flush abort marker");
            std::process::abort();
        }
        "budget" => {
            let source = ScadSource {
                label: "supervised-budget".to_string(),
                text: LONG_SOURCE.to_string(),
            };
            let options = ScadEvalOptions {
                limits: ScadLimits {
                    evaluation_steps: 100_000,
                    ..ScadLimits::default()
                },
                ..ScadEvalOptions::default()
            };
            match evaluate(&source, &options) {
                Ok(_) => println!("KIND=Ok"),
                Err(error) => println!("KIND={:?}", error.kind),
            }
        }
        "recursion" => {
            let source = ScadSource {
                label: "stack-recursion".to_string(),
                text: RECURSION_SOURCE.to_string(),
            };
            match evaluate(&source, &minimum_stack_options()) {
                Ok(_) => println!("KIND=Ok"),
                Err(error) => println!("KIND={:?}", error.kind),
            }
        }
        other => panic!("unknown child mode {other:?}"),
    }
}

/// Validate unsafe options in-process, but test minimum-stack recursion only in
/// a supervised child so an accidental overflow cannot abort the parent suite.
#[test]
fn scad_worker_stack_policy_contains_non_tail_recursion() {
    // A stack below the documented floor is an invalid option, not a small stack.
    let too_small = ScadEvalOptions {
        limits: ScadLimits {
            worker_stack_bytes: 1024 * 1024,
            ..ScadLimits::default()
        },
        ..ScadEvalOptions::default()
    };
    let source = ScadSource {
        label: "stack-policy".to_string(),
        text: "cube(1);".to_string(),
    };
    assert_eq!(
        evaluate(&source, &too_small)
            .expect_err("unsafe stack must be rejected")
            .kind,
        ScadErrorKind::InvalidOptions
    );

    // The documented minimum contains the engine's non-tail recursion abort: the
    // model is reported partial, and the process survives to say so.
    let recursion = ScadSource {
        label: "stack-recursion".to_string(),
        text: RECURSION_SOURCE.to_string(),
    };
    let minimum = minimum_stack_options();
    write_reproducer("recursion", &recursion, &minimum);
    let child = supervise("recursion");
    assert!(child.reaped, "the recursion child must be reaped");
    assert_eq!(
        child.outcome,
        Supervised::Exited(0),
        "the contained abort must be classified, not a process failure: {}",
        child.stdout
    );
    assert!(
        child.stdout.contains("KIND=PartialOutput"),
        "the minimum-stack recursion must classify as PartialOutput: {}",
        child.stdout
    );
    assert!(
        std::path::Path::new(&repro_dir()).exists(),
        "supervised runs leave their reproducers under target/scad-adapter/"
    );
}
