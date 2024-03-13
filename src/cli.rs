use crate::errors::Error;
use crate::settings::Settings;
use clap::{arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

pub enum Action {
    Stats,
    Dependencies { direct_only: bool, path: PathBuf },
    GprShow { gprpath: PathBuf, print_vars: bool },
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
        .subcommand_precedence_over_arg(true) //  --x val1 val2 subcommand
        .flatten_help(true) // Show help for all subcommands as well
        .arg_required_else_help(true) // show full help if nothing given
        .args([
            arg!(--missing_sources "Report missing sources")
                .global(true)
                .action(ArgAction::SetTrue),
            arg!(-l --symlinks "Resolve symbolic links")
                .global(true)
                .action(ArgAction::SetFalse),
            arg!(--runtime <RUNTIME> ... "Projects implicitly imported by all")
                .global(true)
                .value_parser(clap::value_parser!(PathBuf)),
            arg!(--root <DIR_OR_GPR> ... "Root directory or project")
                .global(true)
                .default_value(".")
                .value_parser(clap::value_parser!(PathBuf)),
        ])
        .subcommand(
            Command::new("stats")
                .about("Show statistics about the project graph"),
        )
        .subcommand(
            Command::new("source")
                .about("Subcommands at the source file level")
                .flatten_help(true)
                .disable_help_subcommand(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("imports")
                        .about("Show dependencies for a source file")
                        .args([
                            arg!(-d --direct "Show direct dependencies only")
                                .action(ArgAction::SetTrue),
                            arg!(<PATH> "Path to the source file")
                                .value_parser(clap::value_parser!(PathBuf)),
                        ]),
                ),
        )
        .subcommand(
            Command::new("gpr")
                .about("Subcommands at the project level")
                .flatten_help(true)
                .disable_help_subcommand(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("show")
                        .about("Expand project attributes for all scenarios")
                        .args([
                            arg!(<PROJECT>  "Project to analyze")
                                .value_parser(clap::value_parser!(PathBuf)),
                            arg!(--print_vars  "Display values of variables")
                                .action(ArgAction::SetTrue),
                        ]),
                ),
        )
        .get_matches();

    let settings = Settings {
        report_missing_source_dirs: matches.get_flag("missing_sources"),
        resolve_symbolic_links: matches.get_flag("symlinks"),
        runtime_gpr: matches
            .get_many::<PathBuf>("runtime")
            .into_iter() // Item is ValuesRef<PathBuf>
            .flatten() // Item is &PathBuf
            .filter_map(|p| to_abs(p).ok()) // Item is PathBuf
            .collect::<Vec<PathBuf>>(),
        root: matches
            .get_one::<PathBuf>("root")
            .map(to_abs) // Option<Result<PathBuf>>
            .unwrap_or_else(|| {
                std::env::current_dir().or_else(|_| Ok(PathBuf::from("/")))
            })?,
    };

    match matches.subcommand() {
        Some(("stats", _)) => Ok((settings, Action::Stats)),
        Some(("source", sub)) => match sub.subcommand() {
            Some(("imports", importsub)) => Ok((
                settings,
                Action::Dependencies {
                    direct_only: importsub.get_flag("direct"),
                    path: get_path(importsub, "PATH")?,
                },
            )),
            _ => unreachable!(),
        },
        Some(("gpr", sub)) => match sub.subcommand() {
            Some(("show", showsub)) => Ok((
                settings,
                Action::GprShow {
                    gprpath: get_path(showsub, "PROJECT")?,
                    print_vars: showsub.get_flag("print_vars"),
                },
            )),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
