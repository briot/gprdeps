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

#[derive(Debug, Default)]
pub struct QualifiedName(Vec<Ustr>);

impl QualifiedName {
    pub fn new(qname: Vec<Ustr>) -> Self {
        QualifiedName(qname)
    }

    pub fn join(&mut self, child: QualifiedName) {
        self.0.extend(child.0);
    }
}

#[derive(Debug, Default)]
pub struct Unit {
    pub name: QualifiedName,

    // The list of dependencies as fully qualified names
    pub deps: Vec<QualifiedName>,
}
