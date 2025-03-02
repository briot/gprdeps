use crate::{
    action_duplicates::ActionDuplicates, action_imported::ActionImported,
    action_path::ActionPath, action_unused::ActionSourceUnused, errors::Error,
    settings::Settings,
};
use clap::{arg, ArgAction, ArgMatches, Command};
use std::path::{Path, PathBuf};

pub enum Action {
    Dependencies(ActionImported),
    DuplicateBase(ActionDuplicates),
    GprShow { gprpath: PathBuf, print_vars: bool },
    ImportPath(ActionPath),
    SourceUnused(ActionSourceUnused),
    Stats,
}

fn to_abs<P>(relpath: P) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    Ok(cwd.join(relpath).canonicalize()?)
}

fn get_path(matches: &ArgMatches, id: &str) -> Result<PathBuf, Error> {
    to_abs(matches.get_one::<PathBuf>(id).unwrap())
}

fn get_path_list(matches: &ArgMatches, id: &str) -> Vec<PathBuf> {
    matches
        .get_many::<PathBuf>(id) // Option<ValuesRef<PathBuf>>
        .into_iter() // Item=ValuesRef<PathBuf>
        .flatten() // Item is &PathBuf
        .filter_map(|p| to_abs(p).ok()) // Item is PathBuf
        .collect()
}

fn get_path_and_root(
    matches: &ArgMatches,
    id: &str,
) -> Vec<(PathBuf, PathBuf)> {
    matches
        .get_many::<String>(id)
        .into_iter()
        .flatten()
        .filter_map(|p| match p.split_once(":") {
            None => Some((to_abs(p).ok()?, to_abs(".").ok()?)),
            Some((p, root)) => Some((to_abs(p).ok()?, to_abs(root).ok()?)),
        })
        .collect()
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
            arg!(--runtime [RUNTIME]... "Projects implicitly imported by all")
                .global(true)
                .value_parser(clap::value_parser!(PathBuf)),
            arg!(--root <DIR_OR_GPR>... "Root directory or project")
                .global(true)
                .default_value(".")
                .value_parser(clap::value_parser!(PathBuf)),
            arg!(--trim  "Only show subset of attributes")
                .global(true)
                .action(ArgAction::SetTrue),
            arg!(--relto [DIR] "Output paths relative to this directory")
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
                    Command::new("imported_by")
                        .about("Show dependencies for a source file")
                        .args([
                            arg!(-d --direct "Show direct dependencies only")
                                .action(ArgAction::SetTrue),
                            arg!(<PATH> "Path to the source file")
                                .value_parser(clap::value_parser!(PathBuf)),
                        ]),
                )
                .subcommand(
                    Command::new("import")
                        .about("Show all files importedby PATH")
                        .args([
                            arg!(-d --direct "Show direct dependencies only")
                                .action(ArgAction::SetTrue),
                            arg!(<PATH> "Path to the source file")
                                .value_parser(clap::value_parser!(PathBuf)),
                        ]),
                )
                .subcommand(
                    Command::new("duplicates")
                        .about("Report duplicate basenames for files"),
                ),
        )
        .subcommand(
            Command::new("path")
                .about("Show how FILE1 imports FILE2, source or gpr")
                .args([
                    arg!(file1: "Importing file")
                        .required(true)
                        .value_parser(clap::value_parser!(PathBuf)),
                    arg!(file2: "Imported file")
                        .required(true)
                        .value_parser(clap::value_parser!(PathBuf)),
                ]),
        )
        .subcommand(
            Command::new("unused")
                .about("Show unused source files")
                .args([
                    arg!(--unused [FILE_ROOT]...
                        "A filename:root that contains a list of \
                         known unused files, relative to ROOT \
                        (defaults to .)"),
                    arg!(--ignore [DIR] ...
                        "Ignore files in those directories")
                    .value_parser(clap::value_parser!(PathBuf)),
                    arg!(--no_recurse
                        "Do not show files only used by unused files")
                    .action(ArgAction::SetTrue),
                ]),
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
        runtime_gpr: get_path_list(&matches, "runtime"),
        root: get_path_list(&matches, "root"),
        trim: matches.get_flag("trim"),
        relto: get_path(&matches, "relto")?,
    };
    let act = match matches.subcommand() {
        Some(("stats", _)) => Action::Stats,
        Some(("source", sub)) => match sub.subcommand() {
            Some(("imported_by", importsub)) => {
                Action::Dependencies(ActionImported {
                    path: get_path(importsub, "PATH")?,
                    recurse: !importsub.get_flag("direct"),
                    kind: crate::action_imported::Kind::ImportedBy,
                })
            }
            Some(("import", importsub)) => {
                Action::Dependencies(ActionImported {
                    path: get_path(importsub, "PATH")?,
                    recurse: !importsub.get_flag("direct"),
                    kind: crate::action_imported::Kind::Import,
                })
            }
            Some(("duplicates", _)) => {
                Action::DuplicateBase(ActionDuplicates {})
            }
            _ => unreachable!(),
        },
        Some(("path", importsub)) => Action::ImportPath(ActionPath {
            source: get_path(importsub, "file1")?,
            target: get_path(importsub, "file2")?,
            show_units: false,
        }),
        Some(("unused", importsub)) => {
            Action::SourceUnused(ActionSourceUnused {
                unused: get_path_and_root(importsub, "unused"),
                ignore: get_path_list(importsub, "ignore"),
                recurse: !importsub.get_flag("no_recurse"),
            })
        }
        Some(("gpr", sub)) => match sub.subcommand() {
            Some(("show", showsub)) => Action::GprShow {
                gprpath: get_path(showsub, "PROJECT")?,
                print_vars: showsub.get_flag("print_vars"),
            },
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    Ok((settings, act))
}
