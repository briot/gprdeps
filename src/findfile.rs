use crate::settings::Settings;
use crate::directory::{Directory, File};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The entry will always be a directory, and this should return True
/// if we should also traverse children.
fn should_traverse_dir(path: &Path) -> bool {
    path
        .to_str()
        .map(|n| {
            !n.ends_with("External/Ada_Web_Server/aws-dev")
                && !n.ends_with("External/GNATCOLL/gnatcoll-dev")
                && !n.ends_with("Examples/Elektron/Ema/Training")
                && !n.ends_with("Packaging")
                && !n.ends_with("Compiler")
                && !n.ends_with(".dbc")
                && !n.ends_with(".git")
                && !n.ends_with("__pycache__")
                && !n.ends_with("objects")
        })
        .unwrap_or(false)
}

#[derive(Default)]
pub struct FileFind {
    pub gprfiles: Vec<PathBuf>,
    // The list of project files we found on the disk
    pub directories: HashMap<PathBuf, Directory>,
    // The list of files in each directory (potential source files).
    // This doesn't include GPR files themselves, since we know they are not
    // source files.
}

pub fn find_files(root: &Path, settings: &Settings) -> FileFind {
    type Current = Option<(PathBuf, Directory, std::fs::ReadDir)>;

    let mut stack: Vec<PathBuf> = vec![root.to_owned()];
    let mut result = FileFind::default();
    let mut current: Current = None;

    loop {
        match &mut current {
            None => match stack.pop() {
                None => {
                    break;
                }
                Some(path) => match std::fs::read_dir(&path) {
                    Err(err) => {
                        println!(
                            "Error reading directory {}: {}",
                            path.display(),
                            err
                        );
                    }
                    Ok(readdir) => {
                        current = Some((path, Directory::default(), readdir));
                    }
                },
            },
            Some((_, _, readdir)) => {
                match readdir.next() {
                    None => {
                        // Nothing else to read in the current directory
                        let mut d: Current = None;
                        std::mem::swap(&mut d, &mut current);
                        if let Some((p, d, _)) = d {
                            result.directories.insert(p, d);
                        }
                    }
                    Some(Ok(entry)) => {
                        let path = entry.path();
                        match entry.file_type() {
                            Err(err) => {
                                println!("Could not read {}: {}",
                                    path.display(), err);
                            }
                            Ok(ft) => {
                                if !settings.resolve_symbolic_links
                                    && ft.is_symlink()
                                {
                                } else if ft.is_dir() {
                                    if should_traverse_dir(&path) {
                                        stack.push(path);
                                    }
                                } else if ft.is_file() {
                                    if let Some("gpr") = path
                                        .extension()
                                        .and_then(OsStr::to_str)
                                    {
                                        result.gprfiles.push(path);
                                    } else if let Some((_, d, _)) = &mut current
                                    {
                                        d.files.insert(path, File::default());
                                    }
                                }
                            }
                        };
                    }
                    Some(Err(err)) => {
                        // Could not read current entry, just skip it
                        println!("Error {}", err);
                    }
                }
            }
        }
    }

    result
}
