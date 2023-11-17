use petgraph::algo::toposort;
use petgraph::graph::Graph;
use petgraph::visit::Bfs;
use petgraph::Directed;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;
pub type PathToIndexes =
    std::collections::HashMap<std::path::PathBuf, (GPRIndex, NodeIndex)>;

/// A reference to a project file.
/// The actual project must be stored in a separate vec.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GPRIndex(pub usize);
impl GPRIndex {
    pub fn new(idx: usize) -> GPRIndex {
        GPRIndex(idx)
    }
}

/// The nodes of a graph
pub enum Node {
    Project(GPRIndex),
    Source,
}

/// The edges of a graph
#[derive(Debug)]
pub enum Edge {
    Extends,
    Imports,
}

/// A unified dependency graph, for both projects and source files
pub struct DepGraph(Graph<Node, Edge, Directed, u32>);
impl DepGraph {
    pub fn get_project(&self, idx: NodeIndex) -> GPRIndex {
        match self.0[idx] {
            Node::Project(gpr) => gpr,
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
    /// Each dependency is reported only once (so if a project imports both A and
    /// B, which both import a common C, then C is only returned once).
    /// The returned value does not include start itself.
    pub fn gpr_dependencies(&self, start: NodeIndex) -> Vec<GPRIndex> {
        let mut bfs = Bfs::new(&self.0, start);
        let mut result = Vec::new();
        while let Some(node) = bfs.next(&self.0) {
            if node != start {
                result.push(self.get_project(node));
            }
        }
        result
    }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self(Graph::new())
    }
}
