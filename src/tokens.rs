use ustr::Ustr;

/// This enum includes all possible tokens for all languages.
/// The actual lexers, though, will only return a subset of those tokens,
/// depending on the language.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    EndOfFile,
    Abstract,
    Aggregate,
    Ampersand,
    Arrow,
    Assign,
    Case,
    Character(char),
    CloseParenthesis,
    Colon,
    Comma,
    Dot,
    Equal,
    End,
    Extends,
    For,
    Function,
    Generic,
    GreaterThan,
    HashDefine,
    HashElse,
    HashEndif,
    HashIf,
    HashIfdef,
    HashIfndef,
    HashInclude,
    HashUndef,
    Identifier(Ustr), // lower-cased
    InvalidChar(char),
    Is,
    LessThan,
    Loop,
    Library,
    Limited,
    Minus,
    Null,
    OpenParenthesis,
    Others,
    Package,
    Pipe,
    Pragma,
    Private,
    Procedure,
    Project,
    Renames,
    Semicolon,
    Separate,
    String(Ustr), //  Doesn't include surrounding quotes, but preserves ""
    Tick,
    Type,
    Use,
    When,
    With,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::String(s) => write!(f, "String({})", s),
            TokenKind::Identifier(s) => write!(f, "Identifier({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub line: u32,
    pub kind: TokenKind,
}

impl Token {
    pub fn new(kind: TokenKind, line: u32) -> Self {
        Self { line, kind }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.kind, self.line)
    }
}
