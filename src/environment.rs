use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::errors::Error;
use crate::gpr::GprFile;
use crate::gpr_scanner::{GprPathToIndex, GprScanner};
use crate::graph::{DepGraph, Edge, Node, NodeIndex};
use crate::rawgpr::RawGPR;
use crate::scenarios::AllScenarios;
use crate::settings::Settings;
use crate::sourcefile::SourceFile;
use crate::units::{QualifiedName, SourceKind};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ustr::Ustr;

type RawGPRs = HashMap<NodeIndex, RawGPR>;
type UnitsMap = HashMap<QualifiedName, NodeIndex>;
type GprMap = HashMap<PathBuf, GprFile>;

#[derive(Clone)]
struct FileInfo {
    file_node: NodeIndex, // Node for the source file
    _lang: Ustr,          // Language of the source file
    kind: SourceKind,     // Role the file plays in its unit
    unit_node: NodeIndex, // The node for the unit in the graph
}

// Maps files to details about the file.  Contains None when the file is to
// be ignored for some reason
type SourceFilesMap = HashMap<PathBuf, Option<FileInfo>>;

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub scenarios: AllScenarios,
    graph: DepGraph,
    gprs: GprMap,
    files: SourceFilesMap,
    units: UnitsMap,

    implicit_projects: Vec<NodeIndex>,
}

impl Environment {
    /// Register a GPR file into the graph.
    /// Double-check it isn't there yet.
    fn register_gpr(
        &mut self,
        gpr: PathBuf,
        gprs: &mut HashMap<PathBuf, NodeIndex>,
    ) -> NodeIndex {
        // ??? Perhaps use raw_entry so that users can pass a &PathBuf parameter
        *gprs.entry(gpr).or_insert_with_key(|key| {
            self.graph.add_node(Node::Project(key.clone()))
        })
    }

    /// Find all GPR files that need to be parsed, in either root directory
    /// or one of its child directories.  If root is a project, we load it and
    /// all its dependencies.
    /// Insert dummy nodes in the graph, so that we have an index
    fn find_all_gpr(
        &mut self,
        root: &Path,
        settings: &Settings,
    ) -> GprPathToIndex {
        let mut gprs = GprPathToIndex::new();
        for imp in &settings.runtime_gpr {
            let nodeidx = self.register_gpr(imp.clone(), &mut gprs);
            self.implicit_projects.push(nodeidx);
        }

        if root.is_file() {
            self.register_gpr(root.to_path_buf(), &mut gprs);
        } else {
            for gpr in crate::findfile::FileFind::new(root) {
                self.register_gpr(gpr, &mut gprs);
            }
        }
        gprs
    }

    /// Parse the raw GPR files, but do not analyze them yet.
    /// We can however setup dependencies in the graph already, so that we can
    /// do topological sort later and parse them in the correct order.
    fn parse_raw_gprs(
        &mut self,
        gprs: &mut GprPathToIndex,
        settings: &Settings,
    ) -> Result<RawGPRs, Error> {
        let mut rawfiles = RawGPRs::new();

        let mut tovisit: Vec<(PathBuf, NodeIndex)> =
            gprs.iter().map(|(p, n)| (p.clone(), *n)).collect();

        while let Some(visit) = tovisit.pop() {
            let (path, nodeidx) = visit;

            let mut file = crate::files::File::new(&path)?;
            let options = AdaLexerOptions {
                kw_aggregate: true,
                kw_body: false,
            };
            let raw = GprScanner::parse(
                AdaLexer::new(&mut file, options),
                &path,
                settings,
            )?;

            if !raw.is_abstract && !self.implicit_projects.contains(&nodeidx) {
                for imp in &self.implicit_projects {
                    self.graph.add_edge(nodeidx, *imp, Edge::GPRImports);
                }
            }
            for dep in &raw.imported {
                let depidx = match gprs.get(dep) {
                    None => {
                        let idx = self.register_gpr(dep.clone(), gprs);
                        tovisit.push((dep.clone(), idx));
                        idx
                    }
                    Some(depidx) => *depidx,
                };
                self.graph.add_edge(nodeidx, depidx, Edge::GPRImports);
            }
            if let Some(ref ext) = raw.extends {
                let extidx = match gprs.get(ext) {
                    None => {
                        let idx = self.register_gpr(ext.clone(), gprs);
                        tovisit.push((ext.clone(), idx));
                        idx
                    }
                    Some(extidx) => *extidx,
                };
                self.graph.add_edge(nodeidx, extidx, Edge::GPRExtends);
            }
            rawfiles.insert(nodeidx, raw);
        }
        Ok(rawfiles)
    }

