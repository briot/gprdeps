pub type Result<R> = std::result::Result<R, Error>;

pub struct Error {
    pub msg: String,
    path: std::path::PathBuf,
    line: i32,
}

impl Error {
    pub fn new(path: &std::path::Path, line: i32, msg: String) -> Self {
        Self {
            msg,
            line,
            path: path.to_owned(),
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error({}:{} {})",
            self.path.display(),
            self.line,
            self.msg
        )
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} {}", self.path.display(), self.line, self.msg)
    }
}
