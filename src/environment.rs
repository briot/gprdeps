use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::errors::Error;
use crate::gpr::GprFile;
use crate::gpr_scanner::GprScanner;
use crate::graph::{DepGraph, Edge, GPRIndex, Node, NodeIndex, PathToIndexes};
use crate::rawgpr::RawGPR;
use crate::scenarios::AllScenarios;
use crate::settings::Settings;
use crate::sourcefile::SourceFile;
use crate::units::{QualifiedName, SourceKind};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ustr::Ustr;

type GPRDetails<'a> = HashMap<GPRIndex, (&'a PathBuf, RawGPR)>;
type GPRIdxToFile = HashMap<GPRIndex, GprFile>;
type UnitsMap = HashMap<QualifiedName, NodeIndex>;
type SourceFilesMap = HashMap<
    PathBuf,
    (
        NodeIndex, // Node for the source file
        Ustr,      // Language of the source file
        Option<(SourceKind, NodeIndex)>,
    ), // unit, and role played by the file
>;

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    settings: Settings,
    scenarios: AllScenarios,
    graph: DepGraph,
    files: SourceFilesMap,
    units: UnitsMap,
}

impl Environment {
    /// Add a project that is implicitly imported by all projects.
    /// This is mostly meant for projects that include runtime files for
    /// various languages.

    pub fn add_implicit_project(&mut self, path: &Path) {}

    /// Find all GPR files that need to be parsed, in either root directory
    /// or one of its child directories.
    /// Insert dummy nodes in the graph, so that we have an index

    fn find_all_gpr(&mut self, root: &Path) -> PathToIndexes {
        let mut gprpath_to_indexes = HashMap::new();
        for (gpridx, gpr) in crate::findfile::FileFind::new(root).enumerate() {
            if gprpath_to_indexes.contains_key(&gpr) {
                // ??? We could instead reuse the same gpridx and nodeidx, but
                // this is unexpected.
                let path = gpr.to_path_buf();
                panic!("Project file found multiple times: {}", path.display());
            }
            let gpridx = GPRIndex::new(gpridx);
            let nodeidx = self.graph.add_node(Node::Project(gpridx));
            gprpath_to_indexes.insert(gpr, (gpridx, nodeidx));
        }
        gprpath_to_indexes
    }

    /// Parse the raw GPR files, but do not analyze them yet.
    /// We can however setup dependencies in the graph already, so that we can
    /// do topological sort later and parse them in the correct order.

