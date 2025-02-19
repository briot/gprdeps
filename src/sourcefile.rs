use crate::{
    ada_lexer::{AdaLexer, AdaLexerOptions},
    ada_scanner::AdaScanner,
    cpp_lexer::CppLexer,
    cpp_scanner::CppScanner,
    errors::Error,
    files::File,
    graph::NodeIndex,
    qnames::QName,
};
use std::path::{Path, PathBuf};
use ustr::Ustr;

/// What is the semantic of a source file within a unit.
/// In C, units are made up of a single file, so this is always the
/// implementation.
#[derive(Debug, Copy, Clone)]
pub enum SourceKind {
    Spec,
    Implementation,
    Separate,
}

pub struct ParseResult {
    pub unitname: QName,
    pub kind: SourceKind,
    pub deps: std::collections::HashSet<QName>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub lang: Ustr, // Lower-case
    pub unitname: QName,
    pub kind: SourceKind,
    pub file_node: NodeIndex, // Node for the source file
    pub unit_node: Option<NodeIndex>, // The node for the unit in the graph
    pub deps: std::collections::HashSet<QName>,

    // Is this file ever marked as a Library_Interface for one project in
    // one scenario ?
    pub is_library_interface: bool,

    // Is this file ever a main unit for one project in one scenario ?
    pub is_ever_main: bool,
}

impl SourceFile {
    /// Parse the source file to extract the unit name and the dependencies.
    /// It should return an empty unit name if the file should be ignored (for
    /// instance in Ada there is a `pragma no_body`, or in C there are
    /// preprocessor directives that make the file empty for the compiler).
    pub fn new(
        path: &Path,
        lang: Ustr,
        file_node: NodeIndex,
    ) -> Result<Self, Error> {
        let mut file = File::new(path)?;
        let info = match lang.as_str() {
            "ada" => AdaScanner::parse(AdaLexer::new(
                &mut file,
                AdaLexerOptions {
                    kw_aggregate: false,
                    kw_body: true,
                },
            ))?,
            "c" | "c++" => CppScanner::parse(CppLexer::new(&mut file), path)?,
            lang => Err(Error::CannotParse {
                path: path.into(),
                lang: lang.into(),
            })?,
        };

        Ok(SourceFile {
            path: path.to_owned(),
            lang,
            file_node,
            unit_node: None,
            unitname: info.unitname,
            kind: info.kind,
            deps: info.deps,
            is_library_interface: false,
            is_ever_main: false,
        })
    }
}
