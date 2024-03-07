use crate::errors::Error;
use crate::settings::Settings;
use clap::{arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

pub enum Action {
    Stats,
    Dependencies { direct_only: bool, path: PathBuf },
    GprShow { gprpath: PathBuf },
}

fn to_abs(relpath: &PathBuf) -> Result<PathBuf, Error> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    Ok(cwd.join(relpath).canonicalize()?)
}

fn get_path(matches: &ArgMatches, id: &str) -> Result<PathBuf, Error> {
    to_abs(matches.get_one::<PathBuf>(id).unwrap())
}

pub fn parse_cli() -> Result<(Settings, Action), Error> {
    let matches = Command::new("gprdeps")
        .version("1.0")
        .about("Querying GPR projects")
        .subcommand_required(true)
        .flatten_help(true) // Show help for all subcommands as well
        .arg_required_else_help(true) // show full help if nothing given
        .args([
            arg!(--missing_sources "Report missing sources")
                .action(ArgAction::SetTrue),
            arg!(-l --symlinks "Resolve symbolic links")
                .action(ArgAction::SetFalse),
            arg!(--runtime <RUNTIME> ... "Projects implicitly imported by all")
                .value_parser(clap::value_parser!(PathBuf))
        ])
        .subcommand(
            Command::new("stats")
                .about("Show statistics about the project graph"),
        )
        .subcommand(
            Command::new("deps")
                .about("Show dependencies for a source file")
                .args([
                    arg!(-d --direct "Show direct dependencies only")
                        .action(ArgAction::SetTrue),
                    arg!(<PATH> "Path to the source file")
                        .value_parser(clap::value_parser!(PathBuf)),
                ])
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
                        .arg(arg!(<PROJECT>  "Project to analyze")
                            .value_parser(clap::value_parser!(PathBuf))
                        ),
                ),
        )
        .get_matches();

    let settings = Settings {
        report_missing_source_dirs: matches.get_flag("missing_sources"),
        resolve_symbolic_links: matches.get_flag("symlinks"),
        runtime_gpr: matches.get_many::<PathBuf>("runtime")
            .into_iter()                     // Item is ValuesRef<PathBuf>
            .flatten()                       // Item is &PathBuf
            .filter_map(|p| to_abs(p).ok())  // Item is PathBuf
            .collect::<Vec<PathBuf>>()
    };

    match matches.subcommand() {
        Some(("stats", _)) => Ok((settings, Action::Stats)),
        Some(("deps", sub)) => Ok((
            settings,
            Action::Dependencies {
                direct_only: sub.get_flag("direct"),
                path: get_path(sub, "PATH")?,
            },
        )),
        Some(("gpr", sub)) => match sub.subcommand() {
            Some(("show", showsub)) => Ok((
                settings,
                Action::GprShow {
                    gprpath: get_path(showsub, "PROJECT")?,
                },
            )),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
