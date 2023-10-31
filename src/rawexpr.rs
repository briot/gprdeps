use crate::lexer::Lexer;
/// The un-interpreted tree, as parsed from a GPR file
use std::fmt::{Debug, Error, Formatter};

pub static PROJECT: &str = "project";

/// A fully qualified name    project.pkg.name'attname (index)
/// for instance:    Config.Compiler'Switches ("Ada")
#[derive(Debug, Default)]
pub struct VariableName {
    pub project: Option<String>,
    pub package: Option<String>,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct AttributeName {
    pub project: Option<String>,
    pub package: Option<String>,
    pub name: String,
    pub attname: Option<String>,
    pub index: Option<Box<RawExpr>>,
}

#[derive(Debug, Default)]
pub struct PackageDecl {
    pub name: String,
    pub renames: Option<VariableName>,
    pub extends: Option<VariableName>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Default)]
pub enum StringOrOthers {
    Str(String),
    #[default]
    Others,
}

#[derive(Debug, Default)]
pub struct AttributeDecl {
    pub name: String,
    pub index: Option<StringOrOthers>,
    pub value: RawExpr,
}

#[derive(Debug, Default)]
pub struct VariableDecl {
    pub name: String,
    pub typename: Option<VariableName>,
    pub expr: RawExpr,
}

#[derive(Debug, Default)]
pub struct TypeDecl {
    pub typename: String,
    pub valid: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CaseStmt {
    pub varname: VariableName,
    pub when: Vec<WhenClause>,
}

#[derive(Debug, Default)]
pub struct WhenClause {
    pub values: Vec<StringOrOthers>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Package(PackageDecl),
    Type(TypeDecl),
    Attribute(AttributeDecl),
    Variable(VariableDecl),
    Case(CaseStmt),
}

#[derive(Debug)]
pub struct FunctionCall {
    pub funcname: String,
    pub args: Vec<RawExpr>,
}

#[derive(Debug, Default)]
pub enum RawExpr {
    #[default]
    Empty,
    Others,
    StaticString(String), //  doesn't include surrounding quotes
    AttributeOrFunc(AttributeName),
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    Comma((Box<RawExpr>, Box<RawExpr>)), // argument lists
    List(Vec<Box<RawExpr>>),
}

impl RawExpr {
    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: Self) -> Self {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Build up an argument list
    pub fn comma(self, right: Self) -> Self {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Comma((Box::new(self), Box::new(right))),
        }
    }

    /// Append an element to a list
    pub fn append(&mut self, right: Self) {
        match self {
            RawExpr::List(list) => list.push(Box::new(right)),
            _ => panic!("Can only append to a list expression"),
        }
    }

    /// Convert to a static string
    pub fn to_static_str(&self, lex: &Lexer) -> crate::errors::Result<String> {
        match self {
            RawExpr::StaticString(s) => Ok(s.to_string()),
            _ => Err(lex.error("not a static string".into())),
        }
    }

    /// Convert to a list of static strings
    pub fn to_static_list(
        &self,
        lex: &Lexer,
    ) -> crate::errors::Result<Vec<String>> {
        match self {
            RawExpr::List(list) => Ok(list
                .iter()
                .map(|e| e.to_static_str(lex))
                .collect::<crate::errors::Result<Vec<String>>>()?),
            _ => Err(lex.error("not a list of static strings".into())),
        }
    }
}

// impl Debug for RawExpr {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
//         match self {
//             RawExpr::Empty => write!(f, "<empty>"),
//             RawExpr::StaticString(s) => write!(f, "'{}'", s),
//             RawExpr::AttributeOrFunc(s) => write!(f, "{:?}", s),
//             RawExpr::Ampersand((left, right)) => {
//                 write!(f, "{:?} & {:?}", left, right)
//             }
//             RawExpr::Comma((left, right)) => {
//                 write!(f, "{:?}, {:?}", left, right)
//             }
//             RawExpr::List(v) => write!(
//                 f,
//                 "({})",
//                 v.iter()
//                     .map(|e| format!("{:?}", e))
//                     .collect::<Vec<String>>()
//                     .join(", ")
//             ),
//         }
//     }
// }
