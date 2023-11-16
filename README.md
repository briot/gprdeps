# gprdeps
Dependency-graph build for AdaCore's GPR files

This is mostly a way to learn Rust.

## Goal

This tool will parse one or more projects, written in the
format used by AdaCore for the GNAT compiler and the gprbuild
tool.

From these projects, we gather a large dependency graph, which
let's us perform various queries:

- dependencies between projects
- (TODO) list of source files for each project, for all languages including Ada
  and C
- (TODO) dependencies between those source files
- (TODO) sanity checks that all files are used, for instance.

All dependencies are computed for all possible scenarios.  For
instance, it is possible that some tasking-related files are not
including when building without tasking, or that it has a different
implementation in this case.
