mod build_guests;

use std::env;
use std::process::ExitCode;

const USAGE: &str = "\
xtask — workspace build helpers for ModularSlicer

USAGE:
    cargo xtask <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    build-guests          Build every core-module and test-guest WASM component.
    build-guests --check  Exit 1 if any guest artifact is stale.
    build-guests --list   Print every discovered guest (crate name, manifest, expected artifact path).

OPTIONS:
    -h, --help            Print this message.
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("build-guests") => {
            let flag = args.get(1).map(String::as_str);
            match flag {
                Some("--list") => {
                    let ws = build_guests::workspace_root();
                    match build_guests::list_command(&ws) {
                        Ok(code) => ExitCode::from(code as u8),
                        Err(e) => {
                            eprintln!("xtask: list_command error: {e}");
                            ExitCode::from(1)
                        }
                    }
                }
                None => {
                    let ws = build_guests::workspace_root();
                    std::process::exit(build_guests::build_command(&ws));
                }
                Some("--check") => {
                    let ws = build_guests::workspace_root();
                    std::process::exit(build_guests::check_command(&ws));
                }
                Some(other) => {
                    eprintln!("xtask: unknown flag '{other}' for build-guests\n");
                    eprintln!("{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        Some(other) => {
            eprintln!("xtask: unknown subcommand '{other}'\n");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
