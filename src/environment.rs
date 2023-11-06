use crate::gpr::GPR;
use crate::scenarios::AllScenarios;
use petgraph::Directed;
use petgraph::graph::{Graph, NodeIndex};

#[derive(Debug)]
pub enum Node {
    Project(GPR),
    Source,
}

#[derive(Debug)]
pub enum Edge {
    Imports,
}

/// The whole set of gpr files
#[derive(Default)]
pub struct Environment {
    pub gprmap: std::collections::HashMap<std::path::PathBuf, NodeIndex>,
    pub scenarios: AllScenarios,
    pub graph: Graph<Node, Edge, Directed, u32>,
}
