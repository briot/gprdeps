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

    pub fn as_bytes(&self) -> &[u8] {
        self.buffer.as_bytes()
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
