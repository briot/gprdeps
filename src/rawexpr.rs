use crate::lexer::Lexer;
/// The un-interpreted tree, as parsed from a GPR file
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PackageName {
    #[default]
    Binder,
    Compiler,
    Linker,
}

/// A fully qualified name    project.pkg.name'attname (index)
/// for instance:    Config.Compiler'Switches ("Ada")
#[derive(Debug, Default)]
pub struct VariableName<'a> {
    pub project: Option<&'a str>,   // None for current project
    pub package: Option<PackageName>,
    pub name: &'a str,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum AttributeName {
    #[default]
    Unknown,
    ExecDir,
    LinkerOptions,
    Main,
    ObjectDir,
    SourceDirs,
    SourceFiles,
    Switches,
}

#[derive(Debug, Default)]
pub struct AttributeRef<'a> {
    pub project: Option<&'a str>,   // None for current project
    pub package: Option<PackageName>,
    pub attname: AttributeName,
    pub index: Option<Vec<RawExpr<'a>>>,
}

#[derive(Debug, Default)]
pub struct PackageDecl<'a> {
    pub name: PackageName,
    pub renames: Option<VariableName<'a>>,
    pub extends: Option<VariableName<'a>>,
    pub body: Vec<Statement<'a>>,
}

#[derive(Debug, Default)]
pub enum StringOrOthers<'a> {
    Str(&'a str),
    #[default]
    Others,
}

#[derive(Debug, Default)]
pub struct AttributeDecl<'a> {
    pub name: &'a str,
    pub index: Option<StringOrOthers<'a>>,
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
    pub valid: Vec<&'a str>,
}

#[derive(Debug, Default)]
pub struct CaseStmt<'a> {
    pub varname: VariableName<'a>,
    pub when: Vec<WhenClause<'a>>,
}

#[derive(Debug, Default)]
pub struct WhenClause<'a> {
    pub values: Vec<StringOrOthers<'a>>,
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

#[derive(Debug, Default)]
pub enum RawExpr<'a> {
    #[default]
    Empty,
    Others,
    StaticString(&'a str), //  doesn't include surrounding quotes
    Attribute(AttributeRef<'a>),
    Var(VariableName<'a>),
    Func(FunctionCall<'a>),
    Ampersand((Box<RawExpr<'a>>, Box<RawExpr<'a>>)),
    Comma((Box<RawExpr<'a>>, Box<RawExpr<'a>>)), // argument lists
    List(Vec<Box<RawExpr<'a>>>),
}

impl<'a> RawExpr<'a> {
    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: RawExpr<'a>) -> RawExpr<'a> {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Build up an argument list
    pub fn comma(self, right: RawExpr<'a>) -> RawExpr<'a> {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Comma((Box::new(self), Box::new(right))),
        }
    }

    /// Append an element to a list
    pub fn append(&mut self, right: RawExpr<'a>) {
        match self {
            RawExpr::List(list) => list.push(Box::new(right)),
            _ => panic!("Can only append to a list expression"),
        }
    }

    /// Convert to a static string
    pub fn to_static_str(&self, lex: &Lexer) -> crate::errors::Result<&'a str> {
        match self {
            RawExpr::StaticString(s) => Ok(s),
            _ => Err(lex.error("not a static string".into())),
        }
    }

    /// Convert to a list of static strings
    pub fn to_static_list(
        &self,
        lex: &Lexer,
    ) -> crate::errors::Result<Vec<&'a str>> {
        match self {
            RawExpr::List(list) => Ok(list
                .iter()
                .map(|e| e.to_static_str(lex))
                .collect::<crate::errors::Result<Vec<&'a str>>>()?),
            _ => Err(lex.error("not a list of static strings".into())),
        }
    }
}
