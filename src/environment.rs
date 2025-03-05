use crate::{
    ada_lexer::{AdaLexer, AdaLexerOptions},
    allscenarios::AllScenarios,
    errors::Error,
    gpr::GprFile,
    gpr_scanner::{GprPathToIndex, GprScanner},
    graph::{DepGraph, Edge, Node, NodeIndex},
    qnames::QName,
    rawgpr::RawGPR,
    settings::Settings,
    sourcefile::{SourceFile, SourceKind},
};
use petgraph::{visit::EdgeRef, Direction};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::debug;
use ustr::Ustr;

type RawGPRs = HashMap<NodeIndex, RawGPR>;
type UnitsMap = HashMap<QName, NodeIndex>;
pub type GprMap = HashMap<PathBuf, GprFile>;

// Maps files to details about the file.
type SourceFilesMap = HashMap<PathBuf, Rc<RefCell<SourceFile>>>;

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub scenarios: AllScenarios,
    pub graph: DepGraph,
    pub gprs: GprMap,
    pub files: SourceFilesMap,
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
    fn find_all_gpr(&mut self, settings: &Settings) -> GprPathToIndex {
        let mut gprs = GprPathToIndex::new();
        for imp in &settings.runtime_gpr {
            let nodeidx = self.register_gpr(imp.clone(), &mut gprs);
            self.implicit_projects.push(nodeidx);
        }

        for root in &settings.root {
            if root.is_file() {
                self.register_gpr(root.to_path_buf(), &mut gprs);
            } else {
                for gpr in crate::findfile::FileFind::new(root) {
                    self.register_gpr(gpr, &mut gprs);
                }
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
                AdaLexer::new(&mut file, options)?,
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
            let mut gpr = GprFile::new(
                &raw.path,
                raw.is_abstract,
                raw.is_aggregate,
                raw.is_library,
                *nodeidx,
            );
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
                //    match s.kind {
                //        SourceKind::Spec => {}
                //        SourceKind::Implementation | SourceKind::Separate => {
                //            self.graph.add_edge(s.file_node, u, Edge::SourceImports)
                //        }
                //    }
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
                                SourceKind::Spec => Edge::UnitSource((
                                    SourceKind::Spec,
                                    *scenario,
                                )),
                                SourceKind::Implementation => Edge::UnitSource(
                                    (SourceKind::Implementation, *scenario),
                                ),
                                SourceKind::Separate => Edge::UnitSource((
                                    SourceKind::Separate,
                                    *scenario,
                                )),
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// From a list of unit nodes, return the paths of all source files.
    /// We return a set, since the same file might be visible in multiple
    /// scenarios.
    pub fn file_paths_from_units<I>(&self, unit_nodes: I) -> HashSet<PathBuf>
    where
        I: Iterator<Item = NodeIndex>,
    {
        unit_nodes
            .flat_map(|unit| {
                self.graph.0.edges_directed(unit, Direction::Outgoing)
            })
            .filter(|e| matches!(e.weight(), Edge::UnitSource(_)))
            .filter_map(|e| match &self.graph.0[e.target()] {
                Node::Source(path) => Some(path.clone()),
                _ => None,
            })
            .collect() // unique
    }

    /// Iterates over unit dependencies.
    /// In the graph, a unit contains source files, which themselves
    /// import units.
    /// For instance, in Ada:
    ///     unit A
    ///     owns source a.ads
    ///     which imports unit B
    /// This level of indirection allows the actual file to vary depending on
    /// the scenario.
    ///
    /// This function bypasses the file nodes, and returns the dependencies
    /// between units (so we get tuples like (A, B) in the example above).
    /// Iteration starts from a set of target units (B in the example above)
    pub fn iter_unit_deps<'a, I>(
        &'a self,
        targets: I,
    ) -> impl Iterator<Item = (NodeIndex, NodeIndex)> + 'a
    where
        I: Iterator<Item = NodeIndex> + 'a,
    {
        targets
            .flat_map(|unit| {
                self.graph
                    .0
                    .edges_directed(unit, Direction::Incoming)
                    .filter_map(move |e| match e.weight() {
                        Edge::SourceImports => Some((e.source(), unit)),
                        _ => None,
                    })
            })
            .flat_map(|(sourcefile, unit)| {
                self.graph
                    .0
                    .edges_directed(sourcefile, Direction::Incoming)
                    .filter_map(move |e| match e.weight() {
                        Edge::UnitSource(_) => Some((e.source(), unit)),
                        _ => None,
                    })
            })
    }

    /// Recursively look for all project files, parse them and prepare the
    /// dependency graph.
    pub fn parse_all(&mut self, settings: &Settings) -> Result<(), Error> {
        let mut gprindexes: GprPathToIndex = self.find_all_gpr(settings);
        let rawfiles: RawGPRs =
            self.parse_raw_gprs(&mut gprindexes, settings)?;
        let mut gprmap: GprMap = self.process_projects(rawfiles)?;

        let mut all_source_dirs = HashSet::new();
        for gpr in gprmap.values_mut() {
            if settings.trim {
                gpr.trim();
            }
            gpr.resolve_source_dirs(&mut all_source_dirs, settings)?;
            gpr.resolve_naming(&mut self.scenarios);
            gpr.resolve_source_files(self, &all_source_dirs);
            debug!("gpr {:?}", gpr);
        }

        // One we have processed everything, another pass
        for gpr in gprmap.values() {
            gpr.resolve_library_interface(
                &mut self.scenarios,
                &gprmap,
                settings,
            );
        }

        self.add_sources_to_graph(gprindexes, &mut gprmap)?;

        self.gprs = gprmap;
        Ok(())
    }

    /// Displays some stats about the graph
    pub fn print_stats(&self) {
        self.scenarios.print_stats();
        println!("\nGraph nodes:  {:-7}", self.graph.node_count());
        println!("   Projects:     = {:-6}", self.gprs.len());
        println!("   Units:        + {:-6}", self.units.len());
        println!("   Source files: + {:-6}", self.files.len());
        println!("Graph edges:  {:-7}", self.graph.edge_count());
    }

    /// Retrieve the node for a project node
    pub fn get_gpr(&self, gprpath: &Path) -> Option<&GprFile> {
        self.gprs.get(gprpath)
    }
}
