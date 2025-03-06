use crate::{
    allscenarios::AllScenarios,
    directory::Directory,
    environment::{Environment, GprMap},
    errors::Error,
    graph::NodeIndex,
    naming::{FileInGPR, Naming},
    packagename::{PackageName, PACKAGE_NAME_VARIANTS},
    perscenario::PerScenario,
    qnames::QName,
    qualifiedname::QualifiedName,
    rawexpr::{Statement, StatementList},
    rawgpr::RawGPR,
    scenarios::Scenario,
    settings::Settings,
    simplename::SimpleName,
    values::ExprValue,
};
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
    pub node: NodeIndex,
    pub name: Ustr,
    is_library: bool,
    is_aggregate: bool,
    is_abstract: bool,
    path: PathBuf,
    types: [HashMap<SimpleName, Vec<Ustr>>; PACKAGE_NAME_VARIANTS],

    values: [HashMap<
        SimpleName, // variable or attribute name
        ExprValue,  // value for each scenario
    >; PACKAGE_NAME_VARIANTS],

    // List of source directories, after resolving relative paths and /** from
    // the Source_Dirs attribute
    pub source_dirs: PerScenario<Vec<PathBuf>>,

    // The Naming scheme, and list of source files
    pub naming: PerScenario<Naming>,
    pub sources: PerScenario<Vec<FileInGPR>>,
}

impl GprFile {
    pub fn new(
        path: &Path,
        is_abstract: bool,
        is_aggregate: bool,
        is_library: bool,
        node: NodeIndex,
    ) -> Self {
        let mut s = Self {
            path: path.to_path_buf(),
            is_abstract,
            is_aggregate,
            is_library,
            node,
            ..Default::default()
        };

        // Declare the fallback value for "project'Target" attribute.
        s.values[PackageName::None as usize].insert(
            SimpleName::Target,
            ExprValue::new_with_str(*CST_X86_64_LINUX),
        );
        s.values[PackageName::Linker as usize].insert(
            SimpleName::LinkerOptions,
            ExprValue::new_with_list(vec![]),
        );
        s.values[PackageName::None as usize].insert(
            SimpleName::SourceDirs,
            ExprValue::new_with_list(vec![*CST_DOT]),
        );
        s.values[PackageName::None as usize].insert(
            SimpleName::ObjectDir,
            ExprValue::new_with_list(vec![*CST_DOT]),
        );
        s.values[PackageName::None as usize].insert(
            SimpleName::ExecDir,
            ExprValue::new_with_list(vec![*CST_DOT]),
        );
        s.values[PackageName::None as usize].insert(
            SimpleName::Languages,
            ExprValue::new_with_list(vec![*CST_ADA]),
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

    // Retrieve the value of a string list attribute
    pub fn strlist_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> Option<&PerScenario<Vec<Ustr>>> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::StrList(v)) => Some(v),
            None => None,
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
        let sourcedirs = self
            .strlist_attr(PackageName::None, &SimpleName::SourceDirs)
            .expect("Source_Dirs should always have a value");
        self.source_dirs = sourcedirs.map(|dirs_in_scenario| {
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
            for_scenar
        });
        Ok(())
    }