    /// Process the projects in topological order, so that any reference to a
    /// variable or attribute in another project is found.
    fn process_projects(&mut self, rawfiles: RawGPRs) -> Result<GprMap, Error> {
        let mut gprs = GprMap::new();
        for nodeidx in self.graph.toposort().iter().rev() {
            let raw = &rawfiles[nodeidx];
            let deps = self.graph.gpr_dependencies(*nodeidx);
            let gprdeps: Vec<&GprFile> = deps
                .iter()
                .map(|i| self.graph.get_project(*i).map(|path| &gprs[path]))
                .collect::<Result<Vec<_>, _>>()?;
            let mut gpr = GprFile::new(&raw.path);
            gpr.process(
                raw,
                raw.extends.as_ref().and_then(|e| gprs.get(e)),
                &gprdeps,
                &mut self.scenarios,
            )?;
            gprs.insert(raw.path.clone(), gpr);
        }
        Ok(gprs)
    }

    /// Resolve all source dirs and source files as found in the project, once
    /// we have processed all projects.
    /// Remove all attributes we do not actually need, which makes some
    /// scenarios useless too.
    fn find_sources(
        &mut self,
        gprs: &mut GprMap,
        settings: &Settings,
        trim_attributes: bool,
    ) -> Result<(), Error> {
        let mut all_source_dirs = HashSet::new();
        for gpr in gprs.values_mut() {
            if trim_attributes {
                gpr.trim();
            }
            gpr.resolve_source_dirs(&mut all_source_dirs, settings)?;
            gpr.resolve_source_files(&all_source_dirs, &mut self.scenarios);
        }
        Ok(())
    }

    /// Add a unit to the graph, if not there yet
    fn add_unit(&mut self, unitname: QualifiedName) -> NodeIndex {
        // ??? Can we avoid the clone here if the unit is already there
        *self
            .units
            .entry(unitname.clone())
            .or_insert_with(|| self.graph.add_node(Node::Unit(unitname)))
    }

    /// Add a new dependency from the source to a given unit
    fn add_source_import(&mut self, source: NodeIndex, unit: QualifiedName) {
        let u = self.add_unit(unit);
        self.graph.add_edge(source, u, Edge::SourceImports);
    }

    /// Retrieve or add source file to the graph.
    /// Returns the node for the source, and information about the unit.
    fn add_source(&mut self, path: &PathBuf, lang: Ustr) -> Option<FileInfo> {
        match self.files.get(path) {
            Some(None) => None,
            Some(Some(info)) => Some(info.clone()),
            None => {
                let sidx = self.graph.add_node(Node::Source(path.clone()));
                let mut s = SourceFile::new(path, lang);
                match s.parse() {
                    Err(e) => {
                        println!("Failed to parse {}: {}", path.display(), e);
                        self.files.insert(path.clone(), None);
                        None
                    }
                    Ok(info) if info.unitname == QualifiedName::default() => {
                        self.files.insert(path.clone(), None);
                        None
                    }
                    Ok(info) => {
                        for dep in info.deps {
                            self.add_source_import(sidx, dep);
                        }

                        // Automatically depend on parent unit
                        if let Some(parent) = info.unitname.parent() {
                            self.add_source_import(sidx, parent);
                        }

                        // An implementation or separate depends on everything
                        // from the same unit, but the spec doesn't.
                        let uidx = self.add_unit(info.unitname);
                        match info.kind {
                            SourceKind::Spec => {}
                            SourceKind::Implementation
                            | SourceKind::Separate => self.graph.add_edge(
                                sidx,
                                uidx,
                                Edge::SourceImports,
                            ),
                        }
                        let details = FileInfo {
                            file_node: sidx,
                            _lang: lang,
                            kind: info.kind,
                            unit_node: uidx,
                        };
                        self.files.insert(path.clone(), Some(details.clone()));
                        Some(details)
                    }
                }
            }
        }
    }

