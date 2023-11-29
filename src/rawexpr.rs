/// The un-interpreted tree, as parsed from a GPR file
use std::fmt::Debug;
use ustr::Ustr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackageName {
    None = 0,
    Binder,
    Builder,
    Compiler,
    IDE,
    Linker,
    Naming,
}

// In rust nightly, we can use std::mem::variant_count::<PackageName>()
pub const PACKAGE_NAME_VARIANTS: usize = 7;

impl PackageName {
    pub fn new(lower: Ustr) -> Result<Self, String> {
        match lower.as_str() {
            "binder" => Ok(PackageName::Binder),
            "builder" => Ok(PackageName::Builder),
            "compiler" => Ok(PackageName::Compiler),
            "ide" => Ok(PackageName::IDE),
            "linker" => Ok(PackageName::Linker),
            "naming" => Ok(PackageName::Naming),
            _ => Err(format!("Invalid package name {}", lower)),
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
            PackageName::IDE => write!(f, "ide"),
            PackageName::Linker => write!(f, "linker"),
            PackageName::Naming => write!(f, "naming"),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StringOrOthers {
    Str(Ustr),
    Others,
}
impl std::fmt::Display for StringOrOthers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringOrOthers::Others => write!(f, "others"),
            StringOrOthers::Str(s) => write!(f, "{}", s),
        }
    }
}

/// An unqualified name, which could be either an attribute or variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SimpleName {
    Name(Ustr),       // Either variable or attribute name, lower-cased
    BodySuffix(Ustr), // indexed on lower-cased language
    Body(Ustr),       // indexed on file basename, casing preserved
    DefaultSwitches(StringOrOthers), // indexed on lower-cased language
    DotReplacement,
    ExcludedSourceFiles,
    ExecDir,
    Executable(Ustr), // indexed on file basename, casing preserved
    ExternallyBuilt,  // "true" or "false", lower-cased
    GlobalConfigurationPragmas,
    Languages, // lower-cased
    LibraryDir,
    LibraryInterface,
    LibraryKind,
    LibraryName,
    LibraryOptions,
    LibraryStandalone,
    LibraryVersion,
    LinkerOptions,
    LocalConfigurationPragmas,
    Main,
    ObjectDir,
    ProjectFiles,
    SharedLibraryPrefix,
    SourceDirs,
    SourceFiles,
    Spec(Ustr),       // indexed on file basename, casing preserved
    SpecSuffix(Ustr), // indexed on lower-cased language
    SourceListFile,
    Switches(StringOrOthers), // indexed on lower-cased language
    Target,
    VCSKind,
    VCSRepositoryRoot,
}
impl SimpleName {
    /// Builds a variable name
    pub fn new_var(lower: Ustr) -> Result<Self, String> {
        Ok(SimpleName::Name(lower))
    }

