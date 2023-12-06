use crate::errors::Error;

pub struct File {
    path: std::path::PathBuf,
    buffer: String,
}

impl File {
    pub fn new(
        path: &std::path::Path,
    ) -> std::result::Result<Self, Error> {
        Ok(Self {
            path: path.to_owned(),
            buffer: std::fs::read_to_string(path)?,
        })
    }

    #[cfg(test)]
    pub fn new_from_str(s: &str) -> Self {
        Self {
            path: std::path::Path::new(":memory:").to_owned(),
            buffer: s.to_string(),
        }
    }

    pub fn as_mut_str(&mut self) -> &mut str {
        self.buffer.as_mut_str()
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
