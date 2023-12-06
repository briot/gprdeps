use std::path::{Path, PathBuf};
use ustr::Ustr;

#[derive(Default)]
pub struct SourceFile {
    path: PathBuf,
    _lang: Ustr, // Lower-case
    _unit: Ustr, // For Ada, the package name; for C, unused

    _deps: Vec<Ustr>,
    // dependencies (on package names, or paths, .. depending on the language.
    // These are unresolved for now, so just as found in the source code.
}

impl SourceFile {
    pub fn new(path: &Path) -> Self {
        SourceFile {
            path: path.to_owned(),
            ..Default::default()
        }
    }

    pub fn parse(&mut self) {

    }
}
