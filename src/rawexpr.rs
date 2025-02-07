/// The un-interpreted tree, as parsed from a GPR file
use crate::errors::Error;
use crate::packagename::PackageName;
use crate::simplename::{SimpleName, StringOrOthers};
use std::fmt::Debug;
use ustr::Ustr;

/// A fully qualified name.
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

#[derive(Debug, PartialEq)]
pub struct WhenClause {
    pub values: Vec<StringOrOthers>,
    pub body: StatementList,
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Package {
        name: PackageName,
        renames: Option<QualifiedName>,
        extends: Option<QualifiedName>,
        body: StatementList,
    },
    TypeDecl {
        typename: Ustr,
        valid: RawExpr,
    },
    AttributeDecl {
        name: SimpleName,
        value: RawExpr,
    },
    VariableDecl {
        name: Ustr,
        typename: Option<QualifiedName>,
        expr: RawExpr,
    },
    Case {
        varname: QualifiedName,
        when: Vec<WhenClause>,
    },
}

/// Line + Statement
pub type StatementList = Vec<(u32, Statement)>;

#[derive(Debug, PartialEq)]
pub enum RawExpr {
    Empty,
    Others,
    Str(Ustr), //  doesn't include surrounding quotes
    Name(QualifiedName),
    FuncCall((QualifiedName, Vec<RawExpr>)),
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    List(Vec<RawExpr>),
}

lazy_static::lazy_static! {
    static ref EXTERNAL: Ustr = Ustr::from("external");
}

impl RawExpr {
    /// Whether the expression contains a call to external().
    /// Returns the name of the scenario variable
    pub fn has_external(&self) -> Option<Ustr> {
        match self {
            RawExpr::Ampersand((left, right)) => {
                left.has_external().or_else(|| right.has_external())
            }
            RawExpr::List(v) => v.iter().find_map(|e| e.has_external()),
            RawExpr::FuncCall((
                QualifiedName {
                    project: None,
                    package: PackageName::None,
                    name: SimpleName::Name(n),
                },
                args,
            )) => {
                if *n == *EXTERNAL {
                    match &args[0] {
                        RawExpr::Str(s) => Some(*s),
                        _ => panic!(
                            "First argument to external must \
                                 be static string"
                        ),
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: RawExpr) -> RawExpr {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Convert to a static string
    /// ??? Should use values.rs
    pub fn as_static_str(&self) -> Result<Ustr, Error> {
        match self {
            RawExpr::Str(s) => Ok(*s),
            _ => Err(Error::NotStaticString),
        }
    }

    /// Convert to a list of static strings
    pub fn as_list(&self) -> Result<Vec<Ustr>, Error> {
        match self {
            RawExpr::List(s) => s.iter().map(|e| e.as_static_str()).collect(),
            _ => Err(Error::NotStaticString),
        }
    }

    /// Convert a list of static strings to lower case
    pub fn to_lowercase(&self) -> RawExpr {
        match &self {
            RawExpr::Str(s) => {
                RawExpr::Str(Ustr::from(&s.as_str().to_lowercase()))
            }
            RawExpr::List(s) => {
                RawExpr::List(s.iter().map(|e| e.to_lowercase()).collect())
            }
            _ => panic!("Can only convert static list to lower-case"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::rawexpr::RawExpr;
    use ustr::Ustr;

    pub fn build_expr_str(s: &str) -> RawExpr {
        RawExpr::Str(Ustr::from(s))
    }

    pub fn build_expr_list(s: &[&str]) -> RawExpr {
        let v = s.iter().map(|st| build_expr_str(st)).collect::<Vec<_>>();
        RawExpr::List(v)
    }
}
