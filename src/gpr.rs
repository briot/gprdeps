use crate::environment::{Environment, GPRIndex};
use crate::scanner::{Abstract, Aggregate, Library, RawGPR};

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it needs an Environment
/// object to resolve paths.
pub struct GPR {
    path: std::path::PathBuf,
    name: String,
    is_abstract: Abstract,
    is_aggregate: Aggregate,
    is_library: Library,
    imported: Vec<GPRIndex>,
}

impl GPR {
    pub fn new(env: &Environment, raw: RawGPR) -> Self {
        Self {
            path: raw.path.to_owned(),
            name: raw.name.to_string(),
            is_abstract: raw.is_abstract,
            is_aggregate: raw.is_aggregate,
            is_library: raw.is_library,
            imported: raw
                .imported
                .iter()
                .map(|p| env.map[&raw.normalize_path(p)])
                .collect(),
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
