/// A fully qualified name in the scanner.
/// The scanner itself cannot distinguish between attributes, variables and
/// function names, this requires access to the symbol table.  For instance:
///     for Source_Files use Source_Files & (..);  --  an attribute
///     for Source_Files use My_List & (..);       --  a variable
///
///     Switches ("Ada")   --  an attribute
///     external ("Ada")   --  a function call
///
/// We know the depth of the names hierarchy, but again the parser is not able
/// to distinguish between packages and projects (though it does have a list
/// of hard-coded package names).
///     name
///     name (index)
///     package.name
///     package'name
///     package'name (index)
///     project.package'name
///     package'name
///     project'name
use crate::packagename::PackageName;
use crate::simplename::SimpleName;
use std::fmt::Debug;
use ustr::Ustr;

#[derive(Debug, PartialEq)]
pub struct QualifiedName {
    pub project: Option<Ustr>, // None for current project or "Project'"
    pub package: PackageName,
    pub name: SimpleName,
}

impl QualifiedName {
    /// When we find a name in the source which an optional leading identifier,
    /// the latter could be either a project or a package.  This function will
    /// guess as needed.
    pub fn from_two(prj_or_pkg: Option<Ustr>, name: SimpleName) -> Self {
        match prj_or_pkg {
            None => QualifiedName {
                project: prj_or_pkg,
                package: PackageName::None,
                name,
            },
            Some(n1) => match PackageName::new(n1) {
                Ok(p) => QualifiedName {
                    project: None,
                    package: p,
                    name,
                },
                Err(_) => QualifiedName {
                    project: Some(n1),
                    package: PackageName::None,
                    name,
                },
            },
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = &self.project {
            write!(f, "{}.", p)?;
        }
        write!(f, "{}{}", self.package, self.name)?;
        Ok(())
    }
}
