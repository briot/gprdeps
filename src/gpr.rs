use crate::environment::NodeIndex;
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
    index: NodeIndex,
    name: String,
    path: std::path::PathBuf,
    values: [HashMap<
        AttributeOrVarName, // variable or attribute name
        ExprValue,          // value for each scenario
    >; PACKAGE_NAME_VARIANTS],
}

impl GPR {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            name: Default::default(),
            index: Default::default(),
            values: Default::default(),
        }
    }

    pub fn set_raw(&mut self, name: &str, index: NodeIndex) {
        self.name = name.to_string();
        self.index = index;
    }

    /// Declare a new named object.
    /// It is an error if such an object already exists.
    pub fn declare(
        &mut self,
        package: PackageName,
        name: AttributeOrVarName,
        value: ExprValue,
    ) -> Result<(), String> {
        let pkg = &mut self.values[package as usize];
        if pkg.contains_key(&name) {
            return Err(format!(
                "{}: object already declared {}.{}",
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
            Some(n) => dependencies
                .iter()
                .copied()
                .find(|gpr| gpr.name == *n),
        };
        project
            .and_then(|p| p.values[name.package as usize].get(&name.name))
            .ok_or_else(|| format!("{}: {} not found", self, name))
    }

    //    /// Find the declaration for the given package, in Self
    //    /// This should be used before the project has been processed.
    //    fn find_raw_package_decl(
    //        &self,
    //        pkg: PackageName,
    //    ) -> Result<&Vec<Statement>, String> {
    //        match pkg {
    //            PackageName::None => Ok(&self.raw.body),
    //            p => {
    //                for s in &self.raw.body {
    //                    if let Statement::Package { name, body, .. } = s {
    //                        if *name == p {
    //                            return Ok(body);
    //                        }
    //                    }
    //                }
    //                Err(format!("Package not found: {:?}", pkg))
    //            }
    //        }
    //    }
    //
    //    /// Find the given named object in self (but not in imported projects).
    //    /// This should be used before the project has been processed.
    //    fn find_raw_named_in_project(
    //        &self,
    //        name: &QualifiedName,
    //    ) -> std::result::Result<&Statement, String> {
    //        for stmt in self.find_raw_package_decl(name.package)? {
    //            match (&name.name, stmt) {
    //                (
    //                    AttributeOrVarName::Name(n),
    //                    Statement::TypeDecl { typename, .. },
    //                ) if *n == *typename => return Ok(stmt),
    //
    //                (n, Statement::AttributeDecl { name, .. }) if *n == *name => {
    //                    return Ok(stmt)
    //                }
    //
    //                (
    //                    AttributeOrVarName::Name(n),
    //                    Statement::VariableDecl { name, .. },
    //                ) if *n == *name => return Ok(stmt),
    //
    //                _ => {}
    //            }
    //        }
    //        Err(format!("{:?} is not defined in {}", name, self))
    //    }
    //
    //    /// Find the given named object in self or its imported projects.
    //    /// This should be used before the project has been processed.
    //    pub fn find_raw_named<'a>(
    //        &'a self,
    //        name: &QualifiedName,
    //        graph: &'a DepGraph,
    //    ) -> std::result::Result<&'a Statement, String> {
    //        let current_project = self.raw.name.as_str();
    //        match &name.project {
    //            None => self.find_raw_named_in_project(name),
    //            Some(c) if c == current_project => {
    //                self.find_raw_named_in_project(name)
    //            }
    //            Some(n) => {
    //                for gpr in graph.gpr_dependencies(self.index) {
    //                    if gpr.raw.name == *n {
    //                        //  ??? Possibly searching multiple times in same project
    //                        let found = gpr.find_raw_named(name, graph);
    //                        if found.is_ok() {
    //                            return found;
    //                        }
    //                    }
    //                }
    //                Err(format!("{}: Object not found {}", self, name))
    //            }
    //        }
    //    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &self,
        raw: &RawGPR,
        dependencies: &[&GPR],
        scenarios: &mut AllScenarios,
    ) -> std::result::Result<(), String> {
        let current_scenario = Scenario::default();
        let _current_pkg = PackageName::None;

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
                    println!("{}: type {} {:?}", self, typename, e);
                    // self.declare(
                    //     current_pkg,
                    //     AttributeOrVarName::Name(typename.clone()),
                    //     e,
                    // );
                }

                Statement::VariableDecl {
                    name: _name,
                    typename: _typename,
                    expr: _expr,
                } => {
                    // // Is this a scenario variable ?
                    // let ext = expr.has_external();
                    // if let (Some(typename), Some(ext)) = (typename, ext) {
                    //     let t = self.find_raw_named(typename, graph)?;
                    //     if let Statement::TypeDecl { valid, .. } = t {
                    //         scenarios.try_add_variable(ext, valid)?;
                    //     }
                    //
                    // // Else a simple variable
                    // } else {
                    //     println!("{}: variable {} {:?}", self, name, expr);
                    //     vars.insert(name.clone(), expr);
                    // }
                }
                Statement::AttributeDecl {
                    name,
                    index: _index,
                    ..
                } => {
                    println!("{}: attribute {}", self, name);
                }
                _ => {
                    panic!("{}: Unhandled statement {:?}", self, s);
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
