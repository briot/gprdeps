use crate::allscenarios::AllScenarios;
use crate::errors::Error;
use crate::qnames::QName;
use crate::scenarios::Scenario;
use petgraph::algo::toposort;
use petgraph::graph::Graph;
use petgraph::visit::Bfs;
use petgraph::Directed;
use std::collections::HashMap;
use std::path::PathBuf;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;

/// The nodes of a graph
#[derive(Debug)]
pub enum Node {
    Project(PathBuf),
    Unit(QName),
    Source(PathBuf), //  ??? Should be UStr
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
}

/// A unified dependency graph, for both projects and source files
pub struct DepGraph(pub Graph<Node, Edge, Directed, u32>);
impl DepGraph {
    pub fn get_project(&self, idx: NodeIndex) -> Result<&PathBuf, Error> {
        match &self.0[idx] {
            Node::Project(g) => Ok(g),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }

    pub fn get_unit(&self, idx: NodeIndex) -> Result<QName, Error> {
        match &self.0[idx] {
            Node::Unit(qname) => Ok(qname.clone()),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }

    pub fn get_source(&self, idx: NodeIndex) -> Result<&PathBuf, Error> {
        match &self.0[idx] {
            Node::Source(path) => Ok(path),
            u => Err(Error::InvalidGraphNode(format!("{:?}", u))),
        }
    }

    /// Given a unit, returns the list of specification files for it
    pub fn get_specs(
        &self,
        _all_scenarios: &mut AllScenarios,
        idx: NodeIndex,
    ) -> HashMap<NodeIndex, Vec<Scenario>> {
        let specs = HashMap::new();

        if let Node::Unit(_) = &self.0[idx] {
            for e in self.0.edges(idx) {
                match e.weight() {
                    Edge::ProjectSource(_scenar) => {}
                    Edge::UnitImports(_scenar) => {}
                    Edge::UnitSpec(_scenar)
                    | Edge::UnitImpl(_scenar)
                    | Edge::UnitSeparate(_scenar) => {
                        // // If we already have the same node in the vector, we
                        // // merge the scenarios
                        // let target = e.target();
                        // match specs.get_mut(&target) {
                        //     None => {
                        //         specs.insert(target, vec![*scenar]);
                        //     }
                        //     Some(scenarios) => {
                        //         let mut merged = false;
                        //         for s in scenarios.iter_mut() {
                        //             if let Some(s2) =
                        //                 all_scenarios.union(*s, *scenar)
                        //             {
                        //                 *s = s2;
                        //                 merged = true
                        //             }
                        //         }
                        //         if !merged {
                        //             scenarios.push(*scenar);
                        //         } else {
                        //             // If we have only two scenarios left, and we
                        //             // did not just add the second one, we should
                        //             // try and merge them.  Otherwise, we might
                        //             // have the following case:
                        //             //   insert checks=off,tasking=off
                        //             //   insert checks=on,tasking=on
                        //             //   insert checks=on,tasking=off
                        //             //      => tasking=off + checks=on,tasking=on
                        //             //   insert checks=off,tasking=on
                        //             //      =>  tasking=off + tasking=on
                        //             // and the last two scenarios could actually
                        //             // be merged now.
                        //
                        //             // ??? This will not be sufficient, and union()
                        //             // should have a better algorithm instead.
                        //
                        //             if scenarios.len() == 2 {
                        //                 if let Some(s2) = all_scenarios
                        //                     .union(scenarios[0], scenarios[1])
                        //                 {
                        //                     scenarios.clear();
                        //                     scenarios.push(s2);
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    }

                    _ => {}
                }
            }
        }
        specs
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
