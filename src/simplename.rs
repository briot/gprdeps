use crate::errors::Error;
/// An unqualified name, which could be either an attribute or variable
use ustr::Ustr;

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
}

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
    pub fn new_var(lower: Ustr) -> Self {
        SimpleName::Name(lower)
    }

    /// Builds an attribute name
    /// Properly detects whether an index was needed or not
    pub fn new_attr(
        lower: Ustr,
        index: Option<StringOrOthers>,
    ) -> Result<Self, Error> {
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
            (_, None) => Err(Error::InvalidAttribute(lower)),
            (_, Some(StringOrOthers::Str(idx))) => {
                Err(Error::InvalidAttributeWithIndex(lower, idx))
            }
            (_, Some(StringOrOthers::Others)) => {
                Err(Error::InvalidAttributeWithOthers(lower))
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
