/// A GPR file that hasn't been processed yet.  All we store here is the info we
/// extracted from the file itself, but we did not resolve paths, for instance.
/// Such an object is only valid as long as the scanner that generates it, since
/// it references memory from that scanner directly.

use crate::rawexpr::RawExpr;

#[derive(Debug, Default)]
pub struct ProjectDecl<'a> {
    pub name: &'a str,
    pub is_abstract: bool,
    pub is_aggregate: bool,
    pub is_library: bool,
    pub extends: Option<&'a str>,
}

#[derive(Debug)]
pub struct PackageDecl<'a> {
    name: String,
    renames: Option<String>,
    extends: Option<String>,
    body: Option<Box<Tree<'a>>>,
}

#[derive(Debug)]
pub struct AttributeDecl {
    name: String,
    index: Option<String>,
    value: RawExpr,
}

#[derive(Debug)]
pub struct VariableDecl {
    name: String,
    value: RawExpr,
}

#[derive(Debug)]
pub struct TypeDecl {
    name: String,
    valid: Vec<String>,
}

#[derive(Debug)]
pub struct CaseStmt<'a> {
    var: String,
    when: Vec<WhenClause<'a>>,
}

#[derive(Debug)]
pub struct WhenClause<'a> {
    values: Vec<String>,
    body: Box<Tree<'a>>,
}

#[derive(Debug)]
pub enum Tree<'a> {
    Unset,
    Project(ProjectDecl<'a>),
    Package(PackageDecl<'a>),
    Type(TypeDecl),
    Attribute(AttributeDecl),
    Variable(VariableDecl),
    Case(CaseStmt<'a>),
}

pub struct RawGPR<'a> {
    pub path: &'a std::path::Path,
    pub imported: Vec<&'a [u8]>,
    pub content: Tree<'a>,
}

impl<'a> RawGPR<'a> {
    /// Create a new, mostly unset, GPR file
    pub fn new(path: &'a std::path::Path) -> Self {
        Self {
            path,
            imported: vec![],
            content: Tree::Unset,
        }
    }

    /// Resolve relative paths
    pub fn normalize_path(&self, path: &'a [u8]) -> std::path::PathBuf {
        let relpath = std::str::from_utf8(path).unwrap();
        let mut p = self.path.parent().unwrap().join(relpath);
        p.set_extension("gpr");
        std::fs::canonicalize(p).unwrap()
    }
}


