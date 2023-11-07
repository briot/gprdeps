use crate::environment::{DepGraph, NodeIndex};
use crate::scenarios::AllScenarios;
use crate::rawexpr::{
    AttributeOrVarName, PackageName, QualifiedName, Statement,
};
use crate::rawgpr::RawGPR;

/// A specific GPR file
/// Such an object is independent of the scanner that created it, though it
/// needs an Environment object to resolve paths.
pub struct GPR {
    index: NodeIndex,
    path: std::path::PathBuf,
    raw: RawGPR,
}

impl GPR {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            index: Default::default(),
            raw: Default::default(),
        }
    }

    pub fn set_raw(&mut self, raw: RawGPR, index: NodeIndex) {
        self.index = index;
        self.raw = raw;
    }

    /// Find the declaration for the given package, in Self
    fn find_package_decl(
        &self,
        pkg: Option<PackageName>,
    ) -> std::result::Result<&Vec<Statement>, String> {
        match pkg {
            None => Ok(&self.raw.body),
            Some(p) => {
                for s in &self.raw.body {
                    if let Statement::Package { name, body, .. } = s {
                        if *name == p {
                            return Ok(body);
                        }
                    }
                }
                Err(format!("Package not found: {:?}", pkg))
            }
        }
    }

    /// Find the given named object in self (but not in imported projects).
    fn find_named_in_project(
        &self,
        name: &QualifiedName,
    ) -> std::result::Result<&Statement, String> {
        for stmt in self.find_package_decl(name.package)? {
            match (&name.name, stmt) {
                (
                    AttributeOrVarName::Name(n),
                    Statement::TypeDecl { typename, .. },
                ) if *n == *typename => return Ok(stmt),

                (
                    n,
                    Statement::AttributeDecl { name, .. },
                ) if *n == *name => return Ok(stmt),

                (
                    AttributeOrVarName::Name(n),
                    Statement::VariableDecl { name, .. },
                ) if *n == *name => return Ok(stmt),

                _ => {}
            }
        }
        Err(format!("{:?} is not defined in {}", name, self))
    }

    /// Find the given named object in self or its imported projects
    fn find_named<'a>(
        &'a self,
        name: &QualifiedName,
        graph: &'a DepGraph,
    ) -> std::result::Result<&'a Statement, String> {
        let current_project = self.raw.name.as_str();
        match &name.project {
            None => self.find_named_in_project(name),
            Some(c) if c == current_project => self.find_named_in_project(name),
            Some(n) => {
                for gpr in graph.gpr_dependencies(self.index) {
                    if gpr.raw.name == *n {
                        //  ??? Possibly searching multiple times in same project
                        let found = gpr.find_named(name, graph);
                        if found.is_ok() {
                            return found;
                        }
                    }
                }
                Err(format!("{}: Object not found {}", self, name))
            }
        }
    }

    /// Process the raw gpr file into the final list of attributes
    pub fn process(
        &self,
        graph: &DepGraph,
        scenarios: &mut AllScenarios,
    ) -> std::result::Result<(), String> {
        for s in &self.raw.body {
            match s {
                Statement::TypeDecl { .. } => {
                },
                Statement::VariableDecl { typename, expr, .. } => {
                    // Is this a scenario variable ?
                    let ext = expr.has_external();
                    if let (Some(typename), Some(ext)) = (typename, ext) {
                        let t = self.find_named(typename, graph)?;
                        if let Statement::TypeDecl { valid, .. } = t {
                            scenarios.try_add_variable(ext, valid.clone())?;
                        }
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for GPR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)
    }
}
