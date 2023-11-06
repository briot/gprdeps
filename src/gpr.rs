use crate::rawgpr::RawGPR;

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
pub struct GPR {
    path: std::path::PathBuf,
    raw: RawGPR,
}

impl GPR {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            raw: Default::default(),
        }
    }

    pub fn set_raw(&mut self, raw: RawGPR) {
        self.raw = raw;
    }

    /// Return the path of the project file
    pub fn path(&self) -> &std::path::PathBuf {
        &self.path
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)
    }
}
