use crate::gpr::GPR;
use crate::scenarios::AllScenarios;
use petgraph::graph::{Graph, NodeIndex};

#[derive(Clone, Copy)]
pub struct GPRIndex(pub usize); // index into Environment.gprs

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
    pub graph: Graph<Node, Edge>,
}
