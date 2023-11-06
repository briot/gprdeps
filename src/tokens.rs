#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind<'a> {
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
    Identifier(String), // lower-cased
    InvalidChar(u8),
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
    String(&'a str), //  Doesn't include surrounding quotes, but preserves ""
    Tick,
    Type,
    Use,
    When,
    With,
}

impl<'a> std::fmt::Display for TokenKind<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::String(s) => write!(f, "String({})", s),
            TokenKind::Identifier(s) => write!(f, "Identifier({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Clone)]
pub struct Token<'a> {
    line: i32,
    pub kind: TokenKind<'a>,
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind<'a>, line: i32) -> Self {
        Self { line, kind }
    }
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.kind, self.line)
    }
}

impl<'a> std::fmt::Debug for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
