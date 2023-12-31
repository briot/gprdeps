use crate::directory::Directory;
use crate::errors::Error;
use crate::graph::NodeIndex;
use crate::rawexpr::{
    PackageName, QualifiedName, SimpleName, Statement, StatementList,
    StringOrOthers, PACKAGE_NAME_VARIANTS,
};
use crate::rawgpr::RawGPR;
use crate::scenarios::{AllScenarios, Scenario, EMPTY_SCENARIO};
use crate::settings::Settings;
use crate::values::ExprValue;
use path_clean::PathClean;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ustr::{Ustr, UstrSet};
use walkdir::WalkDir;

lazy_static::lazy_static! {
    static ref CST_ADA: Ustr = Ustr::from("ada");
    static ref CST_C: Ustr = Ustr::from("c");
    static ref CST_CPP: Ustr = Ustr::from("c++");
    static ref CST_X86_64_LINUX: Ustr = Ustr::from("x86_64-linux");
    static ref CST_DOT: Ustr = Ustr::from(".");
    static ref CST_MINUS: Ustr = Ustr::from("-");
    static ref CST_EXT_ADS: Ustr = Ustr::from(".ads");
    static ref CST_EXT_ADB: Ustr = Ustr::from(".adb");
    static ref CST_EXT_H: Ustr = Ustr::from(".h");
    static ref CST_EXT_C: Ustr = Ustr::from(".c");
    static ref CST_EXT_HH: Ustr = Ustr::from(".hh");
    static ref CST_EXT_CPP: Ustr = Ustr::from(".cpp");
}

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
#[derive(Default)]
pub struct GprFile {
    pub index: NodeIndex,
    pub name: Ustr,
    path: PathBuf,
    values: [HashMap<
        SimpleName, // variable or attribute name
        ExprValue,  // value for each scenario
    >; PACKAGE_NAME_VARIANTS],

    source_files: HashMap<Scenario, Vec<(PathBuf, Ustr)>>, // path and lang
}

impl GprFile {
    pub fn new(path: &Path, index: NodeIndex, name: Ustr) -> Self {
        let mut s = Self {
            path: path.into(),
            name,
            index,
            ..Default::default()
        };

        // Declare the fallback value for "project'Target" attribute.
        s.values[PackageName::None as usize].insert(
            SimpleName::Target,
            ExprValue::new_with_str(*CST_X86_64_LINUX),
        );
        s.values[PackageName::Linker as usize]
            .insert(SimpleName::LinkerOptions, ExprValue::new_with_list(&[]));
        s.values[PackageName::None as usize].insert(
            SimpleName::SourceDirs,
            ExprValue::new_with_list(&[*CST_DOT]),
        );
        s.values[PackageName::None as usize].insert(
            SimpleName::ObjectDir,
            ExprValue::new_with_list(&[*CST_DOT]),
        );
        s.values[PackageName::None as usize]
            .insert(SimpleName::ExecDir, ExprValue::new_with_list(&[*CST_DOT]));
        s.values[PackageName::None as usize].insert(
            SimpleName::Languages,
            ExprValue::new_with_list(&[*CST_ADA]),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::DotReplacement,
            ExprValue::new_with_str(*CST_MINUS),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::SpecSuffix(*CST_ADA),
            ExprValue::new_with_str(*CST_EXT_ADS),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::BodySuffix(*CST_ADA),
            ExprValue::new_with_str(*CST_EXT_ADB),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::SpecSuffix(*CST_CPP),
            ExprValue::new_with_str(*CST_EXT_HH),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::BodySuffix(*CST_CPP),
            ExprValue::new_with_str(*CST_EXT_CPP),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::SpecSuffix(*CST_C),
            ExprValue::new_with_str(*CST_EXT_H),
        );
        s.values[PackageName::Naming as usize].insert(
            SimpleName::BodySuffix(*CST_C),
            ExprValue::new_with_str(*CST_EXT_C),
        );
        s
    }

