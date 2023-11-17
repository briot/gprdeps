use crate::graph::NodeIndex;
use crate::rawexpr::{
    PackageName, QualifiedName, SimpleName, Statement, PACKAGE_NAME_VARIANTS,
    StringOrOthers,
};
use crate::rawgpr::RawGPR;
use crate::scenarios::{AllScenarios, Scenario};
use crate::values::ExprValue;
use std::collections::HashMap;

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
pub struct GPR {
    pub index: NodeIndex,
    name: String,
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
            ExprValue::new_static_str("unknown_target"),
        );
        s.values[PackageName::Linker as usize].insert(
            SimpleName::LinkerOptions,
            ExprValue::new_empty_list(),
        );

        s
    }

    /// Declare a new named object.
    /// It is an error if such an object already exists.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: SimpleName,
        value: ExprValue,
    ) -> Result<(), String> {
        println!("MANU {}: declared {}{} as {:?}", self, package, name, value);
        let pkg = &mut self.values[package as usize];
        if pkg.contains_key(&name) {
            println!("MANU overriding");
//            return Err(format!(
//                "{}: object already declared {}{}",
//                self, package, name
//            ));
        }
        pkg.insert(name, value);
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

    /// Process a set of statements
    pub fn process_body(
        &mut self,
        dependencies: &[&GPR],
        scenarios: &mut AllScenarios,
        current_scenario: Scenario,
        current_pkg: PackageName,
        body: &Vec<Statement>,
    ) -> std::result::Result<(), String> {
        for s in body {
            match s {
                Statement::TypeDecl { typename, valid } => {
                    let e = ExprValue::eval(
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
                                .as_static_list()
                                .iter()
                                .map(|s| s.as_ref())
                                .collect::<Vec<_>>(),
                        )?;

                        self.declare(
                            current_pkg,
                            SimpleName::Name(name.clone()),
                            ExprValue::from_scenario_variable(
                                scenarios, ext, valid,
                            ),
                        )?;

                    // Else we have a standard variable (either untyped or not
                    // using external), and we get its value from the expression
                    } else {
                        self.declare(
                            current_pkg,
                            SimpleName::Name(name.clone()),
                            ExprValue::eval(
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
                        ExprValue::eval(
                            value,
                            self,
                            dependencies,
                            scenarios,
                            current_scenario,
                            current_pkg,
                        )?,
                    )?;
                }

                Statement::Package {
                    name,
                    renames,
                    extends,
                    body,
                } => {
                    if let Some(r) = renames {
                        let mut orig = self
                            .lookup_gpr(r, dependencies)?
                            .values[current_pkg as usize]
                            .clone();
                        for (n, expr) in orig.drain() {
                            self.values[current_pkg as usize].insert(
                                n.clone(), expr.clone()
                            );
                        }
                    }
                    if let Some(e) = extends {
                        let _orig =
                            self.lookup(e, dependencies, current_pkg)?;
                    }

                    self.process_body(
                        dependencies, scenarios,
                        current_scenario,
                        *name,
                        body
                    )?;
                }

                Statement::Case { varname, when } => {
                    // This is a scenario variable, so it's ExprValue is one
                    // entry per scenario, with one static string every time.
                    // We no longer have the link to the external name, so we use
                    // the ExprValue itself.
                    let var = self.lookup(varname, dependencies, current_pkg)?;
                    let mut remaining = var.prepare_case_stmt()?;

                    println!("MANU case {:?}", varname);

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
                                    combine(remaining[s])?;
                                    remaining.remove(s);
                                }
                                StringOrOthers::Others => {
                                    for s in remaining.values() {
                                        combine(*s)?;
                                    }
                                    remaining.clear();
                                }
                            }
                        }

                        let s = scenarios.intersection(
                            current_scenario, combined);
                        println!("MANU   when {} => {}",
                            scenarios.debug(combined),
                            scenarios.debug(s));
                        self.process_body(
                            dependencies, scenarios,
                            s,
                            current_pkg,
                            &w.body
                        )?;
                    }
                }

            }
        }
        Ok(())
    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &mut self,
        raw: &RawGPR,
        dependencies: &[&GPR],
        scenarios: &mut AllScenarios,
    ) -> std::result::Result<(), String> {
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
