use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};
use std::path::PathBuf;
use ustr::Ustr;

fn is_wordchar(c: char) -> bool {
    matches!(c, '0' ..= '9' | 'A' ..= 'Z' | 'a' ..= 'z' | '_')
}

pub struct AdaLexerOptions {
    pub aggregate_is_keyword: bool,
}

pub struct AdaLexer<'a> {
    path: PathBuf,
    input: &'a mut str,
    options: AdaLexerOptions,

    line: u32,
    offset: usize,
    current: char,
    // The next character to process, the source line it is at, and the
    // offset at which we read it.
    peeked: Token, // ??? Could let users use Peekable
                   // One symbol ahead
}

impl<'a> AdaLexer<'a> {
    pub fn new(file: &'a mut File, options: AdaLexerOptions) -> Self {
        let path = file.path().to_owned();
        let f = file.as_mut_str();
        let mut s = Self {
            path,
            line: 1,
            current: f.chars().next().unwrap(),
            input: f,
            offset: 0,
            options,
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
        self.input[self.offset + self.current.len_utf8()..].chars().next()
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
    fn scan_quote(&mut self) -> TokenKind {
        self.scan_char(); // consume leading quote
        let start_offset = self.offset;
        loop {
            match self.current {
                // Unterminated string
                '\x00' => return TokenKind::EndOfFile,
                '"' => {
                    let end_offset = self.offset;
                    self.scan_char();
                    let s = Ustr::from(&self.input[start_offset..end_offset]);
                    return TokenKind::String(s);
                }
                _ => {}
            }
            self.scan_char();
        }
    }

    fn scan_colon(&mut self) -> TokenKind {
        self.scan_char();
        if self.current == '=' {
            self.scan_char();
            TokenKind::Assign
        } else {
            TokenKind::Colon
        }
    }

    fn scan_equal(&mut self) -> TokenKind {
        self.scan_char();
        if self.current == '>' {
            self.scan_char();
            TokenKind::Arrow
        } else {
            TokenKind::Equal
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

    fn scan_identifier(&mut self) -> TokenKind {
        let start_offset = self.offset;
        loop {
            self.scan_char();
            if !is_wordchar(self.current) {
                break;
            }
        }
        let n: &mut str = &mut self.input[start_offset..self.offset];
        n.make_ascii_lowercase();
        match &*n {
            "abstract" => TokenKind::Abstract,
            "aggregate" if self.options.aggregate_is_keyword =>
                TokenKind::Aggregate,
            "case" => TokenKind::Case,
            "end" => TokenKind::End,
            "extends" => TokenKind::Extends,
            "for" => TokenKind::For,
            "function" => TokenKind::Function,
            "generic" => TokenKind::Generic,
            "is" => TokenKind::Is,
            "library" => TokenKind::Library,
            "limited" => TokenKind::Limited,
            "others" => TokenKind::Others,
            "package" => TokenKind::Package,
            "pragma" => TokenKind::Pragma,
            "private" => TokenKind::Private,
            "procedure" => TokenKind::Procedure,
            "project" => TokenKind::Project,
            "renames" => TokenKind::Renames,
            "separate" => TokenKind::Separate,
            "type" => TokenKind::Type,
            "null" => TokenKind::Null,
            "use" => TokenKind::Use,
            "with" => TokenKind::With,
            "when" => TokenKind::When,
            _ => {
                // We can't just do ASCII lower-case, but instead need to do
                // full conversion to lower case here.
                TokenKind::Identifier(Ustr::from(&n.to_lowercase()))
            }
        }
    }

    fn scan_token(&mut self) -> Token {
        self.skip_non_tokens();
        let start_line = self.line;
        let kind = match self.current {
            '\x00' => return Token {
                kind: TokenKind::EndOfFile,
                line: start_line,
            },
            '(' => TokenKind::OpenParenthesis,
            ')' => TokenKind::CloseParenthesis,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '|' => TokenKind::Pipe,
            '&' => TokenKind::Ampersand,
            '\'' => {
                //  Either a character or a simple tick
                let saved_offset = self.offset;
                let c = {
                    self.scan_char();
                    self.current
                };
                if let Some('\'') = self.peek_char() {
                    TokenKind::Character(c)
                } else {
                    self.offset = saved_offset;
                    TokenKind::Tick
                }
            },
            '-' => TokenKind::Minus, // comments handled in skip_non_tokens
            '"' => return Token {
                kind: self.scan_quote(),
                line: start_line,
            },
            ':' => return Token {
                kind: self.scan_colon(),
                line: start_line
            },
            '=' => return Token {
                kind: self.scan_equal(),
                line: start_line,
            },
            c if is_wordchar(c) => return Token {
                kind: self.scan_identifier(),
                line: start_line,
            },
            c => TokenKind::InvalidChar(c),
        };

        self.scan_char();
        Token {
            kind,
            line: start_line,
        }
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

impl<'a> Iterator for AdaLexer<'a> {
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
