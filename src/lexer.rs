use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};

fn is_wordchar(c: char) -> bool {
    matches!(c, '0' ..= '9' | 'A' ..= 'Z' | 'a' ..= 'z' | '_')
}

pub struct Lexer<'a> {
    line: u32,
    path: &'a std::path::Path,

    input: &'a str,
    iter: std::iter::Peekable<std::str::CharIndices<'a>>,

    offset: usize,
    current: char,

    peeked: Token<'a>,  // ??? Could let users use Peekable
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a File) -> Self {
        let mut s = Self {
            line: 1,
            path: file.path(),
            input: file.as_str(),
            iter: file.as_str().char_indices().peekable(),
            peeked: Token::new(TokenKind::EOF, 0),
            offset: 0,
            current: '\x00',
        };
        s.scan_char();
        s.peeked = s.scan_token();
        s
    }

    pub fn decorate_error(&self, error: Error) -> Error {
        error.decorate(Some(self.path), self.line)
    }

    /// Consumes one character
    #[inline]
    fn scan_char(&mut self) {
        if let Some((index, c)) = self.iter.next() {
            self.offset = index;
            self.current = c;
        } else {
            self.offset = self.input.len();
            self.current = '\x00';
        }
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> TokenKind {
        self.peeked.kind.clone()
    }

    /// Peek at the next item, without consuming it
    pub fn peek_with_line(&self) -> (u32, TokenKind) {
        (self.peeked.line, self.peeked.kind.clone())
    }

    /// On input, self.current is the leading quote
    fn scan_quote(&mut self) -> Token<'a> {
        let start_line = self.line;
        self.scan_char();  // consume leading quote
        let start_offset = self.offset;
        loop {
            match self.current {
                '\x00' => {
                     // Unterminated string
                     return Token {
                         line: start_line,
                         kind: TokenKind::EOF,
                     };
                }
                '"' => {
                    let end_offset = self.offset;
                    self.scan_char();
                    let s = &self.input[start_offset..end_offset];
                    return Token {
                        line: start_line,
                        kind: TokenKind::String(s),
                    };
                }
                _ => {}
            }
            self.scan_char();
        }
    }

    fn scan_colon(&mut self) -> Token<'a> {
        let start_line = self.line;
        self.scan_char();
        if self.current == '=' {
            self.scan_char();
            Token {
                line: start_line,
                kind: TokenKind::Assign,
            }
        } else {
            Token {
               line: start_line,
               kind: TokenKind::Colon,
            }
        }
    }

    fn scan_equal(&mut self) -> Token<'a> {
        let start_line = self.line;
        self.scan_char();
        if self.current == '>' {
            self.scan_char();
            Token {
                line: start_line,
                kind: TokenKind::Arrow,
            }
        } else {
            Token {
               line: start_line,
               kind: TokenKind::Equal,
            }
        }
    }

    fn skip_non_tokens(&mut self) {
        loop {
            match self.current {
                '\n' => self.line += 1,
                ' ' | '\t' | '\r' => {},
                '-' => {
                    if let Some((_, '-')) = self.iter.peek() {
                        loop {
                            self.scan_char();
                            match self.current {
                                '\n' | '\x00' => break,
                                _ => {}
                            }
                        }
                    } else {
                       break
                    }
                }
                _ => break,
            }
            self.scan_char();
        }
    }

    fn scan_identifier(&mut self) -> Token<'a> {
        let start_line = self.line;
        let start_offset = self.offset;
        loop {
            self.scan_char();
            if !is_wordchar(self.current) {
                break;
            }
        }
        let n = &self.input[start_offset..self.offset];
        let lower = n.to_lowercase();
        let kind = match lower.as_str() {
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
            _ => TokenKind::Identifier(lower),
        };
        Token {
            line: start_line,
            kind,
        }
    }

    fn scan_token(&mut self) -> Token<'a> {
        self.skip_non_tokens();
        let start_line = self.line;
        let kind = match self.current {
            '\x00' => TokenKind::EOF,
            '(' => TokenKind::OpenParenthesis,
            ')' => TokenKind::CloseParenthesis,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '|' => TokenKind::Pipe,
            '&' => TokenKind::Ampersand,
            '\'' => TokenKind::Tick,
            '-' => TokenKind::Minus,  // comments handled in skip_non_tokens
            '"' => return self.scan_quote(),
            ':' => return self.scan_colon(),
            '=' => return self.scan_equal(),
            c if is_wordchar(c) => return self.scan_identifier(),
            c  => TokenKind::InvalidChar(c),
        };

        let token = Token {
            line: start_line,
            kind,
        };
        self.scan_char();
        token
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    /// Consume the next token in the stream
    fn next(&mut self) -> Option<Self::Item> {
        let mut p = self.scan_token();
        std::mem::swap(&mut self.peeked, &mut p);
        if p.kind == TokenKind::EOF {
            None
        } else {
            Some(p)
        }
    }
}