    /// Trim attributes and variables that will no longer be used.
    /// This is optional, and just a way to reduce the number of combinations
    /// that we will need to look at for scenarios.
    pub fn trim(&mut self) {
        for pkg in 0..PACKAGE_NAME_VARIANTS {
            self.values[pkg].retain(|name, _| {
                matches!(
                    name,
                    SimpleName::BodySuffix(_)
                        | SimpleName::Body(_)
                        | SimpleName::ExcludedSourceFiles
                        | SimpleName::Languages
                        | SimpleName::Main
                        | SimpleName::ProjectFiles
                        | SimpleName::SourceDirs
                        | SimpleName::SourceFiles
                        | SimpleName::Spec(_)
                        | SimpleName::SpecSuffix(_)
                        | SimpleName::SourceListFile
                )
            });
        }
    }

    /// Find which scenarios are actually useful for this project
    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
        for pkg in 0..PACKAGE_NAME_VARIANTS {
            for v in self.values[pkg].values() {
                v.find_used_scenarios(useful);
            }
        }
    }

    /// Resolve relative paths, and cleanup ".." from the name.
    /// It optionally resolves symbolic links, in which case it might fail if
    /// the file doesn't exist on the disk.
    fn normalize_path(
        &self,
        path: &str,
        settings: &Settings,
    ) -> Option<PathBuf> {
        let p = self.path.parent().unwrap().join(path);
        if settings.resolve_symbolic_links {
            match p.canonicalize() {
                Ok(p) => Some(p),
                Err(e) => {
                    if settings.report_missing_source_dirs {
                        eprintln!("{}: {} {}", self, e, p.display());
                    }
                    None
                }
            }
        } else {
            Some(p.clean())
        }
    }

    // Retrieve the value of a string attribute
    fn str_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> &HashMap<Scenario, Ustr> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::Str(v)) => v,
            v => panic!("Wrong type for attribute {}{}, {:?}", pkg, name, v),
        }
    }

    // Retrieve the value of a string list attribute
    fn strlist_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> &HashMap<Scenario, Vec<Ustr>> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::StrList(v)) => v,
            v => panic!("Wrong type for attribute {}{}, {:?}", pkg, name, v),
        }
    }

    // Retrieve the value of a path list attribute
    fn pathlist_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> &HashMap<Scenario, Vec<PathBuf>> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::PathList(v)) => v,
            v => panic!("Wrong type for attribute {}{}, {:?}", pkg, name, v),
        }
    }

    //  Resolve source directories from the list of relative path names (as
    //  strings) read from the project file, into full paths.
    //  This is done for all scenarios.
    pub fn resolve_source_dirs(
        &mut self,
        dirs: &mut HashSet<Directory>,
        settings: &Settings,
    ) -> Result<(), Error> {
        let sourcedirs =
            self.strlist_attr(PackageName::None, &SimpleName::SourceDirs);

        let mut resolved_dirs = HashMap::new();

        for (scenar, dirs_in_scenario) in sourcedirs {
            let mut for_scenar = Vec::new();
            for d in dirs_in_scenario {
                if d.ends_with("/**") {
                    let parent =
                        d.chars().take(d.len() - 3).collect::<String>();

                    if let Some(s) = self.normalize_path(&parent, settings) {
                        for entry in WalkDir::new(s)
                            .follow_links(true)
                            .into_iter()
                            .filter_map(Result::ok)
                            .filter(|e| e.file_type().is_dir())
                        {
                            for_scenar.push(entry.into_path());
                        }
                    }
                } else if let Some(s) = self.normalize_path(d, settings) {
                    for_scenar.push(s);
                }
            }

            for d in &for_scenar {
                if !dirs.contains(d) {
                    dirs.insert(Directory::new(d.clone()));
                }
            }
            resolved_dirs.insert(*scenar, for_scenar);
        }

        self.values[PackageName::None as usize]
            .insert(SimpleName::SourceDirs, ExprValue::PathList(resolved_dirs));

        Ok(())
    }

    /// Given a directory, find all source files matching the naming scheme,
    /// and add them to `files`.  The naming scheme is for one specific
    /// scenario.
    fn check_file_candidates(
        scenarios: &mut AllScenarios,
        scenario: Scenario,
        dirs_in_scenario: &Vec<PathBuf>,
        lang: Ustr,
        suffixes: &HashMap<Scenario, Ustr>,
        all_dirs: &HashSet<Directory>,
        files: &mut HashMap<Scenario, Vec<(PathBuf, Ustr)>>,
    ) {
        for (scenar_spec, suffix) in suffixes {
            let s = scenarios.intersection(scenario, *scenar_spec);
            if s != EMPTY_SCENARIO {
                let sfiles = files.entry(s).or_default();
                for d in dirs_in_scenario {
                    if let Some(dir) = all_dirs.get(d) {
                        dir.filter_suffix(suffix, lang, sfiles);
                    }
                }
            }
        }
    }

    /// Return the list of source files for all scenarios
    pub fn resolve_source_files(
        &mut self,
        all_dirs: &HashSet<Directory>,
        scenarios: &mut AllScenarios,
    ) {
        let source_dirs =
            self.pathlist_attr(PackageName::None, &SimpleName::SourceDirs);
        let languages =
            self.strlist_attr(PackageName::None, &SimpleName::Languages);

        let mut files: HashMap<Scenario, Vec<(PathBuf, Ustr)>> = HashMap::new();

        for (scenar_dir, dirs_in_scenar) in source_dirs {
            for (scenar_lang, langs_in_scenar) in languages {
                let s = scenarios.intersection(*scenar_dir, *scenar_lang);
                if s == EMPTY_SCENARIO {
                    continue;
                }

                for lang in langs_in_scenar {
                    GprFile::check_file_candidates(
                        scenarios,
                        s,
                        dirs_in_scenar,
                        *lang,
                        self.str_attr(
                            PackageName::Naming,
                            &SimpleName::SpecSuffix(*lang),
                        ),
                        all_dirs,
                        &mut files,
                    );
                    GprFile::check_file_candidates(
                        scenarios,
                        s,
                        dirs_in_scenar,
                        *lang,
                        self.str_attr(
                            PackageName::Naming,
                            &SimpleName::BodySuffix(*lang),
                        ),
                        all_dirs,
                        &mut files,
                    );
                }
            }
        }

        self.source_files = files;
    }

    /// Add the list of source files
    pub fn get_source_files(&self, all_files: &mut HashSet<(PathBuf, Ustr)>) {
        all_files.extend(self.source_files.values().flatten().cloned());
    }

    /// Declare a new named object.
    /// It is an error if such an object already exists.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: SimpleName,
        value: ExprValue,
    ) -> Result<(), Error> {
        self.values[package as usize].insert(name, value);
        Ok(())
    }

    /// Lookup the project file referenced by the given name, in self or its
    /// dependencies.
    fn lookup_gpr<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GprFile],
    ) -> Result<&'a GprFile, Error> {
        match &name.project {
            None => Ok(self),
            Some(c) if *c == self.name => Ok(self),
            Some(n) => dependencies
                .iter()
                .copied()
                .find(|gpr| gpr.name == *n)
                .ok_or_else(|| Error::not_found(name)),
        }
    }

    /// After a project has been processed, we can lookup values of variables
    /// and attributes directly, for each scenario.
    /// The lookup is also done in imported projects.
    pub fn lookup<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GprFile],
        current_pkg: PackageName,
    ) -> Result<&'a ExprValue, Error> {
        let project = self.lookup_gpr(name, dependencies)?;
        let mut r1 = None;

        // An unqualified name is first searched in the current package
        if name.package == PackageName::None && current_pkg != PackageName::None
        {
            r1 = project.values[current_pkg as usize].get(&name.name);
        }

        if r1.is_none() {
            r1 = project.values[name.package as usize].get(&name.name);
        }

        r1.ok_or_else(|| Error::not_found(name))
    }

    /// Process one statement
    fn process_one_stmt(
        &mut self,
        dependencies: &[&GprFile],
        scenarios: &mut AllScenarios,
        current_scenario: Scenario,
        current_pkg: PackageName,
        statement: &Statement,
    ) -> std::result::Result<(), Error> {
        match statement {
            Statement::TypeDecl { typename, valid } => {
                let e = ExprValue::new_with_raw(
                    valid,
                    self,
                    dependencies,
                    scenarios,
                    current_scenario,
                    current_pkg,
                )?;
                self.declare(current_pkg, SimpleName::Name(*typename), e)?;
            }

            Statement::VariableDecl {
                name,
                typename,
                expr,
            } => {
                // Is this a scenario variable ?
                // It has both a type and "external".  In this case, we do
                // not check its actual value from the environment or the
                // default, but instead create a ExprValue with a different
                // value for each scenario
                let ext = expr.has_external();
                if let (Some(typename), Some(ext)) = (typename, ext) {
                    let valid =
                        self.lookup(typename, dependencies, current_pkg)?;

                    // Check that this variable wasn't already declared
                    // with a different set of values.
                    scenarios.try_add_variable(
                        ext,
                        valid.as_list().iter().copied().collect::<UstrSet>(),
                    )?;

                    self.declare(
                        current_pkg,
                        SimpleName::Name(*name),
                        ExprValue::new_with_variable(scenarios, ext, valid),
                    )?;

                // Else we have a standard variable (either untyped or not
                // using external), and we get its value from the expression
                } else {
                    self.declare(
                        current_pkg,
                        SimpleName::Name(*name),
                        ExprValue::new_with_raw(
                            expr,
                            self,
                            dependencies,
                            scenarios,
                            current_scenario,
                            current_pkg,
                        )?,
                    )?;
                }
            }

            Statement::AttributeDecl { name, value } => self.declare(
                current_pkg,
                name.clone(),
                ExprValue::new_with_raw(
                    value,
                    self,
                    dependencies,
                    scenarios,
                    current_scenario,
                    current_pkg,
                )?,
            )?,

            Statement::Package {
                name,
                renames,
                extends,
                body,
            } => {
                match (renames, extends) {
                    (Some(r), None) | (None, Some(r)) => {
                        let mut orig = self.lookup_gpr(r, dependencies)?.values
                            [*name as usize]
                            .clone();
                        for (n, expr) in orig.drain() {
                            self.values[*name as usize]
                                .insert(n.clone(), expr.clone());
                        }
                    }
                    _ => {}
                }

                self.process_body(
                    dependencies,
                    scenarios,
                    current_scenario,
                    *name,
                    body,
                )?;
            }

            Statement::Case { varname, when } => {
                // This is a scenario variable, so it's ExprValue is one
                // entry per scenario, with one static string every time.
                // We no longer have the link to the external name, so we use
                // the ExprValue itself.
                let var = self.lookup(varname, dependencies, current_pkg)?;
                let mut remaining = var.prepare_case_stmt()?;

                for w in when {
                    let mut combined = Scenario::default();
                    let mut is_first = true;

                    let mut combine = |s: Scenario| -> Result<(), Error> {
                        if is_first {
                            combined = s;
                            is_first = false;
                        } else {
                            combined = scenarios
                                .union(combined, s)
                                .ok_or(Error::CannotCombineScenarios)?;
                        }
                        Ok(())
                    };

                    for val in &w.values {
                        match val {
                            StringOrOthers::Str(s) => {
                                // If the variable wasn't a scenario
                                // variable, we might not have all possible
                                // values (e.g. Target variable)
                                let c = remaining.get(s);
                                if let Some(c) = c {
                                    combine(*c)?;
                                    remaining.remove(s);
                                }
                            }
                            StringOrOthers::Others => {
                                for s in remaining.values() {
                                    combine(*s)?;
                                }
                                remaining.clear();
                            }
                        }
                    }

                    let s = scenarios.intersection(current_scenario, combined);
                    self.process_body(
                        dependencies,
                        scenarios,
                        s,
                        current_pkg,
                        &w.body,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Process a set of statements
    fn process_body(
        &mut self,
        dependencies: &[&GprFile],
        scenarios: &mut AllScenarios,
        current_scenario: Scenario,
        current_pkg: PackageName,
        body: &StatementList,
    ) -> std::result::Result<(), Error> {
        for s in body {
            self.process_one_stmt(
                dependencies,
                scenarios,
                current_scenario,
                current_pkg,
                &s.1,
            )?;
        }
        Ok(())
    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &mut self,
        raw: &RawGPR,
        extends: Option<&GprFile>,
        dependencies: &[&GprFile],
        scenarios: &mut AllScenarios,
    ) -> std::result::Result<(), Error> {
        if let Some(ext) = extends {
            for v in 0..PACKAGE_NAME_VARIANTS {
                self.values[v] = ext.values[v].clone();
            }
        }

        self.process_body(
            dependencies,
            scenarios,
            Scenario::default(),
            PackageName::None,
            &raw.body,
        )
        .map_err(|e| Error::WithPath {
            path: self.path.clone(),
            error: Box::new(e),
        })
    }
}

impl std::fmt::Debug for GprFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl std::fmt::Display for GprFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
