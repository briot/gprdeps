use crate::errors::Error;
use crate::files::File;
use crate::tokens::{Token, TokenKind};
use std::path::PathBuf;
use ustr::Ustr;

#[derive(Copy, Clone)]
pub struct Context {
    // The next character to process, the source line it is at, and the
    // offset at which we read it.
    offset: usize,
    line: u32,
    current: char,
}

impl Context {
    pub fn build_token(&self, kind: TokenKind) -> Token {
        Token {
            kind,
            line: self.line,
        }
    }
}

/// This type includes all base services shared by lexers.
pub(crate) struct BaseLexer<'a> {
    path: PathBuf,
    input: &'a mut str,
    context: Context,
}

impl<'a> BaseLexer<'a> {
    /// Builds a new lexer
    pub fn new(file: &'a mut File) -> Self {
        let path = file.path().to_owned();
        let f = file.as_mut_str();
        Self {
            path,
            context: Context {
                current: f.chars().next().unwrap(),
                line: 1,
                offset: 0,
            },
            input: f,
        }
    }

    /// Save and restore the position in the stream.  Useful when we need to
    /// backtrack.
    pub fn save_context(&self) -> Context {
        self.context
    }
    pub fn restore_context(&mut self, ctx: Context) {
        self.context = ctx;
    }

    /// Whether the current character is valid for an identifier
    pub fn is_wordchar(&self) -> bool {
        matches!(
            self.context.current,
            '0' ..= '9' | 'A' ..= 'Z' | 'a' ..= 'z' | '_'
        )
    }

    /// Wraps an error with location information, so that we can report
    /// which file+line the error occurred at.
    pub fn error_with_location(&self, error: Error) -> Error {
        Error::WithLocation {
            path: self.path.clone(),
            line: self.context.line,
            error: Box::new(error),
        }
    }

    /// Consumes one character.  This character is both returned and made
    /// available in self.current.
    /// At end of file, it returns \x00
    #[inline]
    pub fn scan_char(&mut self) -> char {
        self.context.offset += self.context.current.len_utf8();
        match self.input[self.context.offset..].chars().next() {
            None => self.context.current = '\x00',
            Some('\n') => {
                self.context.line += 1;
                self.context.current = '\n';
            }
            Some(c) => self.context.current = c,
        };
        self.context.current
    }

    /// Peek at the following character, and return it.
    #[inline]
    pub fn peek_char(&mut self) -> Option<char> {
        self.input[self.context.offset + self.context.current.len_utf8()..]
            .chars()
            .next()
    }

    /// On input, self.current is the leading quote
    pub fn scan_quote(&mut self) -> TokenKind {
        self.scan_char(); // consume leading quote
        let start_offset = self.context.offset;
        loop {
            match self.context.current {
                '\x00' => return TokenKind::EndOfFile, //  Unterminated str
                '"' => {
                    let end_offset = self.context.offset;
                    self.scan_char();
                    let s = Ustr::from(&self.input[start_offset..end_offset]);
                    return TokenKind::String(s);
                }
                _ => {}
            }
            self.scan_char();
        }
    }

    /// Skip all characters until end of line (which is not consumed)
    pub fn skip_to_eol(&mut self) {
        loop {
            match self.scan_char() {
                '\n' | '\x00' => break,
                _ => {}
            }
        }
    }

    /// Skip until the given character, and return the matching substring not
    /// including the final marker.  The final marker has been skipped and will
    /// not be seen again.
    /// The current character is also not included in the substring.
    pub fn skip_to_char(&mut self, marker: char) -> &mut str {
        let mut c = self.scan_char(); //  skip current character
        let start_offset = self.context.offset;
        loop {
            if c == marker || c == '\x00' {
                break;
            }
            c = self.scan_char();
        }
        let end_offset = self.context.offset;
        self.scan_char(); //  consume end-of-line
        &mut self.input[start_offset..end_offset]
    }

    /// Get the next identifier (which might end up being a keyword, but that
    /// will be tested by each language-specific lexer).
    /// The returned str can be modified in place to lower ASCII letters for
    /// instance, in the case of case-insensitive languages.
    pub fn scan_identifier(&mut self) -> &mut str {
        let start_offset = self.context.offset;
        loop {
            self.scan_char();
            if !self.is_wordchar() {
                break;
            }
        }
        &mut self.input[start_offset..self.context.offset]
    }
}

pub(crate) trait Lexer {
    /// Scan the next token.  The last character read, which hasn't been
    /// processed yet, is `current`.
    fn scan_token(&mut self, current: char) -> TokenKind;

    /// Decorate an error to indicate precisely where the error occurred.
    fn error_with_location(&self, error: Error) -> Error;

    /// Build a token, with proper location
    fn save_context(&self) -> Context;
}

pub(crate) struct BaseScanner<LEXER: Lexer> {
    pub(crate) lex: LEXER,

    //  One symbol ahead (??? could let users use Peekable)
    pub(crate) peeked: Token,
}

impl<LEXER: Lexer> BaseScanner<LEXER> {
    pub fn new(lex: LEXER) -> Self {
        let mut s = Self {
            lex,
            peeked: Token::new(TokenKind::EndOfFile, 0),
        };
        let _ = s.next_token(); // always returns None, but sets s.peeked()
        s
    }

    /// Peek at the next token, without consuming it
    pub fn peek(&self) -> TokenKind {
        self.peeked.kind.clone()
    }

    /// Consume the next token in the stream
    pub fn next_token(&mut self) -> Option<Token> {
        let ctx = self.lex.save_context();
        let mut p = ctx.build_token(self.lex.scan_token(ctx.current));
        std::mem::swap(&mut self.peeked, &mut p);
        if p.kind == TokenKind::EndOfFile {
            None
        } else {
            Some(p)
        }
    }
    /// Get the next token, failing with error on end of file
    pub fn safe_next(&mut self) -> Result<Token, Error> {
        self.next_token().ok_or(Error::UnexpectedEOF)
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

    /// Expects a qualified name ("parent1.parent2.name")
    pub fn expect_qname(&mut self) -> Result<(), Error> {
        loop {
            self.expect_identifier()?;
            if TokenKind::Dot != self.peek() {
                break;
            }

            let _ = self.next_token(); // consume the dot
        }
        Ok(())
    }
}
