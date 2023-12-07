use crate::base_lexer::{BaseScanner, Lexer};
use crate::cpp_lexer::CppLexer;
use crate::errors::Error;
use crate::tokens::TokenKind;
use ustr::Ustr;

pub struct CppScanner<'a> {
    base: BaseScanner<CppLexer<'a>>,
}

impl<'a> CppScanner<'a> {
    pub fn parse(lex: CppLexer<'a>) -> Result<(), Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
        };

        loop {
            match scan.base.peek() {
                TokenKind::EndOfFile => break,
                TokenKind::HashElse => {
                    let _ = scan.base.next_token(); // consume keyword
                    Ok(())
                }
                TokenKind::HashIf => break, //  already consumed end-of-line
                TokenKind::HashInclude => {
                    let _ = scan.base.next_token(); // consume keyword
                    let _ = scan.expect_str_or_syspath();
                    break;
                }
                TokenKind::HashIfndef
                | TokenKind::HashIfdef
                | TokenKind::HashDefine
                | TokenKind::HashUndef
                | TokenKind::Pragma => {
                    let _ = scan.base.next_token(); // consume keyword
                    let _ = scan.base.expect_identifier();
                    break;
                }
                TokenKind::HashEndif => {
                    let _ = scan.base.next_token(); // consume keyword
                    break;
                }
                TokenKind::Identifier(_) => {
                    // Stop parsing at the first function definition.  The
                    // single identifier is likely a type.
                    // ??? This is incorrect, there might be further includes
                    // later
                    break;
                }
                t => Err(Error::wrong_token(
                    "#include|#ifndef|#ifdef|#endif|#pragma",
                    t,
                ))?,
            }
            .map_err(|e| scan.base.lex.error_with_location(e))?;
        }
        Ok(())
    }

    /// Consumes the next token from the lexer, and expects it to be a string,
    /// or a system path "<path.h>" which is returned.
    fn expect_str_or_syspath(&mut self) -> Result<Ustr, Error> {
        match self.base.peek() {
            TokenKind::String(s) => {
                let _ = self.base.safe_next()?; //  consume the string
                Ok(s)
            }
            TokenKind::LessThan => {
                let n = self.base.lex.skip_to_char('>');
                let res = Ustr::from(n);
                Ok(res)
            }
            t => Err(Error::wrong_token("string", t)),
        }
    }
}
