use crate::gpr::GPR;
use crate::scenarios::AllScenarios;
use petgraph::Directed;
use petgraph::graph::Graph;
use petgraph::algo::toposort;

pub enum Node {
    Project(GPR),
    Source,
}

#[derive(Debug)]
pub enum Edge {
    Imports,
}

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;
pub type PathToId = std::collections::HashMap<std::path::PathBuf, NodeIndex>;

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub scenarios: AllScenarios,
    pub graph: Graph<Node, Edge, Directed, u32>,
}

impl Environment {

    /// Recursively look for all project files, parse them and prepare the
    /// dependency graph.
    pub fn parse_all(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut gprmap: PathToId = Default::default();

        // Find all GPR files we will have to parse
        // Insert dummy nodes in the graph, so that we have an index
        for gpr in crate::findfile::FileFind::new(path) {
            let gprfile = GPR::new(gpr.to_path_buf());
            let idx = self.graph.add_node(Node::Project(gprfile));
            gprmap.insert(gpr.to_path_buf(), idx);
        }

        // Parse the GPR files, but do not analyze them yet.
        // We can however setup dependencies in the graph already.

        let mut edges = Vec::new();
        for (path, idx) in &gprmap {
            let file = crate::files::File::new(path)?;
            let scan = crate::scanner::Scanner::new(&file);
            let raw = scan.parse(&gprmap)?;
            if let Node::Project(ref mut p) = self.graph[*idx] {
                for dep in &raw.imported {
                    edges.push((*idx, *dep));
                }
                p.set_raw(raw);
            }
        }
        for (from, to) in edges {
            self.graph.add_edge(from, to, Edge::Imports);
        }

        // Process the projects in topological order, so that any reference to a
        // variable or attribute in another project is found.

        println!("Parsed {} gpr files", gprmap.len());

        let sorted = toposort(&self.graph, None).unwrap();
        for idx in sorted.iter().rev() {
            match &self.graph[*idx] {
               Node::Project(gpr) => print!("{} ", gpr),
               _ => print!("unknown node"),
            }
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
