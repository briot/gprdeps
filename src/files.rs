pub struct File {
    path: std::path::PathBuf,
    buffer: String,
}

impl File {
    pub fn new(
        path: &std::path::Path,
    ) -> std::result::Result<Self, std::io::Error> {
        Ok(Self {
            path: path.to_owned(),
            buffer: std::fs::read_to_string(path)?,
        })
    }

    pub fn new_from_str(s: &str) -> Self {
        Self {
            path: std::path::Path::new(":memory:").to_owned(),
            buffer: s.to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