    /// Builds an attribute name
    /// Properly detects whether an index was needed or not
    pub fn new_attr(
        lower: Ustr,
        index: Option<StringOrOthers>,
    ) -> Result<Self, String> {
        match (lower.as_str(), index) {
            ("body_suffix", Some(StringOrOthers::Str(idx))) => {
                Ok(SimpleName::BodySuffix(idx))
            }
            ("body", Some(StringOrOthers::Str(idx))) => {
                Ok(SimpleName::Body(idx))
            }
            ("default_switches", Some(idx)) => {
                Ok(SimpleName::DefaultSwitches(idx))
            }
            ("dot_replacement", None) => Ok(SimpleName::DotReplacement),
            ("excluded_source_files", None) => {
                Ok(SimpleName::ExcludedSourceFiles)
            }
            ("exec_dir", None) => Ok(SimpleName::ExecDir),
            ("executable", Some(StringOrOthers::Str(idx))) => {
                Ok(SimpleName::Executable(idx))
            }
            ("externally_built", None) => Ok(SimpleName::ExternallyBuilt),
            ("global_configuration_pragmas", None) => {
                Ok(SimpleName::GlobalConfigurationPragmas)
            }
            ("languages", None) => Ok(SimpleName::Languages),
            ("library_dir", None) => Ok(SimpleName::LibraryDir),
            ("library_interface", None) => Ok(SimpleName::LibraryInterface),
            ("library_kind", None) => Ok(SimpleName::LibraryKind),
            ("library_name", None) => Ok(SimpleName::LibraryName),
            ("library_options", None) => Ok(SimpleName::LibraryOptions),
            ("library_standalone", None) => Ok(SimpleName::LibraryStandalone),
            ("library_version", None) => Ok(SimpleName::LibraryVersion),
            ("linker_options", None) => Ok(SimpleName::LinkerOptions),
            ("local_configuration_pragmas", None) => {
                Ok(SimpleName::LocalConfigurationPragmas)
            }
            ("main", None) => Ok(SimpleName::Main),
            ("object_dir", None) => Ok(SimpleName::ObjectDir),
            ("project_files", None) => Ok(SimpleName::ProjectFiles),
            ("shared_library_prefix", None) => {
                Ok(SimpleName::SharedLibraryPrefix)
            }
            ("source_dirs", None) => Ok(SimpleName::SourceDirs),
            ("source_files", None) => Ok(SimpleName::SourceFiles),
            ("source_list_file", None) => Ok(SimpleName::SourceListFile),
            ("spec", Some(StringOrOthers::Str(idx))) => {
                Ok(SimpleName::Spec(idx))
            }
            ("spec_suffix", Some(StringOrOthers::Str(idx))) => {
                Ok(SimpleName::SpecSuffix(idx))
            }
            ("switches", Some(idx)) => Ok(SimpleName::Switches(idx)),
            ("target", None) => Ok(SimpleName::Target),
            ("vcs_kind", None) => Ok(SimpleName::VCSKind),
            ("vcs_repository_root", None) => Ok(SimpleName::VCSRepositoryRoot),
            (_, None) => Err(format!("Invalid attribute name {}", lower)),
            (_, Some(idx)) => {
                Err(format!("Invalid attribute name {}({})", lower, idx))
            }
        }
    }
}

impl std::fmt::Display for SimpleName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleName::Name(s) => write!(f, ".{}", s),
            SimpleName::BodySuffix(idx) => write!(f, "'body_suffix({})", idx),
            SimpleName::Body(idx) => write!(f, "'body({})", idx),
            SimpleName::DotReplacement => write!(f, "'dot_replacement"),
            SimpleName::DefaultSwitches(idx) => {
                write!(f, "'default_switches({})", idx)
            }
            SimpleName::ExcludedSourceFiles => {
                write!(f, "'excluded_source_files")
            }
            SimpleName::ExecDir => write!(f, "'exec_dir"),
            SimpleName::Executable(idx) => write!(f, "'executable({})", idx),
            SimpleName::ExternallyBuilt => write!(f, "'externally_build"),
            SimpleName::GlobalConfigurationPragmas => {
                write!(f, "'global_configuration_pragmas")
            }
            SimpleName::Languages => write!(f, "'languages"),
            SimpleName::LibraryDir => write!(f, "'library_dir"),
            SimpleName::LibraryInterface => write!(f, "'library_interface"),
            SimpleName::LibraryKind => write!(f, "'library_kind"),
            SimpleName::LibraryName => write!(f, "'library_name"),
            SimpleName::LibraryOptions => write!(f, "'library_options"),
            SimpleName::LibraryStandalone => write!(f, "'library_standalone"),
            SimpleName::LibraryVersion => write!(f, "'library_version"),
            SimpleName::LinkerOptions => write!(f, "'linker_options"),
            SimpleName::LocalConfigurationPragmas => {
                write!(f, "'local_configuration_pragmas")
            }
            SimpleName::Main => write!(f, "'main"),
            SimpleName::ObjectDir => write!(f, "'object_dir"),
            SimpleName::ProjectFiles => write!(f, "'project_files"),
            SimpleName::SharedLibraryPrefix => {
                write!(f, "'shared_library_prefix")
            }
            SimpleName::SourceDirs => write!(f, "'source_dirs"),
            SimpleName::SourceFiles => write!(f, "'source_files"),
            SimpleName::Spec(idx) => write!(f, "'spec({})", idx),
            SimpleName::SpecSuffix(idx) => write!(f, "'spec_suffix({})", idx),
            SimpleName::SourceListFile => write!(f, "'source_list_file"),
            SimpleName::Switches(idx) => write!(f, "'switches({})", idx),
            SimpleName::Target => write!(f, "'target"),
            SimpleName::VCSKind => write!(f, "'vcs_kind"),
            SimpleName::VCSRepositoryRoot => write!(f, "'vcs_repository_root"),
        }
    }
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
    pub project: Option<Ustr>, // None for current project or "Project'"
    pub package: PackageName,
    pub name: SimpleName,
}

