#[derive(Default)]
pub struct Settings {
    pub report_missing_source_dirs: bool,
    // Whether to display error messages when source directories referenced
    // in project files do not actually exist on the disk.
    // This is false by default.
    pub resolve_symbolic_links: bool,
    // Whether to resolve symbol links in paths (false by default).
    // This will in general slow things down because we need more
    // system calls, but will avoid parsing files multiple times
    // if they are seen via different symbol links.
}
