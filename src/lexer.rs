type Result<R> = std::result::Result<R, String>;

pub struct Lexer<'a> {
    current: usize,
    buffer: &'a str,
    peeked: Token,
}

#[derive(Debug, PartialEq)]
pub enum Token {
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
    Identifier(String),   // ??? Would be nice to reference the internal buffer
    InvalidChar(char),
    Is,
    Minus,
    Null,
    Number(i32),
    OpenParenthesis,
    Package,
    Pipe,
    Project,
    Semicolon,
    String(String),   //  Doesn't include the quotes themselves, but preserves "" for instance.
    Tick,
    Use,
    When,
    With,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a str) -> Self {
        let mut s = Self {
            current: 0,
            buffer,
            peeked: Token::EOF,
        };
        _ = s.next_token();
        s
    }

    /// Consumes one character
    fn take(&mut self, c: char) {
        self.current += c.len_utf8();
    }

    /// Consumes chars while a predicate evaluates to true.  The first
    /// character is always accepted, so should have been tested by the caller
    /// first.
    fn take_while<F>(&mut self, mut predicate: F) -> &str
        where F: FnMut(char) -> bool
    {
        let start = self.current;
        let mut chars = self.buffer[start..].chars();
        let c = chars.next().unwrap();
        self.current += c.len_utf8();

        for c in chars {
            if !predicate(c) {
                break;
            }
            self.current += c.len_utf8();
        }
        &self.buffer[start..self.current]
    }

    /// Consume chars until the predicate evaluates to true.  The current
    /// character is always skipped, and the last character when the predicates
    /// is True is also consumed, but not included in the result.
    fn take_until<F>(&mut self, mut predicate: F) -> &str
        where F: FnMut(char) -> bool
    {
        let start = self.current;
        let mut chars = self.buffer[start..].chars();
        let c = chars.next().unwrap();
        self.current += c.len_utf8();

        let mut last: usize = start;

        for c in chars {
            last = self.current;
            self.current += c.len_utf8();
            if predicate(c) {
                break;
            }
        }
        &self.buffer[start..last]
    }

    /// Skip all characters until the start of the next line
    fn skip_till_end_of_line(&mut self) {
        for c in self.buffer[self.current..].chars() {
            self.current += c.len_utf8();
            if c == '\n' {
                return
            }
        }
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> &Token {
        &self.peeked
    }

    /// Consume the next token in the stream
    pub fn next_token(&mut self) -> Result<Token> {
        // Now load the next one
        loop {
            let mut peeked = match self.buffer[self.current..].chars().next() {
                None      => Token::EOF,
                Some('(') => { self.take('('); Token::OpenParenthesis },
                Some(')') => { self.take(')'); Token::CloseParenthesis },
                Some(';') => { self.take(';'); Token::Semicolon },
                Some(',') => { self.take(','); Token::Comma },
                Some('.') => { self.take('.'); Token::Dot },
                Some('|') => { self.take('|'); Token::Pipe },
                Some('&') => { self.take('&'); Token::Ampersand },
                Some(':') => {
                    self.take(':');
                    match self.buffer[self.current..].chars().next() {
                        Some('=') => {self.take('='); Token::Assign },
                        Some(c)   => {self.take(c);   Token::InvalidChar(c) },
                        None      => Token::EOF,
                    }
                }
                Some('=') => {
                    self.take('=');
                    match self.buffer[self.current..].chars().next() {
                        Some('>') => { self.take('>'); Token::Arrow },
                        _         => Token::Equal,
                    }
                },
                Some('\'') => { self.take('\''); Token::Tick },
                Some('"') => {
                    self.take('"');  // discard opening quote
                    let s = self.take_until(|c| c == '"');
                    Token::String(s.to_string())
                },
                Some('-') => {
                    if self.buffer[self.current..].starts_with("--") {
                        self.skip_till_end_of_line();
                        continue;
                    }
                    // ??? Could also be negative number
                    Token::Minus
                },
                Some(c) if c.is_whitespace() => {
                    let _ = self.take_while(|ch| ch.is_whitespace());
                    continue;
                },
                Some(c) if c.is_ascii_digit() => {
                    let s = self.take_while(|c| c.is_ascii_digit());
                    Token::Number(s.parse::<i32>().unwrap())
                },
                Some(c) if c.is_alphanumeric() => {
                    match self.take_while(|c| c == '_' || c.is_alphanumeric()) {
                        // ??? Should check case insensitive
                        "case"    => Token::Case,
                        "end"     => Token::End,
                        "extends" => Token::Extends,
                        "for"     => Token::For,
                        "is"      => Token::Is,
                        "package" => Token::Package,
                        "project" => Token::Project,
                        "null"    => Token::Null,
                        "use"     => Token::Use,
                        "with"    => Token::With,
                        "when"    => Token::When,
                        t         => Token::Identifier(t.to_string()),
                    }
                },
                Some(c) => Token::InvalidChar(c),
            };

            std::mem::swap(&mut self.peeked, &mut peeked);

            match &peeked {
                Token::InvalidChar(c) => println!("ERROR: invalid character {}", c),
                t => println!("{:?}", t),
            }
            return Ok(peeked);
        }
    }

}
