use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The entry will always be a directory, and this should return True
/// if we should also traverse children.
fn should_traverse_dir(path: &Path) -> bool {
    path.to_str()
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
    stack: Vec<PathBuf>,
    current: Option<std::fs::ReadDir>,
}

impl FileFind {
    /// Start searching for file in path, recursively
    pub fn new(path: &Path) -> FileFind {
        let mut f = FileFind::default();
        f.pushdir(path.to_owned());
        f
    }

    /// Push a new directory to traverse (we will first return the entries from
    /// that directory, then the remaining ones from the parent directory,
    /// and so on).
    fn pushdir(&mut self, path: PathBuf) {
        self.stack.push(path);
    }
}

impl Iterator for FileFind {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current {
                None => match self.stack.pop() {
                    None => {
                        return None;
                    }
                    Some(path) => match std::fs::read_dir(&path) {
                        Err(err) => {
                            eprintln!(
                                "Error reading directory {}: {}",
                                path.display(),
                                err
                            );
                        }
                        Ok(readdir) => {
                            self.current = Some(readdir);
                        }
                    },
                },
                Some(readdir) => {
                    match readdir.next() {
                        None => {
                            // Nothing else to read in the current directory
                            self.current = None;
                        }
                        Some(Ok(entry)) => {
                            let path = &entry.path();
                            match entry.file_type() {
                                Err(e) => eprintln!(
                                    "Could not read {}: {}",
                                    entry.path().display(),
                                    e
                                ),
                                Ok(ft) => {
                                    if ft.is_symlink() {
                                    } else if ft.is_dir() {
                                        if should_traverse_dir(path) {
                                            self.pushdir(path.to_owned());
                                        }
                                    } else if ft.is_file() {
                                        if let Some("gpr") = path
                                            .extension()
                                            .and_then(OsStr::to_str)
                                        {
                                            return Some(path.to_owned());
                                        }
                                    }
                                }
                            };
                        }
                        Some(Err(err)) => {
                            // Could not read current entry, just skip it
                            eprintln!("Error {}", err);
                        }
                    }
                }
            }
        }
    }
}
