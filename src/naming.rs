use crate::directory::Directory;
use crate::sourcefile::SourceFile;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ustr::Ustr;

lazy_static::lazy_static! {
    static ref CST_ADA: Ustr = Ustr::from("ada");
}

/// The naming scheme, for one scenario.  This groups all required attributes
/// used to find source files.
#[derive(Clone, Default, Debug)]
pub struct Naming {
    languages: Vec<Ustr>, // list of languages for this project
    source_dirs: Vec<PathBuf>, // source_dirs in this scenario
    source_files: Option<HashSet<Ustr>>, // source_files in this scenario
    pub spec_suffix: HashMap<Ustr, Ustr>, // lang->spec suffix
    pub body_suffix: HashMap<Ustr, Ustr>, // lang->body suffix
    pub spec_files: HashMap<Ustr, Ustr>, // unit name -> spec file name
    pub body_files: HashMap<Ustr, Ustr>, // unit name -> body file name
}

impl Naming {
    pub fn new_with_dirs(source_dirs: Vec<PathBuf>) -> Self {
        Naming {
            source_dirs,
            ..Default::default()
        }
    }

    pub fn set_source_files(&mut self, files: HashSet<Ustr>) {
        self.source_files = Some(files);
    }

    pub fn set_languages(&mut self, langs: Vec<Ustr>) {
        self.languages = langs;
    }

    fn register_source(
        &self,
        lang: Ustr,
        basename: &Ustr,
        path: &Path,
    ) -> Option<SourceFile> {
        let valid = match &self.source_files {
            None => true,
            Some(sf) => sf.contains(basename),
        };
        if valid {
            Some(SourceFile::new(path, lang))
        } else {
            None
        }
    }

    /// Find all source files for this naming scheme.  `all_dirs` is used as a
    /// cache and includes all source directories in any of the loaded projects.
    pub fn find_source_files(
        &self,
        all_dirs: &HashSet<Directory>,
    ) -> Vec<SourceFile> {
        let mut files = Vec::new();

        for d in &self.source_dirs {
            if let Some(dir) = all_dirs.get(d) {
                for lang in &self.languages {
                    files.extend(
                        dir.find_suffix(&self.spec_suffix[lang]).filter_map(
                            |(b, p)| self.register_source(*lang, b, p),
                        ),
                    );
                    files.extend(
                        dir.find_suffix(&self.body_suffix[lang]).filter_map(
                            |(b, p)| self.register_source(*lang, b, p),
                        ),
                    );
                }

                if self.languages.contains(&CST_ADA) {
                    files.extend(
                        dir.add_basenames(self.spec_files.values()).filter_map(
                            |(b, p)| self.register_source(*CST_ADA, b, p),
                        ),
                    );
                    files.extend(
                        dir.add_basenames(self.body_files.values()).filter_map(
                            |(b, p)| self.register_source(*CST_ADA, b, p),
                        ),
                    );
                }
            }
        }
        files
    }
}