    /// Resolve the naming scheme for each scenario
    pub fn resolve_naming(&mut self, scenars: &mut AllScenarios) {
        let mut naming =
            self.source_dirs.map(|d| Naming::new_with_dirs(d.clone()));

        if let Some(sf) =
            self.strlist_attr(PackageName::None, &SimpleName::SourceFiles)
        {
            naming.update(sf, Scenario::default(), scenars, |naming, files| {
                naming.set_source_files(files.iter().cloned().collect())
            });
        }

        if let Some(sf) =
            self.strlist_attr(PackageName::None, &SimpleName::Main)
        {
            naming.update(sf, Scenario::default(), scenars, |naming, files| {
                naming.main = Some(files.iter().cloned().collect());
            });
        }

        if let Some(sf) =
            self.strlist_attr(PackageName::None, &SimpleName::LibraryInterface)
        {
            naming.update(sf, Scenario::default(), scenars, |naming, units| {
                naming.library_interfaces = Some(
                    units.iter().map(|u| QName::from_str(u, ".")).collect(),
                );
            });
        }

        naming.update(
            self.strlist_attr(PackageName::None, &SimpleName::Languages)
                .expect("Languages attribute is always defined"),
            Scenario::default(),
            scenars,
            |naming, langs| naming.set_languages(langs.clone()),
        );

        for (name, val) in &self.values[PackageName::Naming as usize] {
            match (name, val) {
                (SimpleName::SpecSuffix(lang), ExprValue::Str(v)) => {
                    naming.update(
                        v,
                        Scenario::default(),
                        scenars,
                        |naming, suffix| {
                            naming.spec_suffix.insert(*lang, *suffix);
                        },
                    );
                }
                (SimpleName::BodySuffix(lang), ExprValue::Str(v)) => {
                    naming.update(
                        v,
                        Scenario::default(),
                        scenars,
                        |naming, suffix| {
                            naming.body_suffix.insert(*lang, *suffix);
                        },
                    );
                }
                (SimpleName::Spec(unit), ExprValue::Str(v)) => {
                    naming.update(
                        v,
                        Scenario::default(),
                        scenars,
                        |naming, filename| {
                            naming.spec_files.insert(*unit, *filename);
                        },
                    );
                }
                (SimpleName::Body(unit), ExprValue::Str(v)) => {
                    naming.update(
                        v,
                        Scenario::default(),
                        scenars,
                        |naming, filename| {
                            naming.body_files.insert(*unit, *filename);
                        },
                    );
                }
                (SimpleName::DotReplacement, ExprValue::Str(v)) => {
                    naming.update(
                        v,
                        Scenario::default(),
                        scenars,
                        |naming, repl| {
                            naming.dot_replacement = *repl;
                        },
                    );
                }
                _ => {
                    panic!("Unexpected attribute Naming{}", name);
                }
            }
        }

        self.naming = naming;
    }

    /// Return the list of source files for all scenarios
    pub fn resolve_source_files(
        &mut self,
        env: &mut Environment,
        all_dirs: &HashSet<Directory>,
    ) {
        if !self.is_abstract && (!self.is_library || !self.is_aggregate) {
            self.sources = self.naming.map(|naming| {
                naming
                    .find_source_files(env, all_dirs)
                    .expect("should handle error")
            });
        }
    }

