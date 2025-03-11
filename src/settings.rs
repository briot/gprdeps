use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Settings {
    // Whether to display error messages when source directories referenced
    // in project files do not actually exist on the disk.
    // This is false by default.
    pub report_missing_source_dirs: bool,

    // Whether to resolve symbol links in paths (false by default).
    // This will in general slow things down because we need more
    // system calls, but will avoid parsing files multiple times
    // if they are seen via different symbol links.
    pub resolve_symbolic_links: bool,

    // List of project files implicitly imported by all others.  This is meant
    // for runtime files for the various languages.
    pub runtime_gpr: Vec<PathBuf>,

    // The root directory, underneath which we look for all project files
    pub root: Vec<PathBuf>,

    // Whether to remove some attributes from projects (all the ones not used
    // by this tool)
    pub trim: bool,

    // All output paths are displayed relative to this directory.  We prefer
    // to display relative file names, in general, as those are shorter and
    // more portable across mchines.
    pub relto: PathBuf,
}

impl Settings {
    /// Format a path for display.  We prefer to display relative file names,
    /// since those are shorter and will stay the same on different machines.
    pub fn display_path<'a>(&self, path: &'a Path) -> std::path::Display<'a> {
        path.strip_prefix(&self.relto).unwrap_or(path).display()
    }

    /// Return the list of root directories (computed from --root)
    pub fn iter_root_dirs(&self) -> impl Iterator<Item = &Path> {
        self.root
            .iter()
            .map(|r| if r.is_dir() { r } else { r.parent().unwrap() })
    }

    /// Print a list of files
    pub fn print_files(
        &self,
        msg: &str,
        mut paths: Vec<&PathBuf>,
        quiet: bool,
    ) {
        if !quiet || !paths.is_empty() {
            println!("{}", msg);
        }
        paths.sort();
        for path in paths {
            println!("   {}", self.display_path(path));
        }
    }

    pub fn print_lines(&self, msg: &str, mut lines: Vec<String>, quiet: bool) {
        if !quiet || !lines.is_empty() {
            println!("{}", msg);
        }
        lines.sort();
        for line in lines {
            println!("   {}", line);
        }
    }
}
