use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct File {}

#[derive(Default)]
pub struct Directory {
    pub files: HashMap<PathBuf, File>,
}

impl Directory {
    /// The number of potential source files in the directory
    pub fn files_count(&self) -> usize {
        self.files.len()
    }
}
