/// The un-interpreted tree, as parsed from a GPR file

use std::fmt::{Debug, Error, Formatter};
use crate::lexer::Lexer;

pub static PROJECT: &str = "project";

/// A fully qualified name    project.pkg.name'attname (index)
/// for instance:    Config.Compiler'Switches ("Ada")
#[derive(Debug, Default)]
pub struct VariableName<'a> {
    pub project: &'a str,
    pub package: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Default)]
pub struct AttributeName<'a> {
    pub project: &'a str,
    pub package: &'a str,
    pub name: &'a str,
    pub attname: Option<&'a str>,
    pub index: Option<Box<RawExpr<'a>>>,
}

#[derive(Debug, Default)]
pub struct PackageDecl<'a> {
    pub name: &'a str,
    pub renames: Option<VariableName<'a>>,
    pub extends: Option<VariableName<'a>>,
    pub body: Vec<Statement<'a>>,
}

#[derive(Debug, Default)]
pub struct AttributeDecl<'a> {
    pub name: &'a str,
    pub index: Option<&'a str>,
    pub value: RawExpr<'a>,
}

#[derive(Debug, Default)]
pub struct VariableDecl<'a> {
    pub name: &'a str,
    pub typename: Option<VariableName<'a>>,
    pub expr: RawExpr<'a>,
}

#[derive(Debug, Default)]
pub struct TypeDecl<'a> {
    pub typename: &'a str,
    pub valid: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CaseStmt<'a> {
    pub varname: VariableName<'a>,
    pub when: Vec<WhenClause<'a>>,
}

#[derive(Debug, Default)]
pub struct WhenClause<'a> {
    pub values: Vec<Option<&'a str>>,  // None is used for "others"
    pub body: Vec<Statement<'a>>,
}

#[derive(Debug)]
pub enum Statement<'a> {
    Package(PackageDecl<'a>),
    Type(TypeDecl<'a>),
    Attribute(AttributeDecl<'a>),
    Variable(VariableDecl<'a>),
    Case(CaseStmt<'a>),
}

#[derive(Debug)]
pub struct FunctionCall<'a> {
    pub funcname: &'a str,
    pub args: Vec<RawExpr<'a>>,
}

#[derive(Default)]
pub enum RawExpr<'a> {
    #[default]
    Empty,
    StaticString(&'a str), //  doesn't include surrounding quotes
    Identifier(AttributeName<'a>),   //  Could be "prj.pkg'attribute"
    Ampersand((Box<RawExpr<'a>>, Box<RawExpr<'a>>)),
    Comma((Box<RawExpr<'a>>, Box<RawExpr<'a>>)),  // argument lists
    List(Vec<Box<RawExpr<'a>>>),
    FuncCall(FunctionCall<'a>),
}

impl<'a> RawExpr<'a> {
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
    pub fn to_static_str(
        &self,
        lex: &Lexer,
    ) -> crate::errors::Result<String> {
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
            RawExpr::List(list) =>
                Ok(list.iter()
                    .map(|e| e.to_static_str(lex))
                    .collect::<crate::errors::Result<Vec<String>>>()?
                ),
            _ => Err(lex.error("not a list of static strings".into())),
        }
    }
}

impl<'a> Debug for RawExpr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            RawExpr::Empty => write!(f, "<empty>"),
            RawExpr::StaticString(s) => write!(f, "'{}'", s),
            RawExpr::Identifier(s) => write!(f, "{:?}", s),
            RawExpr::Ampersand((left, right)) =>
                write!(f, "{:?} & {:?}", left, right),
            RawExpr::Comma((left, right)) =>
                write!(f, "{:?}, {:?}", left, right),
            RawExpr::FuncCall(c) =>
                write!(f, "{:?} ({:?})", c.funcname, c.args),
            RawExpr::List(v) => write!(
                f,
                "({})",
                v.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}
