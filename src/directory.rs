use std::collections::HashMap;
use std::path::PathBuf;
use ustr::Ustr;

pub struct Directory {
    path: PathBuf,
    files: HashMap<Ustr, PathBuf>, // basename -> full path
}

impl Directory {
    pub fn new(path: std::path::PathBuf) -> Self {
        let mut files = HashMap::new();
        if let Ok(iter) = std::fs::read_dir(&path) {
            for entry in iter.flatten() {
                if let Ok(t) = entry.file_type() {
                    if t.is_file() {
                        match entry.file_name().to_str() {
                            None => {}
                            Some(fname) => {
                                files.insert(Ustr::from(fname), entry.path());
                            }
                        }
                    }
                }
            }
        }

        Self { path, files }
    }

    /// The number of potential source files in the directory
    //    pub fn files_count(&self) -> usize {
    //        self.files.len()
    //    }

    /// Find all files matching the given suffix.
    pub fn filter_suffix(
        &self,
        suffix: &str,
        lang: Ustr,
        files: &mut Vec<(PathBuf, Ustr)>, // path and lang
    ) {
        for (filename, f) in &self.files {
            if filename.as_str().ends_with(suffix) {
                files.push((f.clone(), lang));
            }
        }
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
