//! The un-interpreted tree, as parsed from a GPR file
use crate::{
    errors::Error,
    packagename::PackageName,
    qualifiedname::QualifiedName,
    simplename::{SimpleName, StringOrOthers},
};
use std::fmt::Debug;
use ustr::Ustr;

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
    /// Returns the name of the scenario variable, and the default value.
    pub fn has_external(&self) -> Option<(Ustr, Option<Ustr>)> {
        match self {
            // ??? Fails if we have two calls to external
            //     external("e1") & external("e2")
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
                        RawExpr::Str(s) => match args.get(1) {
                            None => Some((*s, None)),
                            Some(RawExpr::Str(default)) => {
                                Some((*s, Some(*default)))
                            }
                            Some(RawExpr::Name(q)) => {
                                Some((*s, Some(Ustr::from(&format!("{}", q)))))
                            }
                            _ => panic!(
                                "Second arg to external must be static string"
                            ),
                        },
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
