use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};

fn is_wordchar(c: u8) -> bool {
    matches!(c, b'0' ..= b'9' | b'A' ..= b'Z' | b'a' ..= b'z' | b'_')
}

pub struct Lexer<'a> {
    current: usize,
    line: i32,
    path: &'a std::path::Path,
    buffer: &'a [u8], // The file, as bytes.  All keywords are ASCII
    peeked: Token<'a>,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a File) -> Self {
        let mut s = Self {
            current: 0,
            line: 1,
            path: file.path(),
            buffer: file.as_bytes(),
            peeked: Token::new(TokenKind::EOF, -1),
        };
        _ = s.next();
        s
    }

    pub fn decorate_error(&self, error: Error) -> Error {
        error.decorate(self.path, self.line)
    }

    pub fn error(&self, msg: String) -> Error {
        Error::new(self.path, self.line, msg)
    }

    pub fn path(&self) -> &'a std::path::Path {
        self.path
    }

    /// Consumes one character
    #[inline]
    fn take(&mut self) {
        self.current += 1;
    }

    /// Consumes chars while a predicate evaluates to true.  The first
    /// character is always accepted, so should have been tested by the caller
    /// first.
    fn take_while<F>(&mut self, mut predicate: F) -> &'a [u8]
    where
        F: FnMut(u8) -> bool,
    {
        let start = self.current;
        self.current += 1; //  consume first character, always

        for c in self.current..self.buffer.len() {
            if !predicate(self.buffer[c]) {
                self.current = c;
                return &self.buffer[start..c];
            }
        }
        self.current = self.buffer.len();
        &self.buffer[start..]
    }

    /// Consume chars until the predicate evaluates to true.  The current
    /// character is always skipped, and the last character when the predicates
    /// is True is also consumed, but not included in the result.
    fn take_until<F>(&mut self, mut predicate: F) -> &'a [u8]
    where
        F: FnMut(u8) -> bool,
    {
        let start = self.current;

        for c in self.current..self.buffer.len() {
            if predicate(self.buffer[c]) {
                self.current = c + 1;
                return &self.buffer[start..c];
            }
        }
        self.current = self.buffer.len();
        &self.buffer[start..]
    }

    /// Skip all characters until the start of the next line
    fn skip_till_end_of_line(&mut self) {
        for c in self.current..self.buffer.len() {
            if self.buffer[c] == b'\n' {
                self.current = c + 1;
                self.line += 1;
                return;
            }
        }
        self.current = self.buffer.len();
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> TokenKind {
        self.peeked.kind.clone()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    /// Consume the next token in the stream
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let start_line = self.line;
            let peeked = match self.buffer.get(self.current) {
                None => TokenKind::EOF,
                Some(b'(') => {
                    self.take();
                    TokenKind::OpenParenthesis
                }
                Some(b')') => {
                    self.take();
                    TokenKind::CloseParenthesis
                }
                Some(b';') => {
                    self.take();
                    TokenKind::Semicolon
                }
                Some(b',') => {
                    self.take();
                    TokenKind::Comma
                }
                Some(b'.') => {
                    self.take();
                    TokenKind::Dot
                }
                Some(b'|') => {
                    self.take();
                    TokenKind::Pipe
                }
                Some(b'&') => {
                    self.take();
                    TokenKind::Ampersand
                }
                Some(b':') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'=') => {
                            self.take();
                            TokenKind::Assign
                        }
                        None => return None,
                        _ => {
                            self.take();
                            TokenKind::Colon
                        }
                    }
                }
                Some(b'=') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'>') => {
                            self.take();
                            TokenKind::Arrow
                        }
                        _ => TokenKind::Equal,
                    }
                }
                Some(b'\'') => {
                    self.take();
                    TokenKind::Tick
                }
                Some(b'"') => {
                    self.take(); // discard opening quote
                    let s = self.take_until(|c| c == b'"');
                    match std::str::from_utf8(s) {
                        Err(_) => panic!("Invalid UTF8 {:?}", s),
                        Ok(s) => TokenKind::String(s),
                    }
                }
                Some(b'-') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(&b'-') => {
                            self.take();
                            self.skip_till_end_of_line();
                            continue;
                        }
                        _ => TokenKind::Minus,
                    }
                }
                Some(b'\n') => {
                    self.take();
                    self.line += 1;
                    continue;
                }
                Some(b' ') | Some(b'\t') | Some(b'\r') => {
                    self.take();
                    continue;
                }
                Some(&c) if is_wordchar(c) => {
                    let n = std::str::from_utf8(self.take_while(is_wordchar))
                        .unwrap()
                        .to_lowercase();
                    match n.as_str() {
                        "abstract" => TokenKind::Abstract,
                        "aggregate" => TokenKind::Aggregate,
                        "case" => TokenKind::Case,
                        "end" => TokenKind::End,
                        "extends" => TokenKind::Extends,
                        "for" => TokenKind::For,
                        "is" => TokenKind::Is,
                        "library" => TokenKind::Library,
                        "others" => TokenKind::Others,
                        "package" => TokenKind::Package,
                        "project" => TokenKind::Project,
                        "renames" => TokenKind::Renames,
                        "type" => TokenKind::Type,
                        "null" => TokenKind::Null,
                        "use" => TokenKind::Use,
                        "with" => TokenKind::With,
                        "when" => TokenKind::When,
                        _ => TokenKind::Identifier(n),
                    }
                }
                Some(c) => TokenKind::InvalidChar(*c),
            };

            let mut p = Token::new(peeked, start_line);
            std::mem::swap(&mut self.peeked, &mut p);
            return Some(p);
        }
    }
}
