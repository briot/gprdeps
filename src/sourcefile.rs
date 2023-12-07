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
    lang: Ustr,  // Lower-case
    _unit: Ustr, // For Ada, the package name; for C, unused

    _deps: Vec<Ustr>,
    // dependencies (on package names, or paths, .. depending on the language.
    // These are unresolved for now, so just as found in the source code.
}

impl SourceFile {
    pub fn new(path: &Path, lang: Ustr) -> Self {
        SourceFile {
            path: path.to_owned(),
            lang,
            ..Default::default()
        }
    }

    pub fn parse(&mut self) -> Result<(), Error> {
        let mut file = File::new(&self.path)?;

        match self.lang.as_str() {
            "ada" => {
                let options = AdaLexerOptions {
                    aggregate_is_keyword: false,
                };
                let mut lex = AdaLexer::new(&mut file, options);
                AdaScanner::parse(&mut lex)?;
            }
            "c" | "c++" => {
                let mut lex = CppLexer::new(&mut file);
                CppScanner::parse(&mut lex)?;
            }
            lang => {
                println!("Cannot parse {} file {}", lang, self.path.display());
            }
        }

        Ok(())
    }
}
