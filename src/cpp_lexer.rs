use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};
use std::path::PathBuf;
use ustr::Ustr;

fn is_wordchar(c: char) -> bool {
    matches!(c, '0' ..= '9' | 'A' ..= 'Z' | 'a' ..= 'z' | '_')
}

pub struct CppLexer<'a> {
    path: PathBuf,
    input: &'a mut str,

    // The next character to process, the source line it is at, and the
    // offset at which we read it.
    line: u32,
    offset: usize,
    current: char,

    //  One symbol ahead (??? could let users use Peekable)
    peeked: Token,
}

impl<'a> CppLexer<'a> {
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
        self.input[self.offset + self.current.len_utf8()..].chars().next()
    }

    /// Peek at the next item, without consuming it
    pub fn peek(&self) -> TokenKind {
        self.peeked.kind.clone()
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

    /// Skip all characters until end of line
    pub fn skip_to_eol(&mut self) {
        loop {
            self.scan_char();
            if self.current == '\n' {
                self.scan_char();  // consume end-of-line
                break;
            }
        }
    }

    fn skip_non_tokens(&mut self) {
        #[derive(Debug)]
        enum InComment {
            NotComment,
            OneLine,
            MultiLine,
        }
        let mut in_comment = InComment::NotComment;
        loop {
            match (self.current, &in_comment) {
                ('\n', InComment::OneLine) => {
                    self.line += 1;
                    in_comment = InComment::NotComment;
                },
                ('\n', _) => self.line += 1,
                (' ' | '\t' | '\r', _) => {}
                ('/', InComment::NotComment) => {
                    match (self.peek_char(), &in_comment) {
                        (Some('*'), InComment::NotComment) => {
                            self.scan_char();  // consume '/'
                            self.scan_char();  // consume '*'
                            in_comment = InComment::MultiLine;
                        }
                        (Some('/'), InComment::NotComment) => {
                            self.scan_char();  // consume '/'
                            self.scan_char();  // consume '/'
                            in_comment = InComment::OneLine;
                        }
                        _ => {}
                    }
                }
                ('*', InComment::MultiLine) => {
                    if let Some('/') = self.peek_char() {
                        self.scan_char(); //  consume '/'
                        in_comment = InComment::NotComment;
                    }
                }
                (_, InComment::NotComment) => break,
                _ => {},
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
        match &*n {
            "loop" => TokenKind::Loop,
            _ => TokenKind::Identifier(Ustr::from(n)),
        }
    }

    fn scan_directive(&mut self) -> TokenKind {
        self.scan_char();        // consume '#'
        self.skip_non_tokens();  // There could be spaces
        let start_offset = self.offset;
        while is_wordchar(self.current) {
            self.scan_char();
        }
        match &self.input[start_offset..self.offset] {
            "define"  => TokenKind::HashDefine,
            "else"    => TokenKind::HashElse,
            "endif"   => TokenKind::HashEndif,
            "if"      => TokenKind::HashIf,
            "ifdef"   => TokenKind::HashIfdef,
            "ifndef"  => TokenKind::HashIfndef,
            "include" => TokenKind::HashInclude,
            "pragma"  => TokenKind::Pragma,
            "undef"   => TokenKind::HashUndef,
            _ => {
                self.offset = start_offset;
                TokenKind::Hash
            }
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
            '<' => TokenKind::LessThan,
            '>' => TokenKind::GreaterThan,
            '#' => return Token {
                kind: self.scan_directive(),
                line: start_line,
            },
            '-' => TokenKind::Minus, // comments handled in skip_non_tokens
            '"' => return Token {
                kind: self.scan_quote(),
                line: start_line,
            },
            c if is_wordchar(c) => return Token {
                kind: self.scan_identifier(),
                line: start_line,
            },
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

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or a system path "<path.h>" which is returned.
    pub fn expect_str_or_syspath(&mut self) -> Result<Ustr, Error> {
        let n = self.safe_next()?;
        match n.kind {
            TokenKind::String(s) => Ok(s),
            TokenKind::LessThan => {
                let start_offset = self.offset;
                loop {
                    let n = self.safe_next()?;
                    if n.kind == TokenKind::GreaterThan {
                        break;
                    }
                }
                let res = Ustr::from(&self.input[start_offset..self.offset]);
                self.next();  // consume '>'
                Ok(res)

            }
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

impl<'a> Iterator for CppLexer<'a> {
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
