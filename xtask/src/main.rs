mod build_guests;
mod check_deviations;
mod dist;
mod gen_config_docs;

use std::env;
use std::process::ExitCode;

const USAGE: &str = "\
xtask — workspace build helpers for Pinch 'n Print

USAGE:
    cargo xtask <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    build-guests          Build every core-module and test-guest WASM component.
    build-guests --check  Exit 1 if any guest artifact is stale.
    build-guests --list   Print every discovered guest (crate name, manifest, expected artifact path).
    check-deviations          Regenerate doc 07 Open Deviation Map + doc 15 config tables.
    check-deviations --check  Exit 1 if doc 07 or doc 15 generated sections are stale.
    gen-config-docs           Regenerate the generated tables in docs/15 from manifests + host-keys.toml.
    gen-config-docs --check   Exit 1 if doc 15's generated tables are stale.
    dist                  Build pnp_cli + all core-module WASMs and stage them under target/dist/.
    dist --debug          Same as `dist`, but stages the debug-profile binary.

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
        Some("check-deviations") => {
            let flag = args.get(1).map(String::as_str);
            let ws = build_guests::workspace_root();
            let check_only = matches!(flag, Some("--check"));
            if let Some(f) = flag {
                if f != "--check" {
                    eprintln!("xtask: unknown flag '{f}' for check-deviations\n");
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
            let mut code = check_deviations::run(&ws, check_only);
            if code == 0 {
                code = gen_config_docs::run(&ws, check_only);
            }
            ExitCode::from(code as u8)
        }
        Some("dist") => {
            let flag = args.get(1).map(String::as_str);
            match flag {
                None => {
                    let ws = build_guests::workspace_root();
                    ExitCode::from(dist::dist_command(&ws, false) as u8)
                }
                Some("--debug") => {
                    let ws = build_guests::workspace_root();
                    ExitCode::from(dist::dist_command(&ws, true) as u8)
                }
                Some(other) => {
                    eprintln!("xtask: unknown flag '{other}' for dist\n");
                    eprintln!("{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        Some("gen-config-docs") => {
            let flag = args.get(1).map(String::as_str);
            match flag {
                None => {
                    let ws = build_guests::workspace_root();
                    ExitCode::from(gen_config_docs::run(&ws, false) as u8)
                }
                Some("--check") => {
                    let ws = build_guests::workspace_root();
                    ExitCode::from(gen_config_docs::run(&ws, true) as u8)
                }
                Some(other) => {
                    eprintln!("xtask: unknown flag '{other}' for gen-config-docs\n");
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
