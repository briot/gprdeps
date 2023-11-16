pub type Result<R> = std::result::Result<R, Error>;

pub struct Error {
    pub msg: String,
    path: Option<std::path::PathBuf>,
    line: i32,
}

impl Error {
    pub fn new(path: &std::path::Path, line: i32, msg: String) -> Self {
        Self {
            msg,
            line,
            path: Some(path.to_owned()),
        }
    }

    pub fn decorate(self, path: &std::path::Path, line: i32) -> Self {
        let p = if self.path.is_none() {
            Some(path.to_owned())
        } else {
            self.path
        };
        Self {
            msg: self.msg,
            line: if self.line <= 0 { line } else { self.line },
            path: p,
        }
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self {
            msg: value,
            path: None,
            line: 0,
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error({:?}:{} {})", self.path, self.line, self.msg)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            None => write!(f, "???: {}", self.msg),
            Some(p) => write!(f, "{}:{} {}", p.display(), self.line, self.msg),
        }
    }
}
