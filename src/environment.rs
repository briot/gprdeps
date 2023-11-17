use crate::gpr::GPR;
use crate::graph::{DepGraph, Edge, GPRIndex, Node};
use crate::scenarios::AllScenarios;
use std::collections::HashMap;

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    scenarios: AllScenarios,
    graph: DepGraph,
}

impl Environment {
    /// Recursively look for all project files, parse them and prepare the
    /// dependency graph.
    pub fn parse_all(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut path_to_indexes = HashMap::new();

        // Find all GPR files we will have to parse
        // Insert dummy nodes in the graph, so that we have an index
        for (gpridx, gpr) in crate::findfile::FileFind::new(path).enumerate() {
            let gpridx = GPRIndex::new(gpridx);
            let nodeidx = self.graph.add_node(Node::Project(gpridx));
            let path = gpr.to_path_buf();
            if path_to_indexes.contains_key(&gpr) {
                // ??? We could instead reuse the same gpridx and nodeidx, but
                // this is unexpected.
                panic!("Project file found multiple times: {}", path.display());
            }
            path_to_indexes.insert(gpr, (gpridx, nodeidx));
        }

        // Parse the raw GPR files, but do not analyze them yet.
        // We can however setup dependencies in the graph already.

        let mut rawfiles = HashMap::new();
        for (path, (gpridx, nodeidx)) in &path_to_indexes {
            let file = crate::files::File::new(path)?;
            let scan = crate::scanner::Scanner::new(&file);
            let raw = scan.parse(&path_to_indexes)?;

            for dep in &raw.imported {
                self.graph.add_edge(*nodeidx, *dep, Edge::Imports);
            }
            if let Some(ext) = raw.extends {
                self.graph.add_edge(*nodeidx, ext, Edge::Extends);
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
            let mut gpr = GPR::new(path, *nodeidx, &raw.name);
            gpr.process(
                raw,
                raw.extends.map(|i| &gprs[&self.graph.get_project(i)]),
                &gprdeps,
                &mut self.scenarios,
            )?;
            gprs.insert(gpridx, gpr);
        }

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
