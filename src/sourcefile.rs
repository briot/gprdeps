use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::ada_scanner::AdaScanner;
use crate::cpp_lexer::CppLexer;
use crate::cpp_scanner::CppScanner;
use crate::errors::Error;
use crate::files::File;
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

    pub fn parse(&mut self) -> Result<(), Error> {
        let mut file = File::new(&self.path)?;
        let _ = match self.lang.as_str() {
            "ada" => {
                AdaScanner::parse(AdaLexer::new(&mut file, AdaLexerOptions {
                    kw_aggregate: false,
                    kw_body: true,
                }))?
            }
            "c" | "c++" => {
                CppScanner::parse(
                    CppLexer::new(&mut file),
                    &self.path)?
            }
            lang => {
                println!("Cannot parse {} file {}", lang, self.path.display());
                return Ok(());
            }
        };
//        println!("{} {:?}", self.path.display(), unit);
        Ok(())
    }
}
