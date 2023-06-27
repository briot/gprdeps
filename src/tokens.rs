#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    EOF,
    Ampersand,
    Arrow,
    Assign,
    Case,
    CloseParenthesis,
    Comma,
    Dot,
    Equal,
    End,
    Extends,
    For,
    Identifier(&'a [u8]),
    InvalidChar(u8),
    Is,
    Minus,
    Null,
    OpenParenthesis,
    Package,
    Pipe,
    Project,
    Semicolon,
    String(&'a [u8]),   //  Doesn't include the quotes themselves, but preserves "" for instance.
    Tick,
    Use,
    When,
    With,
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::String(s) | Token::Identifier(s) =>
                match std::str::from_utf8(s) {
                    Err(_)  => write!(f, "String(invalid-utf8, {:?})", s),
                    Ok(s)   => write!(f, "String({})", s),
                },
            _                => write!(f, "{:?}", self),
        }
    }
}
