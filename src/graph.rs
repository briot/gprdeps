use crate::errors::Error;
use crate::scenarios::Scenario;
use crate::units::QualifiedName;
use petgraph::algo::toposort;
use petgraph::graph::Graph;
use petgraph::visit::{Bfs, EdgeRef};
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
#[derive(Debug)]
pub enum Node {
    Project(GPRIndex),
    Unit(QualifiedName),
    Source(std::path::PathBuf),   //  ??? Should be UStr
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
pub enum Edge {
    GPRExtends,              // for project files
    GPRImports,              // between project files
    ProjectSource(Scenario), // from project to source file
    UnitSpec(Scenario),      // from unit to source files
    UnitImpl(Scenario),      // from unit to source files
    UnitSeparate(Scenario),  // from unit to source files
    SourceImports, // from source file to unit (??? should depend on scenario)
    UnitImports(Scenario), // from unit to unit
                   // duplicates the SourceImporte edges
}

/// A unified dependency graph, for both projects and source files
pub struct DepGraph(pub Graph<Node, Edge, Directed, u32>);
impl DepGraph {
    pub fn get_project(&self, idx: NodeIndex) -> GPRIndex {
        match self.0[idx] {
            Node::Project(gpr) => gpr,
            _ => panic!("Invalid project reference {:?}", idx),
        }
    }

    pub fn get_unit_name(
        &self,
        idx: NodeIndex,
    ) -> Result<QualifiedName, Error> {
        match &self.0[idx] {
            Node::Unit(qname) => Ok(qname.clone()),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }

    pub fn get_source_path(
        &self,
        idx: NodeIndex,
    ) -> Result<&std::path::PathBuf, Error> {
        match &self.0[idx] {
            Node::Source(path) => Ok(path),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }


    /// Given a unit, returns the list of specification files for it
    pub fn get_specs(&self, idx: NodeIndex) -> Vec<(NodeIndex, Scenario)> {
        match &self.0[idx] {
            Node::Unit(_) => self
                .0
                .edges(idx)
                .filter_map(|e| match e.weight() {
                    Edge::UnitSpec(s) => Some((e.target(), *s)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.0.node_count()
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
