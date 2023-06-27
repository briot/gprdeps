type Result<R> = std::result::Result<R, String>;

pub struct Lexer<'a> {
    current: usize,
    buffer: &'a [u8],   // The file, as bytes.  All keywords are ASCII
    peeked: Token<'a>,
}

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

fn is_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n')
}

fn is_alphanumeric(c: u8) -> bool {
    matches!(c, b'0' ..= b'9' | b'A' ..= b'Z' | b'a' ..= b'z')
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a str) -> Self {
        let mut s = Self {
            current: 0,
            buffer: buffer.as_bytes(),
            peeked: Token::EOF,
        };
        _ = s.next_token();
        s
    }

    /// Consumes one character
    fn take(&mut self) {
        self.current += 1;
    }

    /// Consumes chars while a predicate evaluates to true.  The first
    /// character is always accepted, so should have been tested by the caller
    /// first.
    fn take_while<F>(&mut self, mut predicate: F) -> &'a [u8]
        where F: FnMut(u8) -> bool
    {
        let start = self.current;
        self.current += 1;  //  consume first character, always

        for c in self.current .. self.buffer.len() {
            if !predicate(self.buffer[c]) {
                self.current = c;
                return &self.buffer[start .. c];
            }
        }
        self.current = self.buffer.len();
        &self.buffer[start..]
    }

    /// Consume chars until the predicate evaluates to true.  The current
    /// character is always skipped, and the last character when the predicates
    /// is True is also consumed, but not included in the result.
    fn take_until<F>(&mut self, mut predicate: F) -> &'a [u8]
        where F: FnMut(u8) -> bool
    {
        let start = self.current;
        self.current += 1;  //  consume first character, always

        for c in self.current .. self.buffer.len() {
            if predicate(self.buffer[c]) {
                self.current = c + 1;
                return &self.buffer[start .. c];
            }
        }
        self.current = self.buffer.len();
        &self.buffer[start ..]
    }

    /// Skip all characters until the start of the next line
    fn skip_till_end_of_line(&mut self) {
        for c in self.current .. self.buffer.len() {
            if self.buffer[c] == b'\n' {
                self.current = c + 1;
                return;
            }
        }
        self.current = self.buffer.len();
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> &Token<'a> {
        &self.peeked
    }

    /// Consume the next token in the stream
    pub fn next_token(&mut self) -> Result<Token<'a>> {
        // Now load the next one
        loop {
            let mut peeked = match self.buffer.get(self.current) {
                None => Token::EOF,
                Some(b'(') => { self.take(); Token::OpenParenthesis },
                Some(b')') => { self.take(); Token::CloseParenthesis },
                Some(b';') => { self.take(); Token::Semicolon },
                Some(b',') => { self.take(); Token::Comma },
                Some(b'.') => { self.take(); Token::Dot },
                Some(b'|') => { self.take(); Token::Pipe },
                Some(b'&') => { self.take(); Token::Ampersand },
                Some(b':') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'=') => {self.take(); Token::Assign },
                        Some(c)    => {self.take(); Token::InvalidChar(*c) },
                        None       => Token::EOF,
                    }
                },
                Some(b'=') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'>') => { self.take(); Token::Arrow },
                        _          => Token::Equal,
                    }
                },
                Some(b'\'') => { self.take(); Token::Tick },
                Some(b'"') => {
                    self.take();  // discard opening quote
                    let s = self.take_until(|c| c == b'"');
                    Token::String(s)
                },
                Some(b'-') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(&b'-') => {
                            self.take();
                            self.skip_till_end_of_line();
                            continue;
                        },
                        // ??? Could also be negative number
                        _ => Token::Minus,
                    }
                },
                Some(&c) if is_whitespace(c) => {
                    let _ = self.take_while(is_whitespace);
                    continue;
                },
                Some(&c) if is_alphanumeric(c) => {
                    match self.take_while(|c| c == b'_' || is_alphanumeric(c)) {
                        // ??? Should check case insensitive
                        b"case"    => Token::Case,
                        b"end"     => Token::End,
                        b"extends" => Token::Extends,
                        b"for"     => Token::For,
                        b"is"      => Token::Is,
                        b"package" => Token::Package,
                        b"project" => Token::Project,
                        b"null"    => Token::Null,
                        b"use"     => Token::Use,
                        b"with"    => Token::With,
                        b"when"    => Token::When,
                        t          => Token::Identifier(t),
                    }
                },
                Some(c) => Token::InvalidChar(*c),
            };

            std::mem::swap(&mut self.peeked, &mut peeked);

            match &peeked {
                Token::InvalidChar(c) => println!("ERROR: invalid character {}", c),
                t => println!("{}", t),
            }
            return Ok(peeked);
        }
    }

}
