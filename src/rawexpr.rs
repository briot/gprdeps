/// The un-interpreted tree, as parsed from a GPR file
use crate::lexer::Lexer;
use std::collections::HashSet;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackageName {
    None = 0,
    Binder,
    Builder,
    Compiler,
    IDE,
    Linker,
    Naming,
}

// In rust nightly, we can use std::mem::variant_count::<PackageName>()
pub const PACKAGE_NAME_VARIANTS: usize = 7;

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageName::None => write!(f, "<top>"),
            PackageName::Binder => write!(f, "binder"),
            PackageName::Builder => write!(f, "builder"),
            PackageName::Compiler => write!(f, "compiler"),
            PackageName::IDE => write!(f, "ide"),
            PackageName::Linker => write!(f, "linker"),
            PackageName::Naming => write!(f, "naming"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttributeOrVarName {
    Name(String), // Either variable or attribute name, lower-cased
    ExecDir,
    LinkerOptions,
    Main,
    ObjectDir,
    SourceDirs,
    SourceFiles,
    Switches,
}
impl std::fmt::Display for AttributeOrVarName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeOrVarName::Name(s) => write!(f, "{}", s),
            AttributeOrVarName::ExecDir => write!(f, "exec_dir"),
            AttributeOrVarName::LinkerOptions => write!(f, "linker_options"),
            AttributeOrVarName::Main => write!(f, "main"),
            AttributeOrVarName::ObjectDir => write!(f, "object_dir"),
            AttributeOrVarName::SourceDirs => write!(f, "source_dirs"),
            AttributeOrVarName::SourceFiles => write!(f, "source_files"),
            AttributeOrVarName::Switches => write!(f, "switches"),
        }
    }
}

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
    pub project: Option<String>, // None for current project or "Project'"
    pub package: PackageName,
    pub name: AttributeOrVarName,
    pub index: Option<Vec<RawExpr>>,
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = &self.project {
            write!(f, "{}.", p)?;
        }
        write!(f, "{}.{}", self.package, self.name)?;
        if self.index.is_some() {
            write!(f, "(..)")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum StringOrOthers {
    Str(String),
    Others,
}

#[derive(Debug, PartialEq)]
pub struct WhenClause {
    pub values: Vec<StringOrOthers>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Package {
        name: PackageName,
        renames: Option<QualifiedName>,
        extends: Option<QualifiedName>,
        body: Vec<Statement>,
    },
    TypeDecl {
        typename: String,
        valid: RawExpr,
    },
    AttributeDecl {
        name: AttributeOrVarName,
        index: Option<StringOrOthers>,
        value: RawExpr,
    },
    VariableDecl {
        name: String,
        typename: Option<QualifiedName>,
        expr: RawExpr,
    },
    Case {
        varname: QualifiedName,
        when: Vec<WhenClause>,
    },
}

#[derive(Debug, PartialEq)]
pub enum RawExpr {
    Empty,
    Others,
    StaticString(String), //  doesn't include surrounding quotes
    Name(QualifiedName),
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    List(Vec<Box<RawExpr>>),
}

impl RawExpr {
    /// Whether the expression contains a call to external().
    /// Returns the name of the scenario variable
    pub fn has_external(&self) -> Option<&String> {
        match self {
            RawExpr::Empty | RawExpr::Others | RawExpr::StaticString(_) => None,
            RawExpr::Ampersand((left, right)) => {
                left.has_external().or_else(|| right.has_external())
            }
            RawExpr::List(v) => v.iter().find_map(|e| e.has_external()),
            RawExpr::Name(QualifiedName {
                project: None,
                package: PackageName::None,
                name: n,
                index: Some(idx),
            }) => match n {
                AttributeOrVarName::Name(n2) if n2 == "external" => {
                    match &idx[0] {
                        RawExpr::StaticString(s) => Some(s),
                        _ => panic!(
                            "First argument to external must \
                                 be static string"
                        ),
                    }
                }
                _ => None,
            },
            RawExpr::Name(_) => None,
        }
    }

    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: RawExpr) -> RawExpr {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Append an element to a list
    pub fn append(&mut self, right: RawExpr) {
        match self {
            RawExpr::List(list) => list.push(Box::new(right)),
            _ => panic!("Can only append to a list expression"),
        }
    }

    /// Convert to a static string
    /// ??? Should use values.rs
    pub fn to_static_str(self, lex: &Lexer) -> crate::errors::Result<String> {
        match self {
            RawExpr::StaticString(s) => Ok(s),
            _ => Err(lex.error("not a static string".into())),
        }
    }

    /// Convert to a list of static strings
    /// ??? Should use values.rs
    pub fn to_static_set(
        self,
        lex: &Lexer,
    ) -> crate::errors::Result<HashSet<String>> {
        match self {
            RawExpr::List(list) => Ok(list
                .into_iter()
                .map(|e| e.to_static_str(lex))
                .collect::<crate::errors::Result<HashSet<String>>>()?),
            _ => Err(lex.error("not a list of static strings".into())),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::rawexpr::RawExpr;

    pub fn build_expr_str(s: &str) -> RawExpr {
        RawExpr::StaticString(s.to_string())
    }

    pub fn build_expr_list(s: &[&str]) -> RawExpr {
        let v = s
            .iter()
            .map(|st| Box::new(build_expr_str(st)))
            .collect::<Vec<_>>();
        RawExpr::List(v)
    }
}
