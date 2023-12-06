use crate::errors::Error;
use crate::files::File;
use crate::cpp_lexer::CppLexer;
use crate::tokens::TokenKind;

pub struct CppScanner<'a> {
    lex: CppLexer<'a>,
}

impl<'a> CppScanner<'a> {

    pub fn new (file: &'a mut File) -> Self {
        Self {
            lex: CppLexer::new(file),
        }
    }

    pub fn parse(mut self) -> Result<(), Error> {
        self.parse_file().map_err(|e| self.lex.error_with_location(e))?;
        Ok(())
    }

    fn parse_file(&mut self) -> Result<(), Error> {
        loop {
            match self.lex.peek() {
                TokenKind::EndOfFile => break,
                TokenKind::HashElse => {
                    let _ = self.lex.next();  // consume keyword
                }
                TokenKind::HashIf => {
                    self.lex.skip_to_eol();
                    break;
                }
                TokenKind::HashInclude => {
                    let _ = self.lex.next();  // consume keyword
                    let _ = self.lex.expect_str_or_syspath()?;
                    break;
                },
                TokenKind::HashIfndef
                | TokenKind::HashIfdef
                | TokenKind::HashDefine
                | TokenKind::HashUndef
                | TokenKind::Pragma => {
                    let _ = self.lex.next();  // consume keyword
                    let _ = self.lex.expect_identifier()?;
                    break;
                }
                TokenKind::HashEndif => {
                    let _ = self.lex.next();  // consume keyword
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
                    t))?
            }
        }
        Ok(())
    }

//    fn parse_opt_arg_list(&mut self) -> Result<(), Error> {
//        if self.lex.expect(TokenKind::OpenParenthesis).is_ok() {
//            loop {
//                let n = self.lex.safe_next()?;
//                if n.kind == TokenKind::CloseParenthesis {
//                    break;
//                }
//            }
//            self.lex.expect(TokenKind::Semicolon)?;
//        }
//
//        Ok(())
//    }

}
