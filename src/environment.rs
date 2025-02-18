use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::allscenarios::AllScenarios;
use crate::errors::Error;
use crate::gpr::GprFile;
use crate::gpr_scanner::{GprPathToIndex, GprScanner};
use crate::graph::{DepGraph, Edge, Node, NodeIndex};
use crate::qnames::QName;
use crate::rawgpr::RawGPR;
use crate::settings::Settings;
use crate::sourcefile::{SourceFile, SourceKind};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use ustr::Ustr;

type RawGPRs = HashMap<NodeIndex, RawGPR>;
type UnitsMap = HashMap<QName, NodeIndex>;
type GprMap = HashMap<PathBuf, GprFile>;

// Maps files to details about the file.
type SourceFilesMap = HashMap<PathBuf, Rc<RefCell<SourceFile>>>;

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

    /// Create a new SourceFile, or return an existing one for the same path.
    /// It is an error if the same file has already been registered with
    /// different attributes.
    pub fn register_source(
        &mut self,
        path: &Path,
        lang: Ustr,
    ) -> Result<Rc<RefCell<SourceFile>>, Error> {
        //  ??? Can we use raw_entry to avoid the clone
        let f = self.files.entry(path.into()).or_insert_with(|| {
            let sidx = self.graph.add_node(Node::Source(path.into()));
            let mut s = SourceFile::new(path, lang, sidx)
                .expect("Should deal with error");
            if s.unitname != QName::default() {
                let u = Environment::add_unit(
                    &mut self.units,
                    &mut self.graph,
                    &s.unitname,
                );
                s.unit_node = Some(u);

                // An implementation or separate depends on everything
                // from the same unit, but the spec doesn't.
                match s.kind {
                    SourceKind::Spec => {}
                    SourceKind::Implementation | SourceKind::Separate => {
                        self.graph.add_edge(s.file_node, u, Edge::SourceImports)
                    }
                }
            }
            for dep in &s.deps {
                Environment::add_source_import(
                    &mut self.units,
                    &mut self.graph,
                    s.file_node,
                    dep,
                );
            }

            // Automatically depend on parent unit
            if let Some(parent) = s.unitname.parent() {
                Environment::add_source_import(
                    &mut self.units,
                    &mut self.graph,
                    s.file_node,
                    &parent,
                );
            }

            Rc::new(RefCell::new(s))
        });

        if f.borrow().lang != lang {
            Err(Error::InconsistentFileLang(path.into()))
        } else {
            Ok(f.clone())
        }
    }

    /// Add a unit to the graph, if not there yet
    fn add_unit(
        units: &mut UnitsMap,
        graph: &mut DepGraph,
        unitname: &QName,
    ) -> NodeIndex {
        match units.get(unitname) {
            Some(u) => *u,
            None => {
                units.insert(
                    unitname.clone(),
                    graph.add_node(Node::Unit(unitname.clone())),
                );
                *units.get(unitname).unwrap()
            }
        }
    }

    /// Add a new dependency from the source to a given unit
    fn add_source_import(
        units: &mut UnitsMap,
        graph: &mut DepGraph,
        source: NodeIndex,
        unit: &QName,
    ) {
        let u = Environment::add_unit(units, graph, unit);
        graph.add_edge(source, u, Edge::SourceImports);
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
            for (scenario, sources) in gpr.sources.iter() {
                for s in sources {
                    let sm = s.file.borrow();

                    self.graph.add_edge(
                        gpridx,
                        sm.file_node,
                        Edge::ProjectSource(*scenario),
                    );

                    if let Some(u) = sm.unit_node {
                        self.graph.add_edge(
                            u,
                            sm.file_node,
                            match sm.kind {
                                SourceKind::Spec => Edge::UnitSpec(*scenario),
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
                        // ??? Though likely incorrect: if we have unit A
                        // in two different projects, and file F depends
                        // on A, we can resolve the dependency, but then
                        // the graph shows a UnitImports to A without
                        // telling us which of the two A.
                        for depunit in &sm.deps {
                            let dep_node = Environment::add_unit(
                                &mut self.units,
                                &mut self.graph,
                                depunit,
                            );

                            self.graph.add_edge(
                                u,
                                dep_node,
                                Edge::UnitImports(*scenario),
                            );
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

        let mut all_source_dirs = HashSet::new();
        for gpr in gprmap.values_mut() {
            if trim_attributes {
                gpr.trim();
            }
            gpr.resolve_source_dirs(&mut all_source_dirs, settings)?;
            gpr.resolve_naming(&mut self.scenarios);
            gpr.resolve_source_files(self, &all_source_dirs);
        }

        self.add_sources_to_graph(gprindexes, &mut gprmap)?;

        self.gprs = gprmap;
        Ok(())
    }

    /// Displays some stats about the graph
    pub fn print_stats(&self) {
        self.scenarios.print_stats();
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
            .clone();
        let file = info.borrow();
        let mut direct_deps = self
            .graph
            .0
            .edges(file.file_node)
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
            .clone();
        let file = info.borrow();
        let unit_node = match file.unit_node {
            None => {
                return Err(Error::NotFound("No unit for this file".into()))
            }
            Some(u) => u,
        };
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&self.graph.0, |e| {
                matches!(e.weight(), Edge::UnitImports(_))
            });
        let mut dfs = petgraph::visit::Dfs::new(&filtered, unit_node);
        let mut deps = Vec::new();
        while let Some(node) = dfs.next(&filtered) {
            if node != file.file_node {
                let mut d: String =
                    format!("   {}", self.graph.get_unit(node)?);

                for (nodeidx, scenars) in
                    self.graph.get_specs(&mut self.scenarios, node)
                {
                    d.push('\n');
                    write!(
                        d,
                        "      {} ",
                        self.graph.get_source(nodeidx)?.display()
                    )?;
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

    /// Report all source files that are never imported.
    /// Ignore those units that are "main" units for a project.
    /// Ignore files in specific directories (typically, third-party libraries)
    pub fn show_unused_sources(&self) -> Result<(), Error> {
        for n in self.graph.0.node_indices() {
            let node = &self.graph.0[n];
            if let Node::Unit(qname) = node {
                let mut count = 0;
                for e in self.graph.0.edges_directed(n, Direction::Incoming) {
                    if let Edge::SourceImports = e.weight() {
                        count += 1;
                        // println!(
                        //     "MANU    imported by {:?}",
                        //     self.graph.0[e.source()],
                        // );
                    }
                }

                if count == 0 {
                    println!("MANU unused unit {:?}", qname);
                }
            }
        }

        Ok(())
    }

    /// Retrieve the node for a project node
    pub fn get_gpr(&self, gprpath: &Path) -> Option<&GprFile> {
        self.gprs.get(gprpath)
    }
}
