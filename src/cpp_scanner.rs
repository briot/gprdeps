use crate::{
    base_lexer::BaseScanner,
    cpp_lexer::CppLexer,
    errors::Error,
    qnames::QName,
    sourcefile::{ParseResult, SourceKind},
    tokens::TokenKind,
};
use std::path::Path;
use ustr::Ustr;

pub struct CppScanner<'a> {
    base: BaseScanner<CppLexer<'a>>,
}

impl<'a> CppScanner<'a> {
    pub fn parse(lex: CppLexer<'a>, path: &Path) -> Result<ParseResult, Error> {
        let mut scan = Self {
            base: BaseScanner::new(lex),
        };
        let mut info = ParseResult {
            unitname: QName::new(vec![Ustr::from(
                path.as_os_str().to_str().unwrap(),
            )]),
            kind: SourceKind::Implementation,
            deps: Default::default(),
        };

        loop {
            match scan.base.peek() {
                TokenKind::EndOfFile => break,
                TokenKind::HashInclude(path) => {
                    scan.base.next_token(); // consume keyword
                    info.deps.insert(QName::new(vec![path]));
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
        Ok(info)
    }
}
