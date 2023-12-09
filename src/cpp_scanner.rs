use crate::base_lexer::BaseScanner;
use crate::cpp_lexer::CppLexer;
use crate::errors::Error;
use crate::tokens::TokenKind;
use crate::units::{QualifiedName, Unit};
use std::path::Path;
use ustr::Ustr;

pub struct CppScanner<'a> {
    base: BaseScanner<CppLexer<'a>>,
}

impl<'a> CppScanner<'a> {
    pub fn parse(lex: CppLexer<'a>, path: &Path) -> Result<Unit, Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
        };
        let mut unit = Unit {
            name: QualifiedName::new(
                vec![Ustr::from(path.as_os_str().to_str().unwrap())]),
            ..Default::default()
        };

        loop {
            match scan.base.peek() {
                TokenKind::EndOfFile => break,
                TokenKind::HashInclude(path) => {
                    scan.base.next_token(); // consume keyword
                    unit.deps.push(QualifiedName::new(vec![path]));
                    Ok(())
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
                )),
            }
            .map_err(|e| scan.base.error_with_location(e))?;
        }
        Ok(unit)
    }
}
