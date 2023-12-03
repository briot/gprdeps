/// The un-interpreted tree, as parsed from a GPR file
use std::fmt::Debug;
use ustr::Ustr;

lazy_static::lazy_static! {
    static ref BODY_SUFFIX: Ustr = Ustr::from("body_suffix");
    static ref BODY: Ustr = Ustr::from("body");
    static ref DEFAULT_SWITCHES: Ustr = Ustr::from("default_switches");
    static ref DOT_REPLACEMENT: Ustr = Ustr::from("dot_replacement");
    static ref EXCLUDED_SOURCE_FILES: Ustr =
        Ustr::from("excluded_source_files");
    static ref EXEC_DIR: Ustr = Ustr::from("exec_dir");
    static ref EXECUTABLE: Ustr = Ustr::from("executable");
    static ref EXTERNALLY_BUILT: Ustr = Ustr::from("externally_built");
    static ref GLOBAL_CONFIGURATION_PRAGMAS: Ustr =
        Ustr::from("global_configuration_pragmas");
    static ref LANGUAGES: Ustr = Ustr::from("languages");
    static ref LIBRARY_DIR: Ustr = Ustr::from("library_dir");
    static ref LIBRARY_INTERFACE: Ustr = Ustr::from("library_interface");
    static ref LIBRARY_KIND: Ustr = Ustr::from("library_kind");
    static ref LIBRARY_NAME: Ustr = Ustr::from("library_name");
    static ref LIBRARY_OPTIONS: Ustr = Ustr::from("library_options");
    static ref LIBRARY_STANDALONE: Ustr = Ustr::from("library_standalone");
    static ref LIBRARY_VERSION: Ustr = Ustr::from("library_version");
    static ref LINKER_OPTIONS: Ustr = Ustr::from("linker_options");
    static ref LOCAL_CONFIGURATION_PRAGMAS: Ustr =
        Ustr::from("local_configuration_pragmas");
    static ref MAIN: Ustr = Ustr::from("main");
    static ref OBJECT_DIR: Ustr = Ustr::from("object_dir");
    static ref PROJECT_FILES: Ustr = Ustr::from("project_files");
    static ref SHARED_LIBRARY_PREFIX: Ustr =
        Ustr::from("shared_library_prefix");
    static ref SOURCE_DIRS: Ustr = Ustr::from("source_dirs");
    static ref SOURCE_FILES: Ustr = Ustr::from("source_files");
    static ref SOURCE_LIST_FILE: Ustr = Ustr::from("source_list_file");
    static ref SPEC: Ustr = Ustr::from("spec");
    static ref SPEC_SUFFIX: Ustr = Ustr::from("spec_suffix");
    static ref SWITCHES: Ustr = Ustr::from("switches");
    static ref TARGET: Ustr = Ustr::from("target");
    static ref VCS_KIND: Ustr = Ustr::from("vcs_kind");
    static ref VCS_REPOSITORY_ROOT: Ustr =
        Ustr::from("vcs_repository_root");

    static ref BINDER:Ustr = Ustr::from("binder");
    static ref BUILDER:Ustr = Ustr::from("builder");
    static ref COMPILER:Ustr = Ustr::from("compiler");
    static ref IDE:Ustr = Ustr::from("ide");
    static ref LINKER:Ustr = Ustr::from("linker");
    static ref NAMING:Ustr = Ustr::from("naming");
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub fn new(lower: Ustr) -> Result<Self, String> {
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
            Err(format!("Invalid package name {}", lower))
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
        match (lower, index) {
            (a, Some(StringOrOthers::Str(idx))) if a == *BODY_SUFFIX => {
                Ok(SimpleName::BodySuffix(idx))
            }
            (a, Some(StringOrOthers::Str(idx))) if a == *BODY => {
                Ok(SimpleName::Body(idx))
            }
            (a, Some(idx)) if a == *DEFAULT_SWITCHES => {
                Ok(SimpleName::DefaultSwitches(idx))
            }
            (a, None) if a == *DOT_REPLACEMENT => {
                Ok(SimpleName::DotReplacement)
            }
            (a, None) if a == *EXCLUDED_SOURCE_FILES => {
                Ok(SimpleName::ExcludedSourceFiles)
            }
            (a, None) if a == *EXEC_DIR => Ok(SimpleName::ExecDir),
            (a, Some(StringOrOthers::Str(idx))) if a == *EXECUTABLE => {
                Ok(SimpleName::Executable(idx))
            }
            (a, None) if a == *EXTERNALLY_BUILT => {
                Ok(SimpleName::ExternallyBuilt)
            }
            (a, None) if a == *GLOBAL_CONFIGURATION_PRAGMAS => {
                Ok(SimpleName::GlobalConfigurationPragmas)
            }
            (a, None) if a == *LANGUAGES => Ok(SimpleName::Languages),
            (a, None) if a == *LIBRARY_DIR => Ok(SimpleName::LibraryDir),
            (a, None) if a == *LIBRARY_INTERFACE => {
                Ok(SimpleName::LibraryInterface)
            }
            (a, None) if a == *LIBRARY_KIND => Ok(SimpleName::LibraryKind),
            (a, None) if a == *LIBRARY_NAME => Ok(SimpleName::LibraryName),
            (a, None) if a == *LIBRARY_OPTIONS => {
                Ok(SimpleName::LibraryOptions)
            }
            (a, None) if a == *LIBRARY_STANDALONE => {
                Ok(SimpleName::LibraryStandalone)
            }
            (a, None) if a == *LIBRARY_VERSION => {
                Ok(SimpleName::LibraryVersion)
            }
            (a, None) if a == *LINKER_OPTIONS => Ok(SimpleName::LinkerOptions),
            (a, None) if a == *LOCAL_CONFIGURATION_PRAGMAS => {
                Ok(SimpleName::LocalConfigurationPragmas)
            }
            (a, None) if a == *MAIN => Ok(SimpleName::Main),
            (a, None) if a == *OBJECT_DIR => Ok(SimpleName::ObjectDir),
            (a, None) if a == *PROJECT_FILES => Ok(SimpleName::ProjectFiles),
            (a, None) if a == *SHARED_LIBRARY_PREFIX => {
                Ok(SimpleName::SharedLibraryPrefix)
            }
            (a, None) if a == *SOURCE_DIRS => Ok(SimpleName::SourceDirs),
            (a, None) if a == *SOURCE_FILES => Ok(SimpleName::SourceFiles),
            (a, None) if a == *SOURCE_LIST_FILE => {
                Ok(SimpleName::SourceListFile)
            }
            (a, Some(StringOrOthers::Str(idx))) if a == *SPEC => {
                Ok(SimpleName::Spec(idx))
            }
            (a, Some(StringOrOthers::Str(idx))) if a == *SPEC_SUFFIX => {
                Ok(SimpleName::SpecSuffix(idx))
            }
            (a, Some(idx)) if a == *SWITCHES => Ok(SimpleName::Switches(idx)),
            (a, None) if a == *TARGET => Ok(SimpleName::Target),
            (a, None) if a == *VCS_KIND => Ok(SimpleName::VCSKind),
            (a, None) if a == *VCS_REPOSITORY_ROOT => {
                Ok(SimpleName::VCSRepositoryRoot)
            }
            (_, None) => Err(format!("Invalid attribute name {}", lower)),
            (_, Some(idx)) => {
                Err(format!("Invalid attribute name {}({})", lower, idx))
            }
        }
    }

    /// Whether this attribute uses a case-insensitive index (first element of
    /// tuple) or case-insensitive value (second element) ?
    pub fn is_case_insensitive(lower: &Ustr) -> (bool, bool) {
        if *lower == *LANGUAGES {
            (false, true) // No index, case-insensitive value
        } else if *lower == *BODY
            || *lower == *SPEC
            || *lower == *BODY_SUFFIX
            || *lower == *SPEC_SUFFIX
            || *lower == *SWITCHES
            || *lower == *DEFAULT_SWITCHES
        {
            (true, false) // case-insensitive index, case-sensitive value
        } else {
            (false, false)
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
    Str(Ustr), //  doesn't include surrounding quotes
    Name(QualifiedName),
    FuncCall((QualifiedName, Vec<RawExpr>)),
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    List(Vec<RawExpr>),
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
                        RawExpr::Str(s) => Some(*s),
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
    pub fn into_static_str(self) -> Result<Ustr, String> {
        match self {
            RawExpr::Str(s) => Ok(s),
            _ => Err("not a static string".into()),
        }
    }

    /// Convert a list of static strings to lower case
    pub fn to_lowercase(&self) -> RawExpr {
        match &self {
            RawExpr::Str(s) => {
                RawExpr::Str(Ustr::from(&s.as_str().to_lowercase()))
            }
            RawExpr::List(s) => {
                RawExpr::List(s.iter().map(|e| e.to_lowercase()).collect())
            }
            _ => panic!("Can only convert static list to lower-case"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::rawexpr::RawExpr;
    use ustr::Ustr;

    pub fn build_expr_str(s: &str) -> RawExpr {
        RawExpr::Str(Ustr::from(s))
    }

    pub fn build_expr_list(s: &[&str]) -> RawExpr {
        let v = s.iter().map(|st| build_expr_str(st)).collect::<Vec<_>>();
        RawExpr::List(v)
    }
}
