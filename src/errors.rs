use ustr::Ustr;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{path}:{line} {error}")]
    WithLocation {
        path: std::path::PathBuf,
        line: u32,
        error: Box<Error>,
    },

    #[error("{path}: {error}")]
    WithPath {
        path: std::path::PathBuf,
        error: Box<Error>,
    },

    #[error("Unexpected end of file")]
    UnexpectedEOF,

    #[error("Expected {expected}, got {got}")]
    WrongToken { expected: String, got: String },

    #[error("Cannot parse {path}, language {lang}")]
    CannotParse {
        path: std::path::PathBuf,
        lang: String,
    },

    #[error("Invalid package name {0}")]
    InvalidPackageName(Ustr),

    #[error("Invalid attribute name {0}")]
    InvalidAttribute(Ustr),

    #[error("Invalid attribute name {0}({1})")]
    InvalidAttributeWithIndex(Ustr, Ustr),

    #[error("Invalid attribute name {0}(others)")]
    InvalidAttributeWithOthers(Ustr),

    #[error("Variable in case statement must be a string")]
    VariableMustBeString,

    #[error(
        "Scenario variable {0} already defined with another set of values"
    )]
    ScenarioTwice(Ustr),

    #[error("Wrong number of indices for {0}")]
    WrongIndexes(Ustr),

    #[error("Unknown function {0}")]
    UnknownFunction(Ustr),

    #[error("`Project'` must be followed by attribute name")]
    MissingAttributeNameAfterProject,

    #[error("Name {0} should have been {1}")]
    MismatchEndName(Ustr, Ustr),

    #[error("Not a static string")]
    NotStaticString,

    #[error("Lists can only contain strings")]
    ListCanOnlyContainStrings,

    #[error("Wrong use of &")]
    WrongAmpersand,

    #[cfg(test)]
    #[error("Cannot merge two values, same scenario occurs twice")]
    CannotMerge,

    #[error("Cannot combine scenarios")]
    CannotCombineScenarios,

    #[error("{0} not found")]
    NotFound(String),

    #[cfg(test)]
    #[error("Values do not have the same type {left} and {right}")]
    TypeMismatch { left: String, right: String },

    #[error("{0} while reading {1}")]
    IoWithPath(std::io::Error, std::path::PathBuf),

    #[error("{source}")]
    Io {
        #[from]
        source: std::io::Error,
        //  backtrace: std::backtrace::Backtrace,
    },

    #[error("Invalid graph node type {0}")]
    InvalidGraphNode(String),
}

impl Error {
    pub fn wrong_token<T1, T2>(expected: T1, got: T2) -> Self
    where
        T1: std::fmt::Display,
        T2: std::fmt::Display,
    {
        Error::WrongToken {
            expected: expected.to_string(),
            got: got.to_string(),
        }
    }

    #[cfg(test)]
    pub fn type_mismatch<T1, T2>(left: T1, right: T2) -> Self
    where
        T1: std::fmt::Debug,
        T2: std::fmt::Debug,
    {
        Error::TypeMismatch {
            left: format!("{:?}", left),
            right: format!("{:?}", right),
        }
    }

    pub fn not_found<T: std::fmt::Display>(name: T) -> Self {
        Error::NotFound(name.to_string())
    }
}

// pub struct Error {
//     pub msg: String,
//     path: Option<std::path::PathBuf>,
//     line: u32,
// }

// impl Error {
//     pub fn decorate(self, path: Option<&std::path::Path>, line: u32) -> Self {
//         let p = match (&self.path, path) {
//             (Some(_), _) => self.path,
//             (None, None) => self.path,
//             (None, Some(p)) => Some(p.to_owned()),
//         };
//         Self {
//             msg: self.msg,
//             line: if self.line == 0 { line } else { self.line },
//             path: p,
//         }
//     }
// }
//
// impl std::error::Error for Error {}
//
// impl From<String> for Error {
//     fn from(value: String) -> Self {
//         Self {
//             msg: value,
//             path: None,
//             line: 0,
//         }
//     }
// }

// impl std::fmt::Debug for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Error({:?}:{} {})", self.path, self.line, self.msg)
//     }
// }

// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match &self.path {
//             None => write!(f, "???: {}", self.msg),
//             Some(p) => write!(f, "{}:{} {}", p.display(), self.line, self.msg),
//         }
//     }
// }
