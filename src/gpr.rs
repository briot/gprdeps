use crate::environment::{Environment, GPRIndex};
use crate::rawgpr::{Abstract, Aggregate, Library, RawGPR};

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it needs an Environment
/// object to resolve paths.
pub struct GPR {
    path: std::path::PathBuf,
    name: String,
    _is_abstract: Abstract,
    _is_aggregate: Aggregate,
    _is_library: Library,
    _imported: Vec<GPRIndex>,
    _types: std::collections::HashMap<String, Vec<String>>, // lower-cased name
}

impl GPR {
    pub fn new(env: &Environment, raw: RawGPR) -> Self {
        Self {
            path: raw.path.to_owned(),
            name: raw.name.to_string(),
            _is_abstract: raw.is_abstract,
            _is_aggregate: raw.is_aggregate,
            _is_library: raw.is_library,
            _imported: raw
                .imported
                .iter()
                .map(|p| env.map[&raw.normalize_path(p)])
                .collect(),
            _types: Default::default(),
        }
    }

    pub fn path(&self) -> &std::path::PathBuf {
        // Assume the path can always be converted to str, since it was specified in a project
        // file anyway.
        &self.path
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}:(name={})", self.path, self.name)
    }
}
