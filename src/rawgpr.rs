/// A GPR file that hasn't been processed yet.  All we store here is the info we
/// extracted from the file itself, but we did not resolve paths, for instance.
/// Such an object is only valid as long as the scanner that generates it, since
/// it references memory from that scanner directly.

use crate::rawexpr::Statement;

pub struct RawGPR<'a> {
    pub path: &'a std::path::Path,
    pub imported: Vec<String>,
    pub name: String,
    pub is_abstract: bool,
    pub is_aggregate: bool,
    pub is_library: bool,
    pub extends: Option<String>,
    pub body: Vec<Statement>,
}

impl<'a> RawGPR<'a> {
    /// Create a new, mostly unset, GPR file
    pub fn new(path: &'a std::path::Path) -> Self {
        Self {
            path,
            imported: vec![],
            name: Default::default(),
            is_abstract: false,
            is_aggregate: false,
            is_library: false,
            extends: None,
            body: vec![],
        }
    }

    /// Resolve relative paths
    pub fn normalize_path(&self, path: &'a str) -> std::path::PathBuf {
        let mut p = self.path.parent().unwrap().join(path);
        p.set_extension("gpr");
        std::fs::canonicalize(p).unwrap()
    }
}