use crate::{
    errors::Error, qnames::QName, scenarios::Scenario, sourcefile::SourceKind,
};
use petgraph::{
    algo::toposort,
    graph::Graph,
    visit::{Bfs, EdgeRef},
    Directed, Direction,
};
use std::path::PathBuf;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;

/// The nodes of a graph
#[derive(Debug)]
pub enum Node {
    Project(PathBuf),

    #[allow(dead_code)]
    Unit(QName),
    Source(PathBuf),
}

/// The edges of a graph
///  - A project might depend on other projects, to import source files
///  - A project includes zero or more source files
///  - Source files are grouped into logical units (Ada: packages), though a
///    whole unit is not necessary contained in the same project, though that is
///    a rare case in practice.
///  - A given source file could in theory be part of two different projects,
///    though for two non-overlapping scenarios.
///  - Source files import zero or more units (in C, we import a source file
///    directly, but then a unit is the same as a source file in this case).
///  - A source file depends on the parent unit implicitly.
///  - An implementation or a separate source file depend on all files from the
///    same unit.  A Spec however doesn't (so that modifying the body doesn't
///    require recompiling the spec for instance).
#[derive(Debug)]
#[allow(dead_code)]
pub enum Edge {
    GPRExtends,                         // between for project files
    GPRImports,                         // between project files
    ProjectSource(Scenario),            // from project to owned source file
    UnitSource((SourceKind, Scenario)), // from unit to owned source files
    SourceImports,                      // from source file to imported unit
}

type G = Graph<Node, Edge, Directed, u32>;

/// A unified dependency graph, for both projects and source files
pub struct DepGraph(pub G);
impl DepGraph {
    pub fn get_project(&self, idx: NodeIndex) -> Result<&PathBuf, Error> {
        match &self.0[idx] {
            Node::Project(g) => Ok(g),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }

    pub fn node_count(&self) -> usize {
        self.0.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.0.edge_count()
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
    pub fn gpr_dependencies(&self, start: NodeIndex) -> Vec<NodeIndex> {
        let mut bfs = Bfs::new(&self.0, start);
        let mut result = Vec::new();
        while let Some(node) = bfs.next(&self.0) {
            if node != start {
                result.push(node);
            }
        }
        result
    }

    // Iterate over source nodes
    //    pub fn iter_source_nodes(
    //        &self,
    //    ) -> impl Iterator<Item = (NodeIndex, &PathBuf)> + '_ {
    //        self.0
    //            .node_indices()
    //            .filter_map(|n| match self.0.node_weight(n) {
    //                Some(Node::Source(path)) => Some((n, path)),
    //                _ => None,
    //            })
    //    }

    /// Iterate over source nodes of a project
    pub fn iter_source_nodes_of_project(
        &self,
        project: NodeIndex,
    ) -> impl Iterator<Item = &PathBuf> + '_ {
        self.0
            .edges_directed(project, Direction::Outgoing)
            .filter_map(|e| match e.weight() {
                Edge::ProjectSource(_) => {
                    if let Node::Source(path) = &self.0[e.target()] {
                        Some(path)
                    } else {
                        None
                    }
                }
                _ => None,
            })
    }

    /// Iterate over project nodes
    pub fn iter_project_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeIndex, &PathBuf)> + '_ {
        self.0
            .node_indices()
            .filter_map(|n| match self.0.node_weight(n) {
                Some(Node::Project(path)) => Some((n, path)),
                _ => None,
            })
    }

    // Returns a subgraph, which only includes edges for unit and file
    // dependencies.
    // pub fn file_dep_subgraph<F>(
    //     &self,
    // ) -> petgraph::visit::EdgeFiltered<&G, F> {
    //     petgraph::visit::EdgeFiltered::from_fn(&self.0, |e| {
    //         matches!(
    //             e.weight(),
    //             Edge::SourceImports
    //                 | Edge::UnitSpec(_)
    //                 | Edge::UnitImpl(_)
    //                 | Edge::UnitSeparate(_)
    //         )
    //     })
    // }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self(Graph::new())
    }
}

impl std::fmt::Debug for DepGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}",
            petgraph::dot::Dot::with_config(
                &self.0,
                &[petgraph::dot::Config::EdgeNoLabel]
            )
        )
    }
}
