use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::ada_scanner::AdaScanner;
use crate::cpp_lexer::CppLexer;
use crate::cpp_scanner::CppScanner;
use crate::errors::Error;
use crate::files::File;
use crate::units::SourceInfo;
use std::path::{Path, PathBuf};
use ustr::Ustr;

#[derive(Default)]
pub struct SourceFile {
    path: PathBuf,
    lang: Ustr, // Lower-case
}

impl SourceFile {
    pub fn new(path: &Path, lang: Ustr) -> Self {
        SourceFile {
            path: path.to_owned(),
            lang,
        }
    }

    /// Parse the source file to extract the unit name and the dependencies.
    /// It should return an empty unit name if the file should be ignored (for instance in Ada
    /// there is a `pragma no_body`, or in C there are preprocessor directives that make the file
    /// empty for the compiler).
    pub fn parse(&mut self) -> Result<SourceInfo, Error> {
        let mut file = File::new(&self.path)?;
        match self.lang.as_str() {
            "ada" => AdaScanner::parse(AdaLexer::new(
                &mut file,
                AdaLexerOptions {
                    kw_aggregate: false,
                    kw_body: true,
                },
            )),
            "c" | "c++" => {
                CppScanner::parse(CppLexer::new(&mut file), &self.path)
            }
            lang => Err(Error::CannotParse {
                path: self.path.clone(),
                lang: lang.into(),
            }),
        }
    }
}
