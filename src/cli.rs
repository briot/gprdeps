use crate::errors::Error;
use crate::settings::Settings;
use clap::{arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

pub enum Action {
    Stats,
    Dependencies { direct_only: bool, path: PathBuf },
    GprShow { gprpath: PathBuf },
}

fn get_path(matches: &ArgMatches, id: &str) -> Result<PathBuf, Error> {
    let raw = matches.get_one::<String>(id).unwrap();
    let relpath = PathBuf::from(raw);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    Ok(cwd.join(relpath).canonicalize()?)
}

pub fn parse_cli() -> (Settings, Action) {
    let matches = Command::new("gprdeps")
        .version("1.0")
        .about("Querying GPR projects")
        .subcommand_required(true)
        .flatten_help(true) // Show help for all subcommands as well
        .arg_required_else_help(true) // show full help if nothing given
        .arg(
            arg!(--missing_sources "Report missing sources")
                .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(-l --symlinks "Resolve symbolic links")
                .action(ArgAction::SetFalse),
        )
        .subcommand(
            Command::new("stats")
                .about("Show statistics about the project graph"),
        )
        .subcommand(
            Command::new("deps")
                .about("Show dependencies for a source file")
                .arg(
                    arg!(-d --direct "Show direct dependencies only")
                        .action(ArgAction::SetTrue),
                )
                .arg(arg!(<PATH> "Path to the source file")),
        )
        .subcommand(
            Command::new("gpr")
                .about("Subcommands at the project level")
                .flatten_help(true)
                .disable_help_subcommand(true)
                .disable_help_flag(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("show")
                        .about("Expand project attributes for all scenarios")
                        .arg(arg!(<PROJECT>  "Project to analyze")),
                ),
        )
        .get_matches();

    let settings = Settings {
        report_missing_source_dirs: matches.get_flag("missing_sources"),
        resolve_symbolic_links: matches.get_flag("symlinks"),
    };

    match matches.subcommand() {
        Some(("stats", _)) => (settings, Action::Stats),
        Some(("deps", sub)) => (
            settings,
            Action::Dependencies {
                direct_only: sub.get_flag("direct"),
                path: get_path(sub, "PATH").expect("Cannot resolve path"),
            },
        ),
        Some(("gpr", sub)) => match sub.subcommand() {
            Some(("show", showsub)) => (
                settings,
                Action::GprShow {
                    gprpath: get_path(showsub, "PROJECT")
                        .expect("Cannot resolve project path"),
                },
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
