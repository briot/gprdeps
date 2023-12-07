use crate::ada_lexer::AdaLexer;
use crate::base_lexer::{BaseScanner, Lexer};
use crate::errors::Error;
use crate::tokens::TokenKind;

pub struct AdaScanner<'a> {
    base: BaseScanner<'a, AdaLexer<'a>>,
}

impl<'a> AdaScanner<'a> {
    pub fn parse(lex: &'a mut AdaLexer<'a>) -> Result<(), Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
        };

        loop {
            match scan.base.peek() {
                TokenKind::Use
                | TokenKind::With => scan.parse_with_or_use_clause(),
                TokenKind::Pragma => scan.parse_pragma(),
                TokenKind::Limited    // limited with 
                | TokenKind::Private => {
                    scan.base.next_token();  // consume keyword
                    Ok(())
                }
                TokenKind::Separate => scan.parse_separate(),
                TokenKind::Generic => scan.parse_generic(),
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function => break,
                t => Err(Error::wrong_token(
                    "with|generic|package|pragma|private|procedure|function|use|separate",
                    t))
            }.map_err(|e| scan.base.lex.error_with_location(e))?;
        }
        Ok(())
    }

    fn parse_with_or_use_clause(&mut self) -> Result<(), Error> {
        let _ = self.base.next_token(); // consume with or use
        if TokenKind::Type == self.base.peek() {
            // use type ...;
            let _ = self.base.next_token(); // consume "type"
        }
        loop {
            self.base.expect_qname()?;
            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::Semicolon => break,
                TokenKind::Comma => {}
                t => Err(Error::wrong_token(",|;", t))?,
            }
        }
        Ok(())
    }

    fn parse_separate(&mut self) -> Result<(), Error> {
        self.base.expect(TokenKind::Separate)?;
        self.base.expect(TokenKind::OpenParenthesis)?;
        self.base.expect_qname()?;
        self.base.expect(TokenKind::CloseParenthesis)?;
        Ok(())
    }

    fn parse_pragma(&mut self) -> Result<(), Error> {
        self.base.expect(TokenKind::Pragma)?;
        let _ = self.base.expect_identifier()?; // name of pragma
        self.parse_opt_arg_list()?; // optional parameters
        Ok(())
    }

    fn parse_generic(&mut self) -> Result<(), Error> {
        self.base.expect(TokenKind::Generic)?;
        loop {
            match self.base.peek() {
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function => break,
                TokenKind::With => {
                    let _ = self.base.next_token(); // consume 'with'
                    let n = self.base.safe_next()?;
                    match n.kind {
                        TokenKind::Private   // type .. is private;
                        | TokenKind::Package
                        | TokenKind::Procedure
                        | TokenKind::Function => {},
                        t => Err(Error::wrong_token("package|procedure|function", t))?,
                    }
                }
                _ => {
                    let _ = self.base.next_token(); // skip
                }
            }
        }
        Ok(())
    }

    fn parse_opt_arg_list(&mut self) -> Result<(), Error> {
        if self.base.expect(TokenKind::OpenParenthesis).is_ok() {
            loop {
                let n = self.base.safe_next()?;
                if n.kind == TokenKind::CloseParenthesis {
                    break;
                }
            }
            self.base.expect(TokenKind::Semicolon)?;
        }

        Ok(())
    }
}
