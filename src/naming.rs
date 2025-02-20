use crate::{
    directory::Directory, environment::Environment, errors::Error,
    qnames::QName, sourcefile::SourceFile,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
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

    pub main: Option<HashSet<Ustr>>, // base names of main files
    pub library_interfaces: Option<HashSet<QName>>, //  Unit names
    pub dot_replacement: Ustr,       // for Ada
}

/// Information for a source file in a project.  The `file` part is shared
/// amongst all projects, but whether the file is a main depends on the
/// project and the scenario.
/// This struct is for a single scenario.
#[derive(Debug)]
pub struct FileInGPR {
    pub file: Rc<RefCell<SourceFile>>,
    pub _is_main: bool,
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
        environ: &mut Environment,
        lang: Ustr,
        basename: &Ustr,
        path: &Path,
    ) -> Result<Option<FileInGPR>, Error> {
        let valid = match &self.source_files {
            None => true,
            Some(sf) => sf.contains(basename),
        };
        if valid {
            let is_main = match &self.main {
                None => false,
                Some(m) => m.contains(basename),
            };
            let s = environ.register_source(path, lang)?;
            if is_main {
                s.borrow_mut().is_ever_main = is_main;
            }
            Ok(Some(FileInGPR {
                file: s.clone(),
                _is_main: is_main,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find all source files for this naming scheme.  `all_dirs` is used as a
    /// cache and includes all source directories in any of the loaded projects.
    pub fn find_source_files(
        &self,
        env: &mut Environment,
        all_dirs: &HashSet<Directory>,
    ) -> Result<Vec<FileInGPR>, Error> {
        let mut files = Vec::new();

        for d in &self.source_dirs {
            if let Some(dir) = all_dirs.get(d) {
                for lang in &self.languages {
                    for (b, p) in dir.find_suffix(&self.spec_suffix[lang]) {
                        let s = self.register_source(env, *lang, b, p)?;
                        if let Some(s) = s {
                            files.push(s);
                        }
                    }
                    for (b, p) in dir.find_suffix(&self.body_suffix[lang]) {
                        let s = self.register_source(env, *lang, b, p)?;
                        if let Some(s) = s {
                            files.push(s);
                        }
                    }
                }

                if self.languages.contains(&CST_ADA) {
                    // ??? Use dot_replacement to resolve unit names

                    for (b, p) in dir.add_basenames(self.spec_files.values()) {
                        let s = self.register_source(env, *CST_ADA, b, p)?;
                        if let Some(s) = s {
                            files.push(s);
                        }
                    }
                    for (b, p) in dir.add_basenames(self.body_files.values()) {
                        let s = self.register_source(env, *CST_ADA, b, p)?;
                        if let Some(s) = s {
                            files.push(s);
                        }
                    }
                }
            }
        }
        Ok(files)
    }
}
