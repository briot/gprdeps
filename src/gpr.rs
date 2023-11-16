use crate::graph::NodeIndex;
use crate::rawexpr::{
    AttributeOrVarName, PackageName, QualifiedName, Statement,
    PACKAGE_NAME_VARIANTS,
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
        AttributeOrVarName, // variable or attribute name
        ExprValue,          // value for each scenario
    >; PACKAGE_NAME_VARIANTS],
}

impl GPR {
    pub fn new(
        path: &std::path::Path,
        index: NodeIndex,
        name: &str,
    ) -> Self {
        let mut s = Self {
            path: path.into(),
            name: name.to_lowercase(),
            index,
            values: Default::default(),
        };

        // Declare the fallback value for "project'Target" attribute.
        s.values[PackageName::None as usize].insert(
            AttributeOrVarName::Target,
            ExprValue::new_static_str("unknown_target"),
        );

        s
    }

    /// Declare a new named object.
    /// It is an error if such an object already exists.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: AttributeOrVarName,
        value: ExprValue,
    ) -> Result<(), String> {
        println!(
            "MANU {}: declared {}{} as {:?}",
            self, package, name, value
        );
        let pkg = &mut self.values[package as usize];
        if pkg.contains_key(&name) {
            return Err(format!(
                "{}: object already declared {}{}",
                self, package, name
            ));
        }
        pkg.insert(name, value);
        Ok(())
    }

    /// After a project has been processed, we can lookup values of variables
    /// and attributes directly, for each scenario.
    /// The lookup is also done in imported projects.
    pub fn lookup<'a>(
        &'a self,
        name: &QualifiedName,
        dependencies: &'a [&GPR],
    ) -> Result<&'a ExprValue, String> {
        let project = match &name.project {
            None => Some(self),
            Some(c) if c == self.name.as_str() => Some(self),
            Some(n) => dependencies.iter().copied().find(|gpr| gpr.name == *n),
        };
        project
            .and_then(|p| p.values[name.package as usize].get(&name.name))
            .ok_or_else(|| format!("{}: {} not found", self, name))
    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &mut self,
        raw: &RawGPR,
        dependencies: &[&GPR],
        scenarios: &mut AllScenarios,
    ) -> std::result::Result<(), String> {
        let current_scenario = Scenario::default();
        let current_pkg = PackageName::None;

        for s in &raw.body {
            match s {
                Statement::TypeDecl { typename, valid } => {
                    let e = ExprValue::eval(
                        valid,
                        self,
                        dependencies,
                        scenarios,
                        current_scenario,
                    )?;
                    self.declare(
                        current_pkg,
                        AttributeOrVarName::Name(typename.clone()),
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
                        let valid = self.lookup(typename, dependencies)?;

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
                            AttributeOrVarName::Name(name.clone()),
                            ExprValue::from_scenario_variable(
                                scenarios,
                                ext,
                                valid,
                            ),
                        )?;

                    // Else we have a standard variable (either untyped or not
                    // using external), and we get its value from the expression
                    } else {
                        self.declare(
                            current_pkg,
                            AttributeOrVarName::Name(name.clone()),
                            ExprValue::eval(
                                expr,
                                self,
                                dependencies,
                                scenarios,
                                current_scenario,
                            )?,
                        )?;
                    }
                }

//                Statement::AttributeDecl {
//                    name,
//                    index: _index,
//                    ..
//                } => {
//                    println!("{}: attribute {}", self, name);
//                }
                _ => {
                    panic!("{}: Unhandled statement {:?}", self, s);
                }
            }
        }
        Ok(())
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
