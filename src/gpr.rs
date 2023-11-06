use crate::rawgpr::RawGPR;
use petgraph::graph::NodeIndex;

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
#[derive(Debug)]
pub struct GPR {
    path: std::path::PathBuf,
    _name: String,
    _is_abstract: bool,
    _is_aggregate: bool,
    _is_library: bool,
    pub imported: Vec<NodeIndex>,
    _types: std::collections::HashMap<String, Vec<String>>, // lower-cased name
}

impl GPR {
    pub fn new(raw: &RawGPR) -> Self {
        Self {
            path: raw.path.to_owned(),
            _name: raw.name.to_string(),
            _is_abstract: raw.is_abstract,
            _is_aggregate: raw.is_aggregate,
            _is_library: raw.is_library,
            imported: Default::default(),
            _types: Default::default(),
        }
    }

    pub fn resolve_deps(
        &mut self,
        map: &std::collections::HashMap<std::path::PathBuf, NodeIndex>,
        raw: &RawGPR,
    ) {
        self.imported = raw
            .imported
            .iter()
            .map(|p| map[&raw.normalize_path(p)])
            .collect();
    }

    pub fn path(&self) -> &std::path::PathBuf {
        // Assume the path can always be converted to str, since it was specified in a project
        // file anyway.
        &self.path
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)
    }
}
