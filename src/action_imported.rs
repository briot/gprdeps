use crate::{
    environment::Environment,
    errors::Error,
    graph::{Edge, Node},
    settings::Settings,
};
use petgraph::{
    visit::{EdgeRef, Reversed, Walker},
    Direction,
};
use std::collections::HashSet;
use std::path::PathBuf;

pub enum Kind {
    ImportedBy,
    Import,
}

/// Report the list of units directly imported by the given file
pub struct ActionImported {
    pub path: PathBuf,
    pub recurse: bool,
    pub kind: Kind,
}

impl ActionImported {
    pub fn perform(
        &self,
        env: &Environment,
        settings: &Settings,
    ) -> Result<(), Error> {
        let info = env
            .files
            .get(&std::path::PathBuf::from(&self.path))
            .ok_or(Error::NotFound("File not found in graph".into()))?
            .clone();
        let file = info.borrow();

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

        let deps: HashSet<PathBuf> = match self.kind {
            Kind::ImportedBy => {
                if self.recurse {
                    petgraph::visit::Dfs::new(&filtered, file.file_node)
                        .iter(&filtered)
                        .filter_map(|node| match &env.graph.0[node] {
                            Node::Source(path) => Some(path.clone()),
                            _ => None,
                        })
                        .collect()
                } else {
                    env.graph
                        .0
                        .edges_directed(file.file_node, Direction::Outgoing)
                        .filter(|e| matches!(e.weight(), Edge::SourceImports))
                        .map(|e| e.target())
                        .flat_map(|unit| {
                            env.graph
                                .0
                                .edges_directed(unit, Direction::Outgoing)
                                .filter_map(move |e| match e.weight() {
                                    Edge::UnitSpec(_)
                                    | Edge::UnitImpl(_)
                                    | Edge::UnitSeparate(_) => {
                                        match &env.graph.0[e.target()] {
                                            Node::Source(path) => {
                                                Some(path.clone())
                                            }
                                            _ => None,
                                        }
                                    }
                                    _ => None,
                                })
                        })
                        .collect()
                }
            }
            Kind::Import => {
                if self.recurse {
                    let r = Reversed(&filtered);
                    petgraph::visit::Dfs::new(&r, file.file_node)
                        .iter(&r)
                        .filter_map(|node| match &env.graph.0[node] {
                            Node::Source(path) => Some(path.clone()),
                            _ => None,
                        })
                        .collect()
                } else {
                    env.graph
                        .0
                        .edges_directed(file.file_node, Direction::Incoming)
                        .filter(|e| {
                            matches!(
                                e.weight(),
                                Edge::UnitSpec(_)
                                    | Edge::UnitImpl(_)
                                    | Edge::UnitSeparate(_)
                            )
                        })
                        .map(|e| e.source())
                        .flat_map(|unit| {
                            env.graph
                                .0
                                .edges_directed(unit, Direction::Incoming)
                                .filter_map(move |e| match e.weight() {
                                    Edge::SourceImports => {
                                        match &env.graph.0[e.source()] {
                                            Node::Source(path) => {
                                                Some(path.clone())
                                            }
                                            _ => None,
                                        }
                                    }
                                    _ => None,
                                })
                        })
                        .collect()
                }
            }
        };

        let mut deps_vec: Vec<&PathBuf> = deps.iter().collect();
        deps_vec.sort();
        for d in deps_vec {
            println!("{}", settings.display_path(d));
        }
        Ok(())
    }
}
