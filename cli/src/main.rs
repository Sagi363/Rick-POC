mod cli;
mod core;
mod error;
mod parsers;

use std::env;
use std::process;

use cli::commands;
use cli::help;
use error::RickError;

fn run() -> error::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        help::print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => {
            help::print_help();
        }
        "--version" | "-v" => {
            help::print_version();
        }
        "list" => {
            if args.len() < 2 {
                eprintln!("\x1b[31mRick: Missing argument. Use 'list agents', 'list workflows', or 'list universes'.\x1b[0m");
                return Err(RickError::InvalidState(
                    "Missing list subcommand".to_string(),
                ));
            }
            match args[1].as_str() {
                "agents" => commands::list_agents()?,
                "workflows" => commands::list_workflows()?,
                "universes" => commands::list_universes()?,
                other => {
                    eprintln!(
                        "\x1b[31mRick: Unknown list target '{}'. Use 'agents', 'workflows', or 'universes'.\x1b[0m",
                        other
                    );
                    return Err(RickError::InvalidState(format!(
                        "Unknown list target: {}",
                        other
                    )));
                }
            }
        }
        "compile" => {
            let name = args.get(1).map(|s| s.as_str());
            commands::compile(name)?;
        }
        "run" => {
            let force = args.iter().any(|a| a == "--force" || a == "-f");
            let wf_name = args[1..].iter()
                .find(|a| !a.starts_with('-'))
                .map(|s| s.as_str());
            match wf_name {
                Some(name) => commands::run(name, force)?,
                None => {
                    eprintln!("\x1b[31mRick: Missing workflow name. Use 'run <workflow>'.\x1b[0m");
                    return Err(RickError::InvalidState(
                        "Missing workflow name".to_string(),
                    ));
                }
            }
        }
        "check" => commands::check()?,
        "invite" => {
            let emails: Vec<&str> = args[1..].iter()
                .filter(|a| !a.starts_with('-'))
                .map(|s| s.as_str())
                .collect();
            commands::invite(&emails)?;
        }
        "status" => commands::status()?,
        "init" => commands::init()?,
        "add" | "install" => {
            if args.len() < 2 {
                eprintln!("\x1b[31mRick: Missing URL. Use 'rick add <universe-repo-url>'.\x1b[0m");
                return Err(RickError::InvalidState("Missing URL".to_string()));
            }
            // Optional: --name / -n flag for custom directory name
            let custom_name = if args.len() >= 4 && (args[2] == "-n" || args[2] == "--name") {
                Some(args[3].as_str())
            } else {
                None
            };
            commands::add(&args[1], custom_name)?;
        }
        "next" => commands::next()?,
        "push" => commands::push()?,
        "setup" => {
            // Parse flags: --universe <url>, --install-deps, --non-interactive
            let universe_url = args.windows(2)
                .find(|w| w[0] == "--universe" || w[0] == "-u")
                .map(|w| w[1].as_str());

            let install_deps = args.iter().any(|a| a == "--install-deps");
            let non_interactive = args.iter().any(|a| a == "--non-interactive");

            commands::setup(universe_url, install_deps, non_interactive)?;
        }
        other => {
            eprintln!(
                "\x1b[31mRick: Unknown command '{}'. Use 'rick help' for usage.\x1b[0m",
                other
            );
            return Err(RickError::InvalidState(format!(
                "Unknown command: {}",
                other
            )));
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\x1b[31mError: {}\x1b[0m", e);
        process::exit(1);
    }
}
