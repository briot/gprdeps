use crate::errors::Error;
use crate::files::File;
use crate::gpr_scanner::GprScanner;
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use ustr::Ustr;

#[derive(Default)]
pub struct SourceFile {
    path: PathBuf,
    lang: Ustr, // Lower-case
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

    pub fn parse(
        &mut self,
        settings: &Settings,
    ) -> Result<(), Error> {
        let mut file = File::new(&self.path)?;
        let scan = GprScanner::new(&mut file, settings);
        // let raw = scan.parse()?;

        Ok(())
    }
}
