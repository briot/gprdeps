use crate::directory::Directory;
use crate::errors::Error;
use crate::graph::NodeIndex;
use crate::rawexpr::{
    PackageName, QualifiedName, SimpleName, Statement, StatementList,
    StringOrOthers, PACKAGE_NAME_VARIANTS,
};
use crate::rawgpr::RawGPR;
use crate::scenarios::{AllScenarios, Scenario};
use crate::values::ExprValue;
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
pub struct GPR {
    pub index: NodeIndex,
    pub name: String,
    path: std::path::PathBuf,
    values: [HashMap<
        SimpleName, // variable or attribute name
        ExprValue,  // value for each scenario
    >; PACKAGE_NAME_VARIANTS],
}

impl GPR {
    pub fn new(path: &std::path::Path, index: NodeIndex, name: &str) -> Self {
        let mut s = Self {
            path: path.into(),
            name: name.to_lowercase(),
            index,
            values: Default::default(),
        };

        // Declare the fallback value for "project'Target" attribute.
        s.values[PackageName::None as usize].insert(
            SimpleName::Target,
            ExprValue::new_with_str("x86_64-linux"),
        );
        s.values[PackageName::Linker as usize]
            .insert(SimpleName::LinkerOptions, ExprValue::new_with_list(&[]));
        s.values[PackageName::None as usize]
            .insert(SimpleName::SourceDirs, ExprValue::new_with_list(&["."]));
        s.values[PackageName::None as usize]
            .insert(SimpleName::ObjectDir, ExprValue::new_with_list(&["."]));
        s.values[PackageName::None as usize]
            .insert(SimpleName::ExecDir, ExprValue::new_with_list(&["."]));
        s
    }

    /// Trim attributes and variables that will no longer be used
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

    /// Resolve relative paths
    pub fn normalize_path(
        &self,
        path: &str,
    ) -> Result<std::path::PathBuf, String> {
        let p = self.path.parent().unwrap().join(path);
        match p.canonicalize() {
            Ok(p) => Ok(p),
            Err(e) => Err(format!("{}: {}", p.display(), e)),
        }
    }

