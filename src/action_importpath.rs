use crate::{
    environment::Environment,
    errors::Error,
    graph::{Edge, Node},
    settings::Settings,
};
use petgraph::algo::astar;
use std::path::PathBuf;

pub struct ActionImportPath {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl ActionImportPath {
    pub fn perform(
        &self,
        env: &Environment,
        settings: &Settings,
    ) -> Result<(), Error> {
        let source = env
            .files
            .get(&std::path::PathBuf::from(&self.source))
            .ok_or(Error::NotFound("File not found in graph".into()))?
            .clone();
        let target = env
            .files
            .get(&std::path::PathBuf::from(&self.target))
            .ok_or(Error::NotFound("File not found in graph".into()))?
            .clone();

        let source = source.borrow();
        let target = target.borrow();

        // A subgraph only taking some of the edges into account
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&env.graph.0, |e| {
                matches!(
                    e.weight(),
                    Edge::SourceImports
                        | Edge::UnitSpec(_)
                        | Edge::UnitImpl(_)
                        | Edge::UnitSeparate(_)
                )
            });

        let path = astar(
            &filtered,
            source.file_node,          // start
            |n| n == target.file_node, // is_goal
            |_| 1,                     // edge_cost
            |_| 0,                     // estimate_cost
        );

        match path {
            Some((_, path)) => {
                for p in path {
                    match &env.graph.0[p] {
                        Node::Source(path) => {
                            println!("{}", settings.display_path(path));
                        }
                        Node::Unit(_) => {}
                        Node::Project(_) => unreachable!(),
                    }
                }
            }
            None => println!("There was no path"),
        }
        Ok(())
    }
}
