type Result<R> = std::result::Result<R, String>;

pub struct Lexer<'a> {
    current: usize,
    buffer: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Identifier(&'a str),
    Minus,
    Number(&'a str),
    Semicolon,
    String(&'a str),
    With,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a str) -> Self {
        Self {
            current: 0,
            buffer,
        }
    }

    /// Consumes bytes while a predicate evaluates to true.  The first
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

    /// Skip all characters until the start of the next line
    fn skip_till_end_of_line(&mut self) {
        for c in self.buffer[self.current..].chars() {
            self.current += c.len_utf8();
            match c {
                '\n' => return (),
                _    => {},
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Token> {
        let tk = loop {
            let tk = match self.buffer[self.current..].chars().next() {
                None      => Err("Unexpected end of string".to_string()),
                Some(';') => Ok(Token::Semicolon),
                Some('"') => Ok(Token::String(self.take_while(|c| c != '"'))),
                Some('-') => {
                    if self.buffer[self.current..].starts_with("--") {
                        self.skip_till_end_of_line();
                        continue;
                    } else {
                        Ok(Token::Minus)
                    }
                },
                Some(c) if c.is_whitespace() => {
                    let _ = self.take_while(|ch| ch.is_whitespace());
                    continue;
                },
                Some(c) if c.is_ascii_digit() =>
                    Ok(Token::Number(self.take_while(|c| c.is_ascii_digit()))),
                Some(c) if c.is_alphanumeric() => {
                    match self.take_while(|c| c == '_' || c.is_alphanumeric()) {
                        // ??? Should check case insensitive
                        "with" => Ok(Token::With),
                        t      => Ok(Token::Identifier(t)),
                    }
                },
                Some(c) => Err(format!("Unexpected token {}", c)),
            };
            break tk;
        };
        match &tk {
            Err(e) => println!("ERROR: {}", e),
            Ok(t)  => println!("{:?}", t),
        }
        tk
    }

}
