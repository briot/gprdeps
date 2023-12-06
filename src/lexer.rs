use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};
use ustr::Ustr;

fn is_wordchar(c: char) -> bool {
    matches!(c, '0' ..= '9' | 'A' ..= 'Z' | 'a' ..= 'z' | '_')
}

pub struct Lexer<'a> {
    path: std::path::PathBuf,
    input: &'a mut str,

    line: u32,
    offset: usize,
    current: char,
    // The next character to process, the source line it is at, and the
    // offset at which we read it.
    peeked: Token, // ??? Could let users use Peekable
                   // One symbol ahead
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a mut File) -> Self {
        let path = file.path().to_owned();
        let f = file.as_mut_str();
        let mut s = Self {
            path,
            line: 1,
            current: f.chars().next().unwrap(),
            input: f,
            offset: 0,
            peeked: Token::new(TokenKind::EndOfFile, 0),
        };
        s.peeked = s.scan_token();
        s
    }

    pub fn error_with_location(&self, error: Error) -> Error {
        Error::WithLocation {
            path: self.path.clone(),
            line: self.line,
            error: Box::new(error),
        }
    }

    /// Consumes one character
    #[inline]
    fn scan_char(&mut self) {
        self.offset += self.current.len_utf8();
        match self.input[self.offset..].chars().next() {
            None => self.current = '\x00',
            Some(c) => self.current = c,
        }
    }

    #[inline]
    fn peek_char(&mut self) -> Option<char> {
        self.input[self.offset..].chars().next()
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
    fn scan_quote(&mut self) -> Token {
        let start_line = self.line;
        self.scan_char(); // consume leading quote
        let start_offset = self.offset;
        loop {
            match self.current {
                '\x00' => {
                    // Unterminated string
                    return Token {
                        line: start_line,
                        kind: TokenKind::EndOfFile,
                    };
                }
                '"' => {
                    let end_offset = self.offset;
                    self.scan_char();
                    let s = Ustr::from(&self.input[start_offset..end_offset]);
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

    fn scan_colon(&mut self) -> Token {
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

    fn scan_equal(&mut self) -> Token {
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
                ' ' | '\t' | '\r' => {}
                '-' => {
                    if let Some('-') = self.peek_char() {
                        loop {
                            self.scan_char();
                            match self.current {
                                '\n' | '\x00' => break,
                                _ => {}
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
            self.scan_char();
        }
    }

    fn scan_identifier(&mut self) -> Token {
        let start_line = self.line;
        let start_offset = self.offset;
        loop {
            self.scan_char();
            if !is_wordchar(self.current) {
                break;
            }
        }
        let kind = {
            let n: &mut str = &mut self.input[start_offset..self.offset];
            n.make_ascii_lowercase();
            match &*n {
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
                _ => {
                    // We can't just do ASCII lower-case, but instead need to do full conversion
                    // to lower case here.
                    TokenKind::Identifier(Ustr::from(&n.to_lowercase()))
                }
            }
        };
        Token {
            line: start_line,
            kind,
        }
    }

    fn scan_token(&mut self) -> Token {
        self.skip_non_tokens();
        let start_line = self.line;
        let kind = match self.current {
            '\x00' => {
                return Token {
                    line: start_line,
                    kind: TokenKind::EndOfFile,
                };
            }
            '(' => TokenKind::OpenParenthesis,
            ')' => TokenKind::CloseParenthesis,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '|' => TokenKind::Pipe,
            '&' => TokenKind::Ampersand,
            '\'' => TokenKind::Tick,
            '-' => TokenKind::Minus, // comments handled in skip_non_tokens
            '"' => return self.scan_quote(),
            ':' => return self.scan_colon(),
            '=' => return self.scan_equal(),
            c if is_wordchar(c) => return self.scan_identifier(),
            c => TokenKind::InvalidChar(c),
        };

        let token = Token {
            line: start_line,
            kind,
        };
        self.scan_char();
        token
    }

    /// Get the next token, failing with error on end of file
    pub fn safe_next(&mut self) -> Result<Token, Error> {
        self.next().ok_or(Error::UnexpectedEOF)
    }

    /// Consumes the next token from the lexer, and expect it to be a specific
    /// token.  Raises an error otherwise.
    pub fn expect(&mut self, token: TokenKind) -> Result<(), Error> {
        let n = self.safe_next()?;
        match n {
            tk if tk.kind == token => Ok(()),
            tk => Err(Error::wrong_token(token, tk)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// which is returned.
    pub fn expect_str(&mut self) -> Result<Ustr, Error> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::String(s) => Ok(s),
            _ => Err(Error::wrong_token("string", n)),
        }
    }

    /// Consumes the next token from the lexer, and expects it to be an identifier
    /// which is returned.  The identifier is always lower-cased.
    pub fn expect_identifier(&mut self) -> Result<Ustr, Error> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::Identifier(s) => Ok(s),
            _ => Err(Error::wrong_token("identifier", n)),
        }
    }

}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    /// Consume the next token in the stream
    fn next(&mut self) -> Option<Self::Item> {
        let mut p = self.scan_token();
        std::mem::swap(&mut self.peeked, &mut p);
        if p.kind == TokenKind::EndOfFile {
            None
        } else {
            Some(p)
        }
    }
}
