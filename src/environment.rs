use crate::gpr::GPR;
use crate::scenarios::AllScenarios;
use petgraph::algo::toposort;
use petgraph::graph::Graph;
use petgraph::visit::Bfs;
use petgraph::Directed;

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

pub struct DepGraph(Graph<Node, Edge, Directed, u32>);
impl DepGraph {
    pub fn get_project(&self, idx: NodeIndex) -> &GPR {
        match &self.0[idx] {
            Node::Project(gpr) => gpr,
            _ => panic!("Invalid project reference {:?}", idx),
        }
    }

    pub fn get_project_mut(&mut self, idx: NodeIndex) -> &mut GPR {
        match &mut self.0[idx] {
            Node::Project(ref mut gpr) => gpr,
            _ => panic!("Invalid project reference {:?}", idx),
        }
    }

    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        self.0.add_node(node)
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, data: Edge) {
        self.0.add_edge(from, to, data);
    }

    /// Return all nodes in the graph, sorted topological (a node appears after
    /// all the ones that import it)
    pub fn toposort(&self) -> Vec<NodeIndex> {
        toposort(&self.0, None).unwrap()
    }

    /// Return the list of dependencies for a node.
    /// Each dependency is reported only once (so if a project import both A and
    /// B, which both import a common C, then C is only returned once)
    pub fn gpr_dependencies(&self, start: NodeIndex) -> Vec<&GPR> {
        let mut bfs = Bfs::new(&self.0, start);
        let mut result = Vec::new();
        while let Some(node) = bfs.next(&self.0) {
            result.push(self.get_project(node));
        }
        result
    }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self(Graph::new())
    }
}

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
            let p = self.graph.get_project_mut(*idx);
            for dep in &raw.imported {
                edges.push((*idx, *dep));
            }
            if let Some(ext) = &raw.extends {
                edges.push((*idx, *ext));
            }

            p.set_raw(raw, *idx);
        }
        for (from, to) in edges {
            self.graph.add_edge(from, to, Edge::Imports);
        }

        // Process the projects in topological order, so that any reference to a
        // variable or attribute in another project is found.

        println!("Parsed {} gpr files", gprmap.len());

        for idx in self.graph.toposort().iter().rev() {
            self.graph
                .get_project(*idx)
                .process(&self.graph, &mut self.scenarios)?;
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
