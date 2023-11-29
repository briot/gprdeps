use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::fs::FileType;

struct DirEntry {
    path: PathBuf,

    // Will always be needed, so save it.
    ft: FileType,
}

impl DirEntry {

    #[cfg(unix)]
    pub(crate) fn from_entry(ent: &std::fs::DirEntry) -> Result<Self, String> {
        let ft = ent
            .file_type()
            .map_err(|e| format!(
                "Could not read {}: {}", ent.path().display(), e))?;
        Ok(DirEntry {
            path: ent.path(),
            ft,
        })
    }

    /// Return the file name of this entry.
    ///
    /// If this entry has no file name (e.g., `/`), then the full path is
    /// returned.
    pub fn file_name(&self) -> &OsStr {
        self.path.file_name().unwrap_or_else(|| self.path.as_os_str())
    }

    pub fn is_dir(&self) -> bool {
        self.ft.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.ft.is_file()
    }

    pub fn is_symlink(&self) -> bool {
        self.ft.is_symlink()
    }

}


/// The entry will always be a directory, and this should return True
/// if we should also traverse children.
fn should_traverse_dir(entry: &DirEntry) -> bool {
    entry.path
     .to_str()
     .map(|n|
         !n.ends_with("External/Ada_Web_Server/aws-dev")
         && !n.ends_with("External/GNATCOLL/gnatcoll-dev")
         && !n.ends_with("Examples/Elektron/Ema/Training")
         && !n.ends_with("Packaging")
         && !n.ends_with("Compiler")
         && !n.ends_with(".dbc")
         && !n.ends_with(".git")
         && !n.ends_with("__pycache__")
         && !n.ends_with("objects"))
     .unwrap_or(false)
}

// pub fn findfile(root: &Path) -> dyn Iterator<Item=PathBuf> {
//     WalkDir::new(root)
//         .follow_links(false)
//         .into_iter()
//         .filter_entry(should_traverse)   // Omit some directories from path
//         .filter_map(|e| e.ok())          // Ignore errors
//         .map(|e| e.path())
// }


pub struct FileFind {
    stack: Vec<std::fs::ReadDir>,
}

impl FileFind {
    /// Start searching for file in path, recursively
    pub fn new(path: &Path) -> FileFind {
        let mut f = FileFind { stack: vec![] };
        f.pushdir(path);
        f
    }

    /// Push a new directory to traverse (we will first return the entries from
    /// that directory, then the remaining ones from the parent directory,
    /// and so on).
    fn pushdir(&mut self, path: &Path) {
        match std::fs::read_dir(path) {
            Ok(readdir) => self.stack.push(readdir),
            Err(err) => {
                println!("Error reading directory {}: {}", path.display(), err);
            }
        }
    }

}

impl Iterator for FileFind {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.last_mut()?.next() {
                None => {
                    self.stack.pop(); //  Nothing else to read in directory
                }
                Some(Ok(e)) => {
                    match DirEntry::from_entry(&e) {
                        Err(err) => println!("{}", err),
                        Ok(e) => {
                            if e.is_dir() {
                                if !e.is_symlink() && should_traverse_dir(&e) {
                                    self.pushdir(&e.path);
                                }
                            } else if e.is_file() {
                                if let Some("gpr") =
                                    e.path.extension().and_then(OsStr::to_str)
                                {
                                    return Some(e.path);
                                }
                            }
                        }
                    };
                }
                Some(Err(err)) => {
                    println!("Error {}", err);
                }
            }
        }
    }
}
