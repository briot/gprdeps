use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub struct FileFind {
    iter: Vec<std::fs::ReadDir>,
}

impl FileFind {
    /// Start searching for file in path, recursively
    pub fn new(path: &Path) -> FileFind {
        let mut f = FileFind { iter: vec![] };
        f.pushdir(path);
        f
    }

    /// Push a new directory to traverse (we will first return the entries from
    /// that directory, then the remaining ones from the parent directory,
    /// and so on).
    fn pushdir(&mut self, path: &Path) {
        match std::fs::read_dir(path) {
            Ok(iter) => self.iter.push(iter),
            Err(err) => {
                println!("Error reading directory {}: {}", path.display(), err);
            }
        }
    }

    /// Whether the directory should be traversed
    fn traverse_dir(&mut self, path: &Path) -> bool {
        let meta = std::fs::symlink_metadata(path);
        let n = path.as_os_str().to_str();
        match (meta, n) {
            (Ok(meta), Some(n)) => {
                meta.is_dir()
                    && !meta.is_symlink()
                    && !n.ends_with("External/Ada_Web_Server/aws-dev")
                    && !n.ends_with("External/GNATCOLL/gnatcoll-dev")
                    && !n.ends_with("Packaging")
                    && !n.ends_with("Compiler")
                    && !n.ends_with(".dbc")
            }
            _ => false,
        }
    }
}

impl Iterator for FileFind {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.last_mut()?.next() {
                None => {
                    self.iter.pop(); //  Nothing else to read in child directory
                }
                Some(Ok(e)) => {
                    let path = e.path();
                    match path.extension().and_then(OsStr::to_str) {
                        Some("gpr") => {
                            return Some(std::fs::canonicalize(path).unwrap())
                        }
                        _ => {
                            if self.traverse_dir(&path) {
                                self.pushdir(&path);
                            }
                        }
                    }
                }
                Some(Err(err)) => {
                    println!("Error {}", err);
                }
            }
        }
    }
}
