use std::path::PathBuf;

pub struct Directory {
    path: PathBuf,
    pub files: Vec<PathBuf>,
}

impl Directory {
    pub fn new(path: std::path::PathBuf) -> Self {
        let mut files = Vec::new();
        if let Ok(iter) = std::fs::read_dir(&path) {
            for entry in iter.flatten() {
                if let Ok(t) = entry.file_type() {
                    if t.is_file() {
                        files.push(entry.path());
                    }
                }
            }
        }

        Self { path, files }
    }

    /// The number of potential source files in the directory
    pub fn files_count(&self) -> usize {
        self.files.len()
    }
}

// So that a HashSet can be checked by passing a &PathBuf
impl std::borrow::Borrow<std::path::PathBuf> for Directory {
    fn borrow(&self) -> &std::path::PathBuf {
        &self.path
    }
}

impl std::cmp::PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl std::cmp::Eq for Directory {}

impl std::hash::Hash for Directory {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.path.hash(state)
    }
}