    /// Once all projects have been processed, this goes through aggregate
    /// library projects and mark the sources of their aggregated projects
    /// as library interface, as needed.
    pub fn resolve_library_interface(
        &self,
        scenars: &mut AllScenarios,
        gprs: &GprMap,
        settings: &Settings,
    ) {
        if !self.is_library || !self.is_aggregate {
            return;
        }

        let intf =
            self.strlist_attr(PackageName::None, &SimpleName::LibraryInterface);
        let prj =
            self.strlist_attr(PackageName::None, &SimpleName::ProjectFiles);
        if let Some(intf) = intf {
            if let Some(prj) = prj {
                let mut prj_intf: PerScenario<(Vec<QName>, Vec<&GprFile>)> =
                    intf.map(|i| {
                        (
                            i.iter().map(|i| QName::from_str(i, ".")).collect(),
                            vec![],
                        )
                    });
                prj_intf.update(
                    prj,
                    Scenario::default(),
                    scenars,
                    |prj_int, p| {
                        prj_int.1.extend(
                            p.iter()
                                .filter_map(|p| {
                                    self.normalize_path(p, settings)
                                })
                                .filter_map(|p| gprs.get(&p)),
                        )
                    },
                );

                for (s, prj_int) in prj_intf.iter() {
                    for gpr in &prj_int.1 {
                        for (s2, sources) in gpr.sources.iter() {
                            if s & s2 != Scenario::empty() {
                                for source in sources.iter() {
                                    let mut sm = source.file.borrow_mut();
                                    sm.is_library_interface =
                                        prj_int.0.contains(&sm.unitname);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Declare a new type
    pub fn declare_type(
        &mut self,
        package: PackageName,
        name: SimpleName,
        mut valid: Vec<Ustr>,
    ) -> Result<(), Error> {
        let old = self.types[package as usize].get(&name);
        if old.is_none() {
            valid.sort();
            self.types[package as usize].insert(name, valid);
            Ok(())
        } else {
            Err(Error::AlreadyDeclared)?
        }
    }

    /// Declare a new named object (or assign a new value to an existing
    /// object).  Note that what we receive are delta values, which only
    /// have relevant values for the current context (e.g. case statements).
    ///     case E is
    ///        when "on" =>   V := V1 & V2;
    /// then delta should be the value of V1 & V2 and might only include a
    /// value for the scenario "E=on".  But if V already had values for other
    /// scenarios they should be preserved.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: SimpleName,
        context: Scenario,
        scenars: &mut AllScenarios,
        mut delta: ExprValue,
    ) -> Result<(), Error> {
        let old = self.values[package as usize].get(&name);
        if old.is_none() {
            self.values[package as usize].insert(name, delta);
            return Ok(());
        }

        let mut old = old.unwrap().clone();

        match (&mut old, &mut delta) {
            (ExprValue::Str(ov), ExprValue::Str(d)) => {
                ov.update(d, context, scenars, |old, new| *old = *new);
            }
            (ExprValue::StrList(ov), ExprValue::Str(d)) => {
                ov.update(d, context, scenars, |old, new| *old = vec![*new]);
            }
            (ExprValue::StrList(ov), ExprValue::StrList(d)) => {
                ov.update(d, context, scenars, |old, new| *old = new.clone());
            }
            _ => {
                Err(Error::VariableMustBeString)?;
            }
        }

        self.values[package as usize].insert(name, old);
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

    /// Lookup a type definition
    pub fn lookup_type<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GprFile],
        current_pkg: PackageName,
    ) -> Result<&'a Vec<Ustr>, Error> {
        let project = self.lookup_gpr(name, dependencies)?;
        let mut r1 = None;

        // An unqualified name is first searched in the current package
        if name.package == PackageName::None && current_pkg != PackageName::None
        {
            r1 = project.types[current_pkg as usize].get(&name.name);
        }

        if r1.is_none() {
            r1 = project.types[name.package as usize].get(&name.name);
        }

        r1.ok_or_else(|| Error::not_found(name))
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
        context: Scenario,
        current_pkg: PackageName,
        statement: &Statement,
    ) -> std::result::Result<(), Error> {
        match statement {
            Statement::TypeDecl { typename, valid } => {
                self.declare_type(
                    current_pkg,
                    SimpleName::Name(*typename),
                    valid.as_list()?,
                )?;
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
                let expr = match (typename, ext) {
                    (Some(typename), Some(ext)) => {
                        let valid = self.lookup_type(
                            typename,
                            dependencies,
                            current_pkg,
                        )?;

                        // Check that this variable wasn't already declared
                        // with a different set of values.
                        scenarios
                            .try_add_variable(ext.0, valid, ext.1)?
                            .value()
                            .clone()
                    }
                    _ => {
                        // Else we have a standard variable (either untyped
                        // or not using external), and we get its value from
                        // the expression
                        ExprValue::new_with_raw(
                            expr,
                            self,
                            dependencies,
                            scenarios,
                            context,
                            current_pkg,
                        )?
                    }
                };

                self.declare(
                    current_pkg,
                    SimpleName::Name(*name),
                    context,
                    scenarios,
                    expr,
                )?;
            }

            Statement::AttributeDecl { name, value } => {
                let expr = ExprValue::new_with_raw(
                    value,
                    self,
                    dependencies,
                    scenarios,
                    context,
                    current_pkg,
                )?;
                self.declare(
                    current_pkg,
                    name.clone(),
                    context,
                    scenarios,
                    expr,
                )?
            }

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
                    context,
                    *name,
                    body,
                )?;
            }

            Statement::Case { varname, when } => {
                // * remaining is the remaining list of valid values for var.
                //   It becomes smaller with each WhenClause.
                // * var is the list of valid values for the scenario variable.

                let mut case_stmt =
                    match self.lookup(varname, dependencies, current_pkg)? {
                        ExprValue::Str(per_scenario) => {
                            scenarios.prepare_case_stmt(per_scenario)
                        }
                        _ => Err(Error::VariableMustBeString)?,
                    };

                for w in when {
                    let scenar = scenarios.process_when_clause(
                        context,
                        &mut case_stmt,
                        w,
                    );
                    if scenar.is_empty() {
                        if !w.body.is_empty() {
                            // ??? Should report proper location
                            //Err(Error::UselessWhenClause)?;
                            println!("{:?} Useless when clause", self);
                        }
                    } else {
                        self.process_body(
                            dependencies,
                            scenarios,
                            scenar,
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
        context: Scenario,
        current_pkg: PackageName,
        body: &StatementList,
    ) -> std::result::Result<(), Error> {
        for s in body {
            self.process_one_stmt(
                dependencies,
                scenarios,
                context,
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
                self.values[v].clone_from(&ext.values[v]);
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

    /// Find all scenarios that result in different values in the project
    pub fn find_used_scenarios(&self, scenars: &mut HashSet<Scenario>) {
        for pkg in 0..PACKAGE_NAME_VARIANTS {
            for v in self.values[pkg].values() {
                v.find_used_scenarios(scenars);
            }
        }
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
    use crate::{
        ada_lexer::{AdaLexer, AdaLexerOptions},
        allscenarios::AllScenarios,
        errors::Error,
        gpr::GprFile,
        gpr_scanner::GprScanner,
        graph::NodeIndex,
        packagename::PackageName,
        rawgpr::RawGPR,
        settings::Settings,
        simplename::SimpleName,
    };
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
        let lex = AdaLexer::new(&mut file, options)?;
        GprScanner::parse(lex, Path::new("memory"), &settings)
    }

    /// Return a process project
    pub fn process(
        raw: &RawGPR,
        scenarios: &mut AllScenarios,
    ) -> Result<GprFile, Error> {
        let mut gpr = GprFile::new(
            &raw.path,
            raw.is_abstract,
            raw.is_aggregate,
            raw.is_library,
            NodeIndex::new(0),
        );
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
            None => "NONE".to_string(),
            Some(a) => a.format(scenarios, "", "\n"),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn find_source_files() -> Result<(), Error> {
        let raw = crate::gpr::tests::parse(
            r#"project P is
               type T is ("a", "b", "c");
               E1 : T := external ("e1");
               E2 : T := external ("e2");

               case E1 is
                  when "a" => for Source_Dirs use ("dir_a", "dir_shared");
                  when "b" => for Source_Dirs use ("dir_b", "dir_shared");
                  when "c" => for Source_Dirs use ("dir_c", "dir_shared");
               end case;

               package Naming is
                  case E2 is
                     when "a" | "b" => for Spec_Suffix ("Ada") use ".1.ads";
                     when "c" => for Spec_Suffix ("Ada") use ".2.ads";
                  end case;
               end Naming;

               end P;"#,
        )?;
        let mut scenarios = crate::allscenarios::AllScenarios::default();
        let gpr = crate::gpr::tests::process(&raw, &mut scenarios)?;
        gpr.print_details(&scenarios, true);
        Ok(())
    }
}
