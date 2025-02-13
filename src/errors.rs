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

    #[error("Already declared")]
    AlreadyDeclared,

    #[error("Wrong use of &")]
    WrongAmpersand,

    #[error("{0} not found")]
    NotFound(String),

    #[error("{0} while reading {1}")]
    IoWithPath(std::io::Error, std::path::PathBuf),

    #[error("{source}")]
    Io {
        #[from]
        source: std::io::Error,
        //  backtrace: std::backtrace::Backtrace,
    },

    #[error("{source}")]
    FmtIo {
        #[from]
        source: std::fmt::Error,
    },

    #[error("Invalid graph node type {0}")]
    InvalidGraphNode(String),

    #[error("When clause can never match")]
    UselessWhenClause,
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

    pub fn not_found<T: std::fmt::Display>(name: T) -> Self {
        Error::NotFound(name.to_string())
    }
}