    fn parse_raw_gprs<'b>(
        &mut self,
        gprs: &'b PathToIndexes,
    ) -> Result<GPRDetails<'b>, Error> {
        let mut rawfiles = HashMap::new();
        for (path, (gpridx, nodeidx)) in gprs {
            let mut file = crate::files::File::new(path)?;
            let options = AdaLexerOptions {
                kw_aggregate: true,
                kw_body: false,
            };
            let raw = GprScanner::parse(
                AdaLexer::new(&mut file, options),
                path,
                gprs,
                &self.settings,
            )?;
            for dep in &raw.imported {
                self.graph.add_edge(*nodeidx, *dep, Edge::GPRImports);
            }
            if let Some(ext) = raw.extends {
                self.graph.add_edge(*nodeidx, ext, Edge::GPRExtends);
            }
            rawfiles.insert(*gpridx, (path, raw));
        }
        Ok(rawfiles)
    }

    /// Process the projects in topological order, so that any reference to a
    /// variable or attribute in another project is found.

    fn process_projects(
        &mut self,
        rawfiles: &GPRDetails,
    ) -> Result<GPRIdxToFile, Error> {
        let mut gprs = HashMap::new();
        for nodeidx in self.graph.toposort().iter().rev() {
            let gpridx = self.graph.get_project(*nodeidx);
            let (path, raw) = &rawfiles[&gpridx];
            let deps = self.graph.gpr_dependencies(*nodeidx);
            let gprdeps = deps.iter().map(|i| &gprs[i]).collect::<Vec<_>>();
            let mut gpr = GprFile::new(path, *nodeidx, raw.name);
            gpr.process(
                raw,
                raw.extends.map(|i| &gprs[&self.graph.get_project(i)]),
                &gprdeps,
                &mut self.scenarios,
            )?;
            gprs.insert(gpridx, gpr);
        }
        Ok(gprs)
    }

    /// Resolve all source dirs and source files as found in the project, once
    /// we have processed all projects.
    /// Remove all attributes we do not actually need, which makes some
    /// scenarios useless too.

    fn find_sources(&mut self, gprs: &mut GPRIdxToFile) -> Result<(), Error> {
        let mut all_source_dirs = HashSet::new();
        for gpr in gprs.values_mut() {
            gpr.trim();
            gpr.resolve_source_dirs(&mut all_source_dirs, &self.settings)?;
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

    fn add_source(
        &mut self,
        path: &PathBuf,
        lang: Ustr,
    ) -> (NodeIndex, Option<(SourceKind, NodeIndex)>) {
        match self.files.get(path) {
            Some((s1, _, ku1)) => (*s1, *ku1),
            None => {
                let sidx = self.graph.add_node(Node::Source(path.clone()));
                let mut s = SourceFile::new(path, lang);
                let opt_kind_and_uidx = match s.parse() {
                    Err(e) => {
                        println!("Failed to parse {}: {}", path.display(), e);
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
                        Some((info.kind, uidx))
                    }
                };
                self.files
                    .insert(path.clone(), (sidx, lang, opt_kind_and_uidx));
                (sidx, opt_kind_and_uidx)
            }
        }
    }

    /// Create graph nodes for the source files, and group the files into
    /// logical units.

    fn add_sources_to_graph(
        &mut self,
        gprs: &mut GPRIdxToFile,
    ) -> Result<(), Error> {
        for gpr in gprs.values() {
            for (scenario, sources) in &gpr.source_files {
                for (path, lang) in sources {
                    let (sidx, opt_kind_and_uidx) =
                        self.add_source(path, *lang);

                    // Add edges
                    self.graph.add_edge(
                        gpr.index,
                        sidx,
                        Edge::ProjectSource(*scenario),
                    );

                    if let Some((kind, uidx)) = opt_kind_and_uidx {
                        self.graph.add_edge(
                            uidx,
                            sidx,
                            match kind {
                                SourceKind::Spec => Edge::UnitSpec(*scenario),
                                SourceKind::Implementation => {
                                    Edge::UnitImpl(*scenario)
                                }
                                SourceKind::Separate => {
                                    Edge::UnitSeparate(*scenario)
                                }
                            },
                        );

                        // Duplicate the source-level dependencies as unit-level
                        // dependencies.  This makes traversing the graph much
                        // easier.
                        let source_deps = self //  Need tmp vector
                            .graph
                            .0
                            .edges(sidx)
                            .filter(|e| {
                                matches!(e.weight(), Edge::SourceImports)
                            })
                            .map(|e| e.target())
                            .collect::<Vec<_>>();
                        for d in source_deps {
                            self.graph.add_edge(
                                uidx,
                                d,
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

    pub fn parse_all(&mut self, path: &Path) -> Result<(), Error> {
        let gprpath_to_indexes = self.find_all_gpr(path);
        let rawfiles = self.parse_raw_gprs(&gprpath_to_indexes)?;
        let mut gprs = self.process_projects(&rawfiles)?;
        self.find_sources(&mut gprs)?;
        self.add_sources_to_graph(&mut gprs)?;

        println!("Total source files={}", self.files.len());
        println!("Total units={}", self.units.len());
        println!(
            "Nodes in graph after adding files and units: {}",
            self.graph.node_count()
        );

        Ok(())
    }

    /// Report the list of units directly imported by the given file

    pub fn show_direct_dependencies(&self, path: &Path) -> Result<(), Error> {
        let (sidx, _, _) = self
            .files
            .get(&std::path::PathBuf::from(path))
            .ok_or(Error::NotFound("File not found in graph".into()))?;
        let mut direct_deps = self
            .graph
            .0
            .edges(*sidx)
            .filter(|e| matches!(e.weight(), Edge::SourceImports))
            .filter_map(|e| self.graph.get_unit_name(e.target()).ok())
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
        let (sidx, _, opt_kind_and_uidx) = self
            .files
            .get(&std::path::PathBuf::from(path))
            .ok_or(Error::NotFound("File not found in graph".into()))?;
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&self.graph.0, |e| {
                matches!(e.weight(), Edge::UnitImports(_))
            });
        if let Some((_, uidx)) = opt_kind_and_uidx {
            let mut dfs = petgraph::visit::Dfs::new(&filtered, *uidx);
            let mut deps = Vec::new();
            while let Some(node) = dfs.next(&filtered) {
                if node != *sidx {
                    let mut d: String =
                        format!("   {}", self.graph.get_unit_name(node)?);

                    for (nodeidx, scenars) in
                        self.graph.get_specs(&mut self.scenarios, node)
                    {
                        d.push('\n');
                        d.push_str(&format!(
                            "      {} ",
                            self.graph.get_source_path(nodeidx)?.display()
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
        }
        Ok(())
    }
}
