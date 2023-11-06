/// The un-interpreted tree, as parsed from a GPR file
use crate::lexer::Lexer;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackageName {
    Binder,
    Builder,
    Compiler,
    IDE,
    Linker,
    Naming,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub package: Option<PackageName>,
    pub name: AttributeOrVarName,
    pub index: Option<Vec<RawExpr>>,
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
        valid: Vec<String>,
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
    Comma((Box<RawExpr>, Box<RawExpr>)), // argument lists
    List(Vec<Box<RawExpr>>),
}

impl RawExpr {
    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: RawExpr) -> RawExpr {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Build up an argument list
    pub fn comma(self, right: RawExpr) -> RawExpr {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Comma((Box::new(self), Box::new(right))),
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
    pub fn to_static_str(self, lex: &Lexer) -> crate::errors::Result<String> {
        match self {
            RawExpr::StaticString(s) => Ok(s),
            _ => Err(lex.error("not a static string".into())),
        }
    }

    /// Convert to a list of static strings
    pub fn to_static_list(
        self,
        lex: &Lexer,
    ) -> crate::errors::Result<Vec<String>> {
        match self {
            RawExpr::List(list) => Ok(list
                .into_iter()
                .map(|e| e.to_static_str(lex))
                .collect::<crate::errors::Result<Vec<String>>>()?),
            _ => Err(lex.error("not a list of static strings".into())),
        }
    }
}