    /// Create graph nodes for the source files, and group the files into
    /// logical units.
    fn add_sources_to_graph(
        &mut self,
        gprindexes: GprPathToIndex,
        gprs: &mut GprMap,
    ) -> Result<(), Error> {
        for (path, gpridx) in gprindexes {
            let gpr = gprs.get_mut(&path).unwrap();

            for (scenario, sources) in &gpr.source_files {
                for (path, lang) in sources {
                    match self.add_source(path, *lang) {
                        None => {
                            // File is being discarded for some reason
                        }
                        Some(info) => {
                            // Add edges
                            self.graph.add_edge(
                                gpridx,
                                info.file_node,
                                Edge::ProjectSource(*scenario),
                            );

                            self.graph.add_edge(
                                info.unit_node,
                                info.file_node,
                                match info.kind {
                                    SourceKind::Spec => {
                                        Edge::UnitSpec(*scenario)
                                    }
                                    SourceKind::Implementation => {
                                        Edge::UnitImpl(*scenario)
                                    }
                                    SourceKind::Separate => {
                                        Edge::UnitSeparate(*scenario)
                                    }
                                },
                            );

                            // Duplicate the source-level dependencies as
                            // unit-level dependencies. This makes traversing
                            // the graph much easier.
                            let source_deps =
                                self //  Need tmp vector
                                    .graph
                                    .0
                                    .edges(info.file_node)
                                    .filter(|e| {
                                        matches!(
                                            e.weight(),
                                            Edge::SourceImports
                                        )
                                    })
                                    .map(|e| e.target())
                                    .collect::<Vec<_>>();
                            for d in source_deps {
                                self.graph.add_edge(
                                    info.unit_node,
                                    d,
                                    Edge::UnitImports(*scenario),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Recursively look for all project files, parse them and prepare the
    /// dependency graph.
    pub fn parse_all(
        &mut self,
        path_or_gpr: &Path,
        settings: &Settings,
        trim_attributes: bool,
    ) -> Result<(), Error> {
        let mut gprindexes: GprPathToIndex =
            self.find_all_gpr(path_or_gpr, settings);
        let rawfiles: RawGPRs =
            self.parse_raw_gprs(&mut gprindexes, settings)?;
        let mut gprmap: GprMap = self.process_projects(rawfiles)?;
        self.find_sources(&mut gprmap, settings, trim_attributes)?;
        self.add_sources_to_graph(gprindexes, &mut gprmap)?;

        self.gprs = gprmap;
        Ok(())
    }

    /// Displays some stats about the graph
    pub fn print_stats(&self) {
        println!("Graph nodes:  {:-7}", self.graph.node_count());
        println!("   Projects:     = {:-6}", self.gprs.len());
        println!("   Units:        + {:-6}", self.units.len());
        println!("   Source files: + {:-6}", self.files.len());
        println!("Graph edges:  {:-7}", self.graph.edge_count());
    }

    /// Report the list of units directly imported by the given file
    pub fn show_direct_dependencies(&self, path: &Path) -> Result<(), Error> {
        let info = self
            .files
            .get(&std::path::PathBuf::from(path))
            .ok_or(Error::NotFound("File not found in graph".into()))?
            .clone()
            .ok_or(Error::NotFound("File has no relevant content".into()))?
            .clone();
        let mut direct_deps = self
            .graph
            .0
            .edges(info.file_node)
            .filter(|e| matches!(e.weight(), Edge::SourceImports))
            .filter_map(|e| self.graph.get_unit(e.target()).ok())
            .map(|e| format!("   {}", e))
            .collect::<Vec<_>>();
        direct_deps.sort();
        println!("{}", direct_deps.join("\n"));
        Ok(())
    }

    /// Report all dependencies of the given source file
    pub fn show_indirect_dependencies(
        &mut self,
        path: &Path,
    ) -> Result<(), Error> {
        let info = self
            .files
            .get(&std::path::PathBuf::from(path))
            .ok_or(Error::NotFound("File not found in graph".into()))?
            .clone()
            .ok_or(Error::NotFound("File has no relevant content".into()))?;
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&self.graph.0, |e| {
                matches!(e.weight(), Edge::UnitImports(_))
            });
        let mut dfs = petgraph::visit::Dfs::new(&filtered, info.unit_node);
        let mut deps = Vec::new();
        while let Some(node) = dfs.next(&filtered) {
            if node != info.file_node {
                let mut d: String =
                    format!("   {}", self.graph.get_unit(node)?);

                for (nodeidx, scenars) in
                    self.graph.get_specs(&mut self.scenarios, node)
                {
                    d.push('\n');
                    d.push_str(&format!(
                        "      {} ",
                        self.graph.get_source(nodeidx)?.display()
                    ));
                    for s in scenars {
                        d.push(' ');
                        d.push_str(&self.scenarios.describe(s));
                    }
                }

                deps.push(d);
            }
        }
        deps.sort();
        println!("{}", deps.join("\n"));
        Ok(())
    }

    /// Retrieve the node for a project node
    pub fn get_gpr(&self, gprpath: &Path) -> Option<&GprFile> {
        self.gprs.get(gprpath)
    }
}
