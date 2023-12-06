use crate::errors::Error;
use crate::files::File;
use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::tokens::TokenKind;

pub struct AdaScanner<'a> {
    lex: AdaLexer<'a>,
}

impl<'a> AdaScanner<'a> {

    pub fn new (file: &'a mut File) -> Self {
        Self {
            lex: AdaLexer::new(file, AdaLexerOptions {
                aggregate_is_keyword: false,
            }),
        }
    }

    pub fn parse(mut self) -> Result<(), Error> {
        self.parse_file().map_err(|e| self.lex.error_with_location(e))?;
        Ok(())
    }

    fn parse_file(&mut self) -> Result<(), Error> {
        loop {
            match self.lex.peek() {
                TokenKind::Use
                | TokenKind::With => self.parse_with_or_use_clause()?,
                TokenKind::Pragma => self.parse_pragma()?,
                TokenKind::Limited    // limited with 
                | TokenKind::Private => {
                    self.lex.next();  // consume keyword
                }
                TokenKind::Separate => self.parse_separate()?,
                TokenKind::Generic => self.parse_generic()?,
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function => break,
                t => Err(Error::wrong_token(
                    "with|generic|package|pragma|private|procedure|function|use|separate",
                    t))?
            }
        }
        Ok(())
    }

    fn parse_with_or_use_clause(&mut self) -> Result<(), Error> {
        let _ = self.lex.next();  // consume with or use
        if TokenKind::Type == self.lex.peek() {   // use type ...;
            let _ = self.lex.next();  // consume "type"
        }
        loop {
            self.expect_qname()?;
            let n = self.lex.safe_next()?;
            match n.kind {
                TokenKind::Semicolon => break,
                TokenKind::Comma => {},
                t => Err(Error::wrong_token(",|;", t))?
            }
        }
        Ok(())
    }

    fn parse_separate(&mut self) -> Result<(), Error> {
        self.lex.expect(TokenKind::Separate)?;
        self.lex.expect(TokenKind::OpenParenthesis)?;
        self.expect_qname()?;
        self.lex.expect(TokenKind::CloseParenthesis)?;
        Ok(())
    }

    fn parse_pragma(&mut self) -> Result<(), Error> {
        self.lex.expect(TokenKind::Pragma)?;
        let _ = self.lex.expect_identifier()?;  // name of pragma
        self.parse_opt_arg_list()?;     // optional parameters
        Ok(())
    }

    fn parse_generic(&mut self) -> Result<(), Error> {
        self.lex.expect(TokenKind::Generic)?;
        loop {
            match self.lex.peek() {
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function => break,
                TokenKind::With => {
                    let _ = self.lex.next();  // consume 'with'
                    let n = self.lex.safe_next()?;
                    match n.kind {
                        TokenKind::Private   // type .. is private;
                        | TokenKind::Package
                        | TokenKind::Procedure
                        | TokenKind::Function => {},
                        t => Err(Error::wrong_token("package|procedure|function", t))?,
                    }
                }
                _ => {
                    let _ = self.lex.next();  // skip
                }
            }
        }
        Ok(())
    }

    fn parse_opt_arg_list(&mut self) -> Result<(), Error> {
        if self.lex.expect(TokenKind::OpenParenthesis).is_ok() {
            loop {
                let n = self.lex.safe_next()?;
                if n.kind == TokenKind::CloseParenthesis {
                    break;
                }
            }
            self.lex.expect(TokenKind::Semicolon)?;
        }

        Ok(())
    }

    fn expect_qname(&mut self) -> Result<(), Error> {
        loop {
            self.lex.expect_identifier()?;
            if TokenKind::Dot != self.lex.peek() {
                break;
            }

            let _ = self.lex.next();  // consume the dot
        }
        Ok(())
    }

}
