use crate::ada_lexer::AdaLexer;
use crate::base_lexer::BaseScanner;
use crate::errors::Error;
use crate::tokens::TokenKind;
use crate::units::{QualifiedName, Unit};

pub struct AdaScanner<'a> {
    base: BaseScanner<AdaLexer<'a>>,
}

impl<'a> AdaScanner<'a> {
    /// Parse an Ada source file, and return the unit name
    pub fn parse(lex: AdaLexer<'a>) -> Result<Unit, Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
        };
        let mut unit = Unit::default();

        loop {
            let n = scan.base.safe_next()?;
            match n.kind {
                TokenKind::Use
                | TokenKind::With => {
                    scan.parse_with_or_use_clause(n.kind, &mut unit.deps)
                }
                TokenKind::Pragma => scan.parse_pragma(),
                TokenKind::Limited    // limited with 
                | TokenKind::Private => Ok(()),
                TokenKind::Separate => {
                    match scan.parse_separate() {
                        Ok(sep) => {
                            unit.name = sep;
                            Ok(())
                        }
                        Err(e) => Err(e)
                    }
                }
                TokenKind::Generic => scan.parse_generic(),
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function => {
                    if scan.base.peek() == TokenKind::Body {
                        scan.base.safe_next()?;
                    }

                    match scan.base.expect_qname(TokenKind::Dot) {
                        Ok(n) => {
                            unit.name.join(n);
                            break;
                        }
                        Err(e) => Err(e)
                    }
                }
                t => Err(Error::wrong_token(
                    "with|generic|package|pragma|private|procedure|function|use|separate",
                    t))
            }.map_err(|e| scan.base.error_with_location(e))?;
        }
        Ok(unit)
    }

    fn parse_with_or_use_clause(
        &mut self,
        kind: TokenKind,
        deps: &mut Vec<QualifiedName>,
    ) -> Result<(), Error> {
        if kind == TokenKind::Use && TokenKind::Type == self.base.peek() {
            self.base.next_token(); // consume "use type"
        }
        loop {
            let d = self.base.expect_qname(TokenKind::Dot)?;
            if kind == TokenKind::With {
                deps.push(d);
            }
            let n = self.base.safe_next()?;
            match n.kind {
                TokenKind::Semicolon => break,
                TokenKind::Comma => {}
                t => Err(Error::wrong_token(",|;", t))?,
            }
        }
        Ok(())
    }

    fn parse_separate(&mut self) -> Result<QualifiedName, Error> {
        self.base.expect(TokenKind::OpenParenthesis)?;
        let sep = self.base.expect_qname(TokenKind::Dot)?;
        self.base.expect(TokenKind::CloseParenthesis)?;
        Ok(sep)
    }

    fn parse_pragma(&mut self) -> Result<(), Error> {
        let _ = self.base.expect_identifier()?; // name of pragma
        self.base.skip_opt_arg_list()?; // optional parameters
        Ok(())
    }

    fn parse_generic(&mut self) -> Result<(), Error> {
        let mut in_parens = 0;
        loop {
            match self.base.peek() {
                TokenKind::OpenParenthesis => {
                    self.base.safe_next()?; //  consume '('
                    in_parens += 1;
                }
                TokenKind::CloseParenthesis => {
                    self.base.safe_next()?; //  consume ')'
                    in_parens -= 1;
                }
                TokenKind::Package
                | TokenKind::Procedure
                | TokenKind::Function
                    if in_parens == 0 =>
                {
                    break
                }
                TokenKind::With => {
                    self.base.safe_next()?; //  skip 'with'
                                            //  Skip next word, which could be package|procedure|
                                            //  function|private or any aspect
                    self.base.safe_next()?;
                }
                _ => {
                    self.base.safe_next()?;
                }
            }
        }
        Ok(())
    }
}
