use crate::graph::NodeIndex;
use crate::rawexpr::StatementList;
/// A GPR file that hasn't been processed yet.  All we store here is the info we
/// extracted from the file itself, but we did not resolve paths, for instance.
/// Such an object is only valid as long as the scanner that generates it, since
/// it references memory from that scanner directly.
use std::path::PathBuf;
use ustr::Ustr;

#[derive(Default)]
pub struct RawGPR {
    pub path: std::path::PathBuf,
    pub imported: Vec<NodeIndex>,
    pub name: Ustr,
    pub is_abstract: bool,
    pub is_aggregate: bool,
    pub is_library: bool,
    pub extends: Option<NodeIndex>,
    pub body: StatementList,
}

impl RawGPR {
    /// Create a new, mostly unset, GPR file
    pub fn new(path: &std::path::Path) -> Self {
        Self {
            path: path.to_path_buf(),
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
    pub fn normalize_path(&self, path: &str) -> Result<PathBuf, String> {
        let mut p = self.path.parent().unwrap().join(path);
        p.set_extension("gpr");
        match std::fs::canonicalize(p) {
            Err(e) => Err(format!("{} {}", e, path)),
            Ok(p) => Ok(p),
        }
    }
}

impl std::fmt::Debug for RawGPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
