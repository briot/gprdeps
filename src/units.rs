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

pub type QualifiedName = Vec<Ustr>;

#[derive(Debug, Default)]
pub struct Unit {
    pub name: QualifiedName,

    // The list of dependencies as fully qualified names
    pub deps: Vec<QualifiedName>,
}
