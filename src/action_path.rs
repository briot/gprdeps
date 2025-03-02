use crate::{
    environment::Environment,
    errors::Error,
    graph::{Node, NodeIndex},
    settings::Settings,
};
use petgraph::algo::astar;
use std::path::{Path, PathBuf};

pub struct ActionPath {
    pub source: PathBuf,
    pub target: PathBuf,
    pub show_units: bool,
}

impl ActionPath {
    fn find_node(env: &Environment, path: &Path) -> Result<NodeIndex, Error> {
        match env.files.get(path) {
            Some(src) => Ok(src.borrow().file_node),
            None => match env.gprs.get(path) {
                Some(gpr) => Ok(gpr.node),
                None => Err(Error::NotFound(format!(
                    "Not found in graph {}",
                    path.display()
                ))),
            },
        }
    }

    pub fn perform(
        &self,
        env: &Environment,
        settings: &Settings,
    ) -> Result<(), Error> {
        let source = ActionPath::find_node(env, &self.source)?;
        let target = ActionPath::find_node(env, &self.target)?;
        let path = astar(
            &env.graph.0,
            source,          // start
            |n| n == target, // is_goal
            |_| 1,           // edge_cost
            |_| 0,           // estimate_cost
        );

        match path {
            Some((_, path)) => {
                for p in path {
                    match &env.graph.0[p] {
                        Node::Source(path) => {
                            println!("file: {}", settings.display_path(path));
                        }
                        Node::Unit(qname) => {
                            if self.show_units {
                                println!("unit: {}", qname);
                            }
                        }
                        Node::Project(path) => {
                            println!("gpr: {}", settings.display_path(path));
                        }
                    }
                }
            }
            None => println!("There was no path"),
        }
        Ok(())
    }
}
