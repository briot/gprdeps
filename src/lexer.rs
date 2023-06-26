type Result<R> = std::result::Result<R, String>;

pub struct Lexer<'a> {
    current: usize,
    buffer: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum Token {
    EOF,
    CloseParenthesis,
    End,
    Extends,
    For,
    Identifier(String),   // ??? Would be nice to reference the internal buffer
    Is,
    Minus,
    Null,
    Number(i32),
    OpenParenthesis,
    Package,
    Project,
    Semicolon,
    String(String),   //  Doesn't include the quotes themselves, but preserves "" for instance.
    Use,
    When,
    With,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a str) -> Self {
        Self {
            current: 0,
            buffer,
        }
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

    pub fn next_token(&mut self) -> Result<Token> {
        loop {
            let tk = match self.buffer[self.current..].chars().next() {
                None      => Ok(Token::EOF),
                Some('(') => { self.take('('); Ok(Token::OpenParenthesis) },
                Some(')') => { self.take(')'); Ok(Token::CloseParenthesis) },
                Some(';') => { self.take(';'); Ok(Token::Semicolon) },
                Some('"') => {
                    self.take('"');  // discard opening quote
                    let s = self.take_until(|c| c == '"');
                    Ok(Token::String(s.to_string()))
                },
                Some('-') => {
                    if self.buffer[self.current..].starts_with("--") {
                        self.skip_till_end_of_line();
                        continue;
                    }
                    // ??? Could also be negative number
                    Ok(Token::Minus)
                },
                Some(c) if c.is_whitespace() => {
                    let _ = self.take_while(|ch| ch.is_whitespace());
                    continue;
                },
                Some(c) if c.is_ascii_digit() => {
                    let s = self.take_while(|c| c.is_ascii_digit());
                    Ok(Token::Number(s.parse::<i32>().unwrap()))
                },
                Some(c) if c.is_alphanumeric() => {
                    match self.take_while(|c| c == '_' || c.is_alphanumeric()) {
                        // ??? Should check case insensitive
                        "end"     => Ok(Token::End),
                        "extends" => Ok(Token::Extends),
                        "for"     => Ok(Token::For),
                        "is"      => Ok(Token::Is),
                        "package" => Ok(Token::Package),
                        "project" => Ok(Token::Project),
                        "null"    => Ok(Token::Null),
                        "use"     => Ok(Token::Use),
                        "with"    => Ok(Token::With),
                        "when"    => Ok(Token::When),
                        t         => Ok(Token::Identifier(t.to_string())),
                    }
                },
                Some(c) => Err(format!("Invalid character {}", c)),
            };

            match &tk {
                Err(e) => println!("ERROR: {}", e),
                Ok(t)  => println!("{:?}", t),
            }
            return tk;
        };
    }

}
