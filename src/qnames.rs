use itertools::join;
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
pub struct QName(pub Vec<Ustr>);

impl QName {
    pub fn new(qname: Vec<Ustr>) -> Self {
        QName(qname)
    }
    pub fn from_slice(qname: &[Ustr]) -> Self {
        QName(qname.to_vec())
    }

    pub fn join(&mut self, child: QName) {
        self.0.extend(child.0);
    }

    pub fn parent(&self) -> Option<QName> {
        match self.0.len() {
            0 => None,
            s => Some(QName::from_slice(&self.0[0..s - 1])),
        }
    }
}

impl std::fmt::Display for QName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", join(self.0.iter(), "."))
    }
}
