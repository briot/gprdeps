use ustr::Ustr;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    EOF,
    Abstract,
    Aggregate,
    Ampersand,
    Arrow,
    Assign,
    Case,
    CloseParenthesis,
    Colon,
    Comma,
    Dot,
    Equal,
    End,
    Extends,
    For,
    Identifier(Ustr), // lower-cased
    InvalidChar(char),
    Is,
    Library,
    Minus,
    Null,
    OpenParenthesis,
    Others,
    Package,
    Pipe,
    Project,
    Renames,
    Semicolon,
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

#[derive(Clone)]
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

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
