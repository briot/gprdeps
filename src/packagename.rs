// ??? Should implement std::ops::Index so that we can use it directly to
// index arrays.

use ustr::Ustr;
use crate::errors::Error;

lazy_static::lazy_static! {
    static ref BINDER:Ustr = Ustr::from("binder");
    static ref BUILDER:Ustr = Ustr::from("builder");
    static ref COMPILER:Ustr = Ustr::from("compiler");
    static ref IDE:Ustr = Ustr::from("ide");
    static ref LINKER:Ustr = Ustr::from("linker");
    static ref NAMING:Ustr = Ustr::from("naming");
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(usize)]
pub enum PackageName {
    None = 0,
    Binder,
    Builder,
    Compiler,
    Ide,
    Linker,
    Naming,
}

// In rust nightly, we can use std::mem::variant_count::<PackageName>()
pub const PACKAGE_NAME_VARIANTS: usize = 7;

impl PackageName {
    pub fn new(lower: Ustr) -> Result<Self, Error> {
        if lower == *BINDER {
            Ok(PackageName::Binder)
        } else if lower == *BUILDER {
            Ok(PackageName::Builder)
        } else if lower == *COMPILER {
            Ok(PackageName::Compiler)
        } else if lower == *IDE {
            Ok(PackageName::Ide)
        } else if lower == *LINKER {
            Ok(PackageName::Linker)
        } else if lower == *NAMING {
            Ok(PackageName::Naming)
        } else {
            Err(Error::InvalidPackageName(lower))
        }
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageName::None => write!(f, "<top>"),
            PackageName::Binder => write!(f, "binder"),
            PackageName::Builder => write!(f, "builder"),
            PackageName::Compiler => write!(f, "compiler"),
            PackageName::Ide => write!(f, "ide"),
            PackageName::Linker => write!(f, "linker"),
            PackageName::Naming => write!(f, "naming"),
        }
    }
}
