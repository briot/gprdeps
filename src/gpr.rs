use crate::directory::Directory;
use crate::errors::Error;
use crate::rawexpr::{
    PackageName, QualifiedName, SimpleName, Statement, StatementList,
    StringOrOthers, PACKAGE_NAME_VARIANTS,
};
use crate::rawgpr::RawGPR;
use crate::scenarios::{AllScenarios, Scenario};
use crate::settings::Settings;
use crate::values::ExprValue;
use path_clean::PathClean;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ustr::Ustr;
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

/// Is this an attribute we want to keep in the project ?
fn keep_attribute(name: &SimpleName) -> bool {
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
}

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
#[derive(Default)]
pub struct GprFile {
    pub name: Ustr,
    path: PathBuf,
    values: [HashMap<
        SimpleName, // variable or attribute name
        ExprValue,  // value for each scenario
    >; PACKAGE_NAME_VARIANTS],

    pub source_files: HashMap<Scenario, Vec<(PathBuf, Ustr)>>, // path and lang
}

impl GprFile {
    pub fn new(path: &Path) -> Self {
        let mut s = Self {
            path: path.to_path_buf(),
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
            self.values[pkg].retain(|name, _| keep_attribute(name));
        }
    }

    /// Find which scenarios are actually useful for this project
    //    pub fn find_used_scenarios(&self, useful: &mut HashSet<Scenario>) {
    //        for pkg in 0..PACKAGE_NAME_VARIANTS {
    //            for v in self.values[pkg].values() {
    //                v.find_used_scenarios(useful);
    //            }
    //        }
    //    }

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
    //    fn str_attr(
    //        &self,
    //        pkg: PackageName,
    //        name: &SimpleName,
    //    ) -> &HashMap<Scenario, Ustr> {
    //        match self.values[pkg as usize].get(name) {
    //            Some(ExprValue::Str(v)) => v,
    //            v => panic!("Wrong type for attribute {}{}, {:?}", pkg, name, v),
    //        }
    //    }

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

    /// Return the list of source files for all scenarios
    pub fn resolve_source_files(
        &mut self,
        all_dirs: &HashSet<Directory>,
        scenarios: &mut AllScenarios,
    ) {
        let source_dirs =
            self.pathlist_attr(PackageName::None, &SimpleName::SourceDirs);
        let mut files: HashMap<Scenario, Vec<(PathBuf, Ustr)>> = HashMap::new();

        for (scenar_dir, dirs_in_scenar) in source_dirs {
            for (name, val) in &self.values[PackageName::Naming as usize] {
                match (name, val) {
                    (
                        SimpleName::SpecSuffix(lang)
                        | SimpleName::BodySuffix(lang),
                        ExprValue::Str(v),
                    ) => {
                        for (scenar_attr, suffix) in v {
                            match scenarios
                                .intersection(*scenar_attr, *scenar_dir)
                            {
                                None => continue,
                                Some(s) => {
                                    let sfiles = files.entry(s).or_default();
                                    for d in dirs_in_scenar {
                                        if let Some(dir) = all_dirs.get(d) {
                                            dir.filter_suffix(
                                                suffix, *lang, sfiles,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    (
                        SimpleName::Spec(_) | SimpleName::Body(_),
                        ExprValue::Str(v),
                    ) => {
                        for (scenar_attr, basename) in v {
                            match scenarios
                                .intersection(*scenar_attr, *scenar_dir)
                            {
                                None => continue,
                                Some(s) => {
                                    let sfiles = files.entry(s).or_default();
                                    for d in dirs_in_scenar {
                                        if let Some(dir) = all_dirs.get(d) {
                                            dir.add_if_found(
                                                basename, *CST_ADA, sfiles,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        self.source_files = files;
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
                    let valid_expr =
                        self.lookup(typename, dependencies, current_pkg)?;
                    let mut valid = valid_expr.as_list()
                        .iter().copied().collect::<Vec<_>>();
                    valid.sort();  // ??? is this needed, is the type sorted

                    // Check that this variable wasn't already declared
                    // with a different set of values.
                    scenarios.try_add_variable(ext, &valid)?;

                    self.declare(
                        current_pkg,
                        SimpleName::Name(*name),
                        ExprValue::Str(scenarios.expr_from_variable(ext)),
                    )?;

                // Else we have a standard variable (either untyped or not
                // using external), and we get its value from the expression
                } else {
                    let e = ExprValue::new_with_raw(
                        expr,
                        self,
                        dependencies,
                        scenarios,
                        current_scenario,
                        current_pkg,
                    )?;
                    self.declare(current_pkg, SimpleName::Name(*name), e)?;
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
                // This is a scenario variable, so its ExprValue is one
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

                    if let Some(s) =
                        scenarios.intersection(current_scenario, combined)
                    {
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
        self.name = raw.name;

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

    /// Print details about the project
    pub fn print_details(&self, scenarios: &AllScenarios, print_vars: bool) {
        println!("file: {}", self.path.display());
        println!("project {} is", self.name);

        for pkgidx in 0..PACKAGE_NAME_VARIANTS {
            if self.values[pkgidx].is_empty() {
                continue;
            }
            let pkg: PackageName = unsafe { std::mem::transmute(pkgidx) };
            for (attrname, value) in &self.values[pkgidx] {
                if print_vars || !matches!(attrname, SimpleName::Name(_)) {
                    println!(
                        "   for {}{}\n{}",
                        pkg,
                        attrname,
                        value.format(scenarios, "      ", "\n"),
                    );
                }
            }
        }
        // TODO should display self.source_files
        println!("end project;");
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

#[cfg(test)]
pub mod tests {
    use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
    use crate::errors::Error;
    use crate::gpr::GprFile;
    use crate::gpr_scanner::{GprPathToIndex, GprScanner};
    use crate::rawexpr::{PackageName, SimpleName};
    use crate::rawgpr::RawGPR;
    use crate::scenarios::AllScenarios;
    use crate::settings::Settings;
    use std::path::Path;
    use ustr::Ustr;

    /// Parse a project, for a test

    pub fn parse(s: &str) -> Result<RawGPR, Error> {
        let mut file = crate::files::File::new_from_str(s);
        let settings = Settings::default();
        let options = AdaLexerOptions {
            kw_aggregate: true,
            kw_body: false,
        };
        let lex = AdaLexer::new(&mut file, options);
        let path_to_id: GprPathToIndex = Default::default();
        GprScanner::parse(lex, Path::new("memory"), &path_to_id, &settings)
    }

    /// Return a process project

    pub fn process(
        raw: &RawGPR,
        scenarios: &mut AllScenarios,
    ) -> Result<GprFile, Error> {
        let mut gpr = GprFile::new(&raw.path);
        gpr.process(raw, None, &[], scenarios)?;
        Ok(gpr)
    }

    /// Asserts the value of a variable

    pub fn assert_variable(
        gpr: &GprFile,
        pkg: PackageName,
        name: &str,
        scenarios: &AllScenarios,
        expected: &str,
    ) {
        let v =
            gpr.values[pkg as usize].get(&SimpleName::Name(Ustr::from(name)));
        let actual = match v {
            None    => "NONE".to_string(),
            Some(a) => a.format(scenarios, "", "\n"),
        };
        assert_eq!(actual, expected);
    }
}