impl QualifiedName {
    /// When we find a name in the source which an optional leading identifier,
    /// the latter could be either a project or a package.  This function will
    /// guess as needed.
    pub fn from_two(prj_or_pkg: Option<Ustr>, name: SimpleName) -> Self {
        match prj_or_pkg {
            None => QualifiedName {
                project: prj_or_pkg,
                package: PackageName::None,
                name,
            },
            Some(n1) => match PackageName::new(n1) {
                Ok(p) => QualifiedName {
                    project: None,
                    package: p,
                    name,
                },
                Err(_) => QualifiedName {
                    project: Some(n1),
                    package: PackageName::None,
                    name,
                },
            },
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = &self.project {
            write!(f, "{}.", p)?;
        }
        write!(f, "{}{}", self.package, self.name)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct WhenClause {
    pub values: Vec<StringOrOthers>,
    pub body: StatementList,
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Package {
        name: PackageName,
        renames: Option<QualifiedName>,
        extends: Option<QualifiedName>,
        body: StatementList,
    },
    TypeDecl {
        typename: Ustr,
        valid: RawExpr,
    },
    AttributeDecl {
        name: SimpleName,
        value: RawExpr,
    },
    VariableDecl {
        name: Ustr,
        typename: Option<QualifiedName>,
        expr: RawExpr,
    },
    Case {
        varname: QualifiedName,
        when: Vec<WhenClause>,
    },
}

/// Line + Statement
pub type StatementList = Vec<(u32, Statement)>;

#[derive(Debug, PartialEq)]
pub enum RawExpr {
    Empty,
    Others,
    StaticString(Ustr), //  doesn't include surrounding quotes
    Name(QualifiedName),
    FuncCall((QualifiedName, Vec<RawExpr>)),
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    List(Vec<Box<RawExpr>>),
}

lazy_static::lazy_static! {
    static ref EXTERNAL: Ustr = Ustr::from("external");
}

impl RawExpr {
    /// Whether the expression contains a call to external().
    /// Returns the name of the scenario variable
    pub fn has_external(&self) -> Option<Ustr> {
        match self {
            RawExpr::Ampersand((left, right)) => {
                left.has_external().or_else(|| right.has_external())
            }
            RawExpr::List(v) => v.iter().find_map(|e| e.has_external()),
            RawExpr::FuncCall((
                QualifiedName {
                    project: None,
                    package: PackageName::None,
                    name: SimpleName::Name(n),
                },
                args,
            )) => {
                if *n == *EXTERNAL {
                    match &args[0] {
                        RawExpr::StaticString(s) => Some(*s),
                        _ => panic!(
                            "First argument to external must \
                                 be static string"
                        ),
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: RawExpr) -> RawExpr {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Convert to a static string
    /// ??? Should use values.rs
    pub fn as_static_str(self) -> Result<Ustr, String> {
        match self {
            RawExpr::StaticString(s) => Ok(s),
            _ => Err("not a static string".into()),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::rawexpr::RawExpr;
    use ustr::Ustr;

    pub fn build_expr_str(s: &str) -> RawExpr {
        RawExpr::StaticString(Ustr::from(s))
    }

    pub fn build_expr_list(s: &[&str]) -> RawExpr {
        let v = s
            .iter()
            .map(|st| Box::new(build_expr_str(st)))
            .collect::<Vec<_>>();
        RawExpr::List(v)
    }
}
