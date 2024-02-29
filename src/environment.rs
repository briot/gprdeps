use crate::ada_lexer::{AdaLexer, AdaLexerOptions};
use crate::errors::Error;
use crate::gpr::GprFile;
use crate::gpr_scanner::GprScanner;
use crate::graph::{DepGraph, Edge, GPRIndex, Node};
use crate::scenarios::AllScenarios;
use crate::settings::Settings;
use crate::sourcefile::SourceFile;
use crate::units::SourceKind;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    settings: Settings,
    scenarios: AllScenarios,
    graph: DepGraph,
}

impl Environment {
    /// Recursively look for all project files, parse them and prepare the
    /// dependency graph.
    pub fn parse_all(&mut self, path: &std::path::Path) -> Result<(), Error> {
        let mut gprpath_to_indexes = HashMap::new();

        // Find all GPR files we will have to parse
        // Insert dummy nodes in the graph, so that we have an index
        for (gpridx, gpr) in crate::findfile::FileFind::new(path).enumerate() {
            let gpridx = GPRIndex::new(gpridx);
            let nodeidx = self.graph.add_node(Node::Project(gpridx));
            let path = gpr.to_path_buf();
            if gprpath_to_indexes.contains_key(&gpr) {
                // ??? We could instead reuse the same gpridx and nodeidx, but
                // this is unexpected.
                panic!("Project file found multiple times: {}", path.display());
            }
            gprpath_to_indexes.insert(gpr, (gpridx, nodeidx));
        }
        println!(
            "Nodes in graph after adding gpr: {}", self.graph.node_count());

        // Parse the raw GPR files, but do not analyze them yet.
        // We can however setup dependencies in the graph already.

        let mut rawfiles = HashMap::new();
        for (path, (gpridx, nodeidx)) in &gprpath_to_indexes {
            let mut file = crate::files::File::new(path)?;
            let options = AdaLexerOptions {
                kw_aggregate: true,
                kw_body: false,
            };
            let lex = AdaLexer::new(&mut file, options);
            let raw =
                GprScanner::parse(lex, path, &gprpath_to_indexes, &self.settings)?;

            for dep in &raw.imported {
                self.graph.add_edge(*nodeidx, *dep, Edge::GPRImports);
            }
            if let Some(ext) = raw.extends {
                self.graph.add_edge(*nodeidx, ext, Edge::GPRExtends);
            }

            rawfiles.insert(*gpridx, (path, raw));
        }

        // Process the projects in topological order, so that any reference to a
        // variable or attribute in another project is found.

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

        // Remove all attributes we do not actually need, which makes some
        // scenarios useless too
        let mut useful_scenars = HashSet::new();
        let mut all_source_dirs = HashSet::new();
        for gpr in gprs.values_mut() {
            gpr.trim();
            gpr.find_used_scenarios(&mut useful_scenars);
            gpr.resolve_source_dirs(&mut all_source_dirs, &self.settings)?;
            gpr.resolve_source_files(&all_source_dirs, &mut self.scenarios);
        }
        let files_count: usize =
            all_source_dirs.iter().map(|d| d.files_count()).sum();

        println!("Actually used scenarios={}", useful_scenars.len());
        println!("Total source directories={}", all_source_dirs.len());
        println!("Total files in source dirs={}", files_count);

        // Create graph nodes for the source files, and group the files into
        // logical units.

        let mut all_source_files = HashMap::new();
        let mut units = HashMap::new();

        for gpr in gprs.values() {
            for (scenario, sources) in &gpr.source_files {
                for (path, lang) in sources {
                    // Create a new graph node for the source file if needed
                    let (sidx, _, opt_kind_and_uidx) = all_source_files
                        .entry(path)
                        .or_insert_with(|| {

                            // Create a new node, since the file is not in the
                            // graph yet
                            let sidx = self.graph.add_node(
                                Node::Source(path.clone()));

                            // Parse the source files to find the unit and
                            // its dependencies.
                            let mut s = SourceFile::new(path, *lang);
                            let info = s.parse();
                            let opt_kind_and_uidx = match info {
                                Err(e) => {
                                    println!(
                                        "Failed to parse {}: {}",
                                        path.display(), e
                                    );
                                    None
                                },
                                Ok(info) => {
                                    let uidx = *units
                                        .entry(info.unitname.clone())
                                        .or_insert_with(|| {
                                            self.graph.add_node(
                                                Node::Unit(
                                                    info.unitname.clone()))
                                        });

                                    // We have now found what the source file
                                    // depends on (??? though that should depend
                                    // on the scenario in the case of C).  We
                                    // can register those dependencies in the
                                    // graph.

                                    for dep in info.deps {
                                        let imported_uidx = units
                                            .entry(dep.clone())
                                            .or_insert_with(|| {
                                                self.graph.add_node(
                                                    Node::Unit(dep.clone()))
                                            });
                                        self.graph.add_edge(
                                            sidx,
                                            *imported_uidx,
                                            Edge::Imports,
                                        );
                                    }

                                    Some((info.kind, uidx))
                                },
                            };



                            (sidx, lang, opt_kind_and_uidx)
                        });

                    // Add edges
                    self.graph.add_edge(
                        gpr.index,
                        *sidx,
                        Edge::ProjectSource(*scenario),
                    );

                    if let Some((kind, uidx)) = opt_kind_and_uidx {
                        self.graph.add_edge(
                            *uidx,
                            *sidx,
                            match kind {
                                SourceKind::Spec           =>
                                    Edge::UnitSpec(*scenario),
                                SourceKind::Implementation =>
                                    Edge::UnitImpl(*scenario),
                                SourceKind::Separate       =>
                                    Edge::UnitSeparate(*scenario),
                            }
                        );
                    }
                }
            }
        }
        println!("Total source files={}", all_source_files.len());
        println!("Total units={}", units.len());
        println!(
            "Nodes in graph after adding files and units: {}",
            self.graph.node_count());

        // println!("{:?}", self.graph);

        // Example: find all direct dependencies for servers-sockets.adb

        let path = "/home/briot/dbc/deepblue/General/Networking/private/servers-sockets.adb";
        let (sidx, lang, opt_kind_and_uidx) = all_source_files.get(
             &std::path::PathBuf::from(path)
        ).ok_or(Error::NotFound("File not found in graph".into()))?;

        println!("File: {}", path);
        println!("Language: {}", lang);
        match opt_kind_and_uidx {
            None => println!("Unit: unknown"),
            Some((kind, uidx)) => {
                println!("Source kind: {:?}", kind);
                println!("Unit: {}", self.graph.get_unit_name(*uidx)?);
            }
        }

        // ??? Implementation implicitly depends on spec and separates
        println!("Direct dependencies:");
        let mut direct_deps = self.graph.0.edges(*sidx)
           .filter(|e| matches!(e.weight(), Edge::Imports))
           .filter_map(|e| self.graph.get_unit_name(e.target()).ok())
           .map(|e| format!("   {}", e))
           .collect::<Vec<_>>();
        direct_deps.sort();
        println!("{}", direct_deps.join("\n"));


        //    let pool = threadpool::ThreadPool::new(1);
        //    for gpr in list_of_gpr {
        //        let gpr = gpr.clone();
        //        pool.execute(move || {
        //            let _ = parse_gpr_file(&gpr);
        //        });
        //    }
        //    pool.join();
        Ok(())
    }
}
