use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};

fn is_alphanumeric(c: u8) -> bool {
    matches!(c, b'0' ..= b'9' | b'A' ..= b'Z' | b'a' ..= b'z')
}

pub struct Lexer<'a> {
    current: usize,
    line: i32,
    file: &'a File,
    buffer: &'a [u8],   // The file, as bytes.  All keywords are ASCII
    peeked: Token<'a>,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a File) -> Self {
        let mut s = Self {
            current: 0,
            line: 1,
            file,
            buffer: file.as_bytes(),
            peeked: Token::new(TokenKind::EOF, 0),
        };
        _ = s.next();
        s
    }

    pub fn error(&self, msg: String) -> Error {
        Error::new(self.file, self.line, msg)
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
                self.line += 1;
                return;
            }
        }
        self.current = self.buffer.len();
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> &TokenKind<'a> {
        &self.peeked.kind
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    /// Consume the next token in the stream
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let start_line = self.line;
            let peeked = match self.buffer.get(self.current) {
                None       => TokenKind::EOF,
                Some(b'(') => { self.take(); TokenKind::OpenParenthesis },
                Some(b')') => { self.take(); TokenKind::CloseParenthesis },
                Some(b';') => { self.take(); TokenKind::Semicolon },
                Some(b',') => { self.take(); TokenKind::Comma },
                Some(b'.') => { self.take(); TokenKind::Dot },
                Some(b'|') => { self.take(); TokenKind::Pipe },
                Some(b'&') => { self.take(); TokenKind::Ampersand },
                Some(b':') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'=') => {self.take(); TokenKind::Assign },
                        None       => return None,
                        _          => {self.take(); TokenKind::Colon },
                    }
                },
                Some(b'=') => {
                    self.take();
                    match self.buffer.get(self.current) {
                        Some(b'>') => { self.take(); TokenKind::Arrow },
                        _          => TokenKind::Equal,
                    }
                },
                Some(b'\'') => { self.take(); TokenKind::Tick },
                Some(b'"') => {
                    self.take();  // discard opening quote
                    let s = self.take_until(|c| c == b'"');
                    TokenKind::String(s)
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
                        _ => TokenKind::Minus,
                    }
                },
                Some(b'\n') => {
                    self.take();
                    self.line += 1;
                    continue;
                }
                Some(b' ') | Some(b'\t') | Some(b'\r') => {
                    self.take();
                    continue;
                },
                Some(&c) if is_alphanumeric(c) => {
                    match self.take_while(|c| c == b'_' || is_alphanumeric(c)) {
                        // ??? Should check case insensitive
                        b"abstract"  => TokenKind::Abstract,
                        b"aggregate" => TokenKind::Aggregate,
                        b"case"      => TokenKind::Case,
                        b"end"       => TokenKind::End,
                        b"extends"   => TokenKind::Extends,
                        b"for"       => TokenKind::For,
                        b"is"        => TokenKind::Is,
                        b"library"   => TokenKind::Library,
                        b"others"    => TokenKind::Others,
                        b"package"   => TokenKind::Package,
                        b"project"   => TokenKind::Project,
                        b"renames"   => TokenKind::Renames,
                        b"type"      => TokenKind::Type,
                        b"null"      => TokenKind::Null,
                        b"use"       => TokenKind::Use,
                        b"with"      => TokenKind::With,
                        b"when"      => TokenKind::When,
                        t            => TokenKind::Identifier(t),
                    }
                },
                Some(c) => TokenKind::InvalidChar(*c),
            };

            let mut p = Token::new(peeked, start_line);
            std::mem::swap(&mut self.peeked, &mut p);

//            match p.kind {
//                TokenKind::InvalidChar(c) => println!("ERROR: invalid character {}", c),
//                _ => println!("{}", p),
//            }

            return match p.kind {
                TokenKind::EOF => None,
                _              => Some(p),
            };
        }
    }

}
