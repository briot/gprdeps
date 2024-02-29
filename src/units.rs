/// Source files are grouped into units.
/// Those units describe at which level the dependencies occur in each
/// particular language.
///
/// For Ada, one unit will include the spec (.ads), the body (.adb), and any
/// number of separates, since a `with` statement mentions a unit name.
///
/// For C, each file is its own unit, since a `#import` mentions a file path.
///
/// For Rust, each file it is own unit, the name of which is given by the
/// crate's fully qualified name "crate::errors::Error" for instance.
use ustr::Ustr;

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct QualifiedName(pub Vec<Ustr>);

impl QualifiedName {
    pub fn new(qname: Vec<Ustr>) -> Self {
        QualifiedName(qname)
    }
    pub fn from_slice(qname: &[Ustr]) -> Self {
        QualifiedName(qname.to_vec())
    }

    pub fn join(&mut self, child: QualifiedName) {
        self.0.extend(child.0);
    }

    pub fn parent(&self) -> Option<QualifiedName> {
        match self.0.len() {
            0 => None,
            s => Some(QualifiedName::from_slice(&self.0[0..s - 1])),
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|n| format!("{}", n))
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

#[derive(Debug)]
pub struct UnitSource {
    pub path: std::path::PathBuf,
    pub kind: SourceKind,
}

#[derive(Debug, Default)]
pub struct Unit {
    pub sources: Vec<UnitSource>,

    // The list of dependencies as fully qualified names
    pub deps: std::collections::HashSet<QualifiedName>,
}

/// What is the semantic of a source file within a unit.
/// In C, units are made up of a single file, so this is always the
/// implementation.
#[derive(Debug, Copy, Clone)]
pub enum SourceKind {
    Spec,
    Implementation,
    Separate,
}

/// The data structure returned when parsing one source file.
/// All SourceInfo with the same unitname will be merged.
#[derive(Debug)]
pub struct SourceInfo {
    pub unitname: QualifiedName,
    pub kind: SourceKind,
    pub deps: std::collections::HashSet<QualifiedName>,
}