    // Retrieve the value of a string list attribute
    fn strlist_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> &HashMap<Scenario, Vec<String>> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::StrList(sourcedirs)) => sourcedirs,
            v => panic!("Wrong type for attribute {}, {:?}", name, v),
        }
    }

    // Retrieve the value of a path list attribute
    fn pathlist_attr(
        &self,
        pkg: PackageName,
        name: &SimpleName,
    ) -> &HashMap<Scenario, Vec<std::path::PathBuf>> {
        match self.values[pkg as usize].get(name) {
            Some(ExprValue::PathList(sourcedirs)) => sourcedirs,
            v => panic!("Wrong type for attribute {}, {:?}", name, v),
        }
    }


    //  Resolve source directories from the list of relative path names (as
    //  strings) read from the project file, into full paths.
    //  This is done for all scenarios.
    pub fn resolve_source_dirs(
        &mut self,
        dirs: &mut HashSet<Directory>,
    ) -> Result<(), String> {
        let sourcedirs = self.strlist_attr(
            PackageName::None, &SimpleName::SourceDirs);

        let mut resolved_dirs = HashMap::new();

        for (scenar, dirs_in_scenario) in sourcedirs {
            let mut for_scenar = Vec::new();
            for d in dirs_in_scenario {
                if d.ends_with("/**") {
                    let parent =
                        d.chars().take(d.len() - 3).collect::<String>();

                    match self.normalize_path(&parent) {
                        Err(e) => {
                            println!("{}: {}", self, e);
                        }
                        Ok(s) => {
                            for entry in WalkDir::new(s)
                                .follow_links(true)
                                .into_iter()
                                .filter_map(Result::ok)
                                .filter(|e| e.file_type().is_dir())
                            {
                                for_scenar.push(entry.into_path());
                            }
                        }
                    }
                } else {
                    match self.normalize_path(d) {
                        Ok(p) => {
                            for_scenar.push(p);
                        }
                        Err(s) => {
                            println!("{}: {}", self, s);
                        }
                    }
                }
            }

            for d in &for_scenar {
                if !dirs.contains(d) {
                    dirs.insert(Directory::new(d.clone()));
                }
            }
            resolved_dirs.insert(*scenar, for_scenar);
        }

        self.values[PackageName::None as usize].insert(
            SimpleName::SourceDirs,
            ExprValue::PathList(resolved_dirs),
        );

        Ok(())
    }

    /// Return the list of source files for all scenarios
    pub fn get_source_files(&mut self, all_dirs: &HashSet<Directory>) {
        let source_dirs = self.pathlist_attr(
            PackageName::None, &SimpleName::SourceDirs);
        for (scenar_dir, dirs_in_scenario) in source_dirs {
            for d in dirs_in_scenario {
                for f in &all_dirs.get(d).unwrap().files {
                    println!("MANU {} {}={:?}", self, scenar_dir, f);
                }
            }
        }
    }

    /// Declare a new named object.
    /// It is an error if such an object already exists.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: SimpleName,
        value: ExprValue,
    ) -> Result<(), String> {
        self.values[package as usize].insert(name, value);
        Ok(())
    }

    /// Lookup the project file referenced by the given name, in self or its
    /// dependencies.
    fn lookup_gpr<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GPR],
    ) -> Result<&'a GPR, String> {
        match &name.project {
            None => Ok(self),
            Some(c) if c == self.name.as_str() => Ok(self),
            Some(n) => dependencies
                .iter()
                .copied()
                .find(|gpr| gpr.name == *n)
                .ok_or_else(|| format!("{}: {} not found", self, name)),
        }
    }

    /// After a project has been processed, we can lookup values of variables
    /// and attributes directly, for each scenario.
    /// The lookup is also done in imported projects.
    pub fn lookup<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GPR],
        current_pkg: PackageName,
    ) -> Result<&'a ExprValue, String> {
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

        r1.ok_or_else(|| format!("{}: {} not found", self, name))
    }

    /// Process one statement
    fn process_one_stmt(
        &mut self,
        dependencies: &[&GPR],
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
                self.declare(
                    current_pkg,
                    SimpleName::Name(typename.clone()),
                    e,
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
                if let (Some(typename), Some(ext)) = (typename, ext) {
                    let valid =
                        self.lookup(typename, dependencies, current_pkg)?;

                    // Check that this variable wasn't already declared
                    // with a different set of values.
                    scenarios.try_add_variable(
                        ext,
                        &valid
                            .as_list()
                            .iter()
                            .map(|s| s.as_ref())
                            .collect::<Vec<_>>(),
                    )?;

                    self.declare(
                        current_pkg,
                        SimpleName::Name(name.clone()),
                        ExprValue::new_with_variable(scenarios, ext, valid),
                    )?;

                // Else we have a standard variable (either untyped or not
                // using external), and we get its value from the expression
                } else {
                    self.declare(
                        current_pkg,
                        SimpleName::Name(name.clone()),
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

            Statement::AttributeDecl { name, value } => {
                self.declare(
                    current_pkg,
                    name.clone(),
                    ExprValue::new_with_raw(
                            value,
                            self,
                            dependencies,
                            scenarios,
                            current_scenario,
                            current_pkg,
                        )?
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

                    let mut combine = |s: Scenario| -> Result<(), String> {
                        if is_first {
                            combined = s;
                            is_first = false;
                        } else {
                            combined = scenarios
                                .union(combined, s)
                                .ok_or("Could not combine scenarios")?;
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

                    let s =
                        scenarios.intersection(current_scenario, combined)?;
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
        dependencies: &[&GPR],
        scenarios: &mut AllScenarios,
        current_scenario: Scenario,
        current_pkg: PackageName,
        body: &StatementList,
    ) -> std::result::Result<(), Error> {
        for s in body {
            if let Err(e) = self.process_one_stmt(
                dependencies,
                scenarios,
                current_scenario,
                current_pkg,
                &s.1,
            ) {
                Err(e.decorate(None, s.0))?;
            }
        }
        Ok(())
    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &mut self,
        raw: &RawGPR,
        extends: Option<&GPR>,
        dependencies: &[&GPR],
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
    }
}

impl std::fmt::Debug for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
