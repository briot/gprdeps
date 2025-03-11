use crate::{
    environment::Environment,
    errors::Error,
    graph::{Edge, Node},
    scenarios::Scenario,
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

        let for_scenario = settings.cli_scenario(&env.scenarios)?;
        if for_scenario != Scenario::default() {
            println!(
                "Limit result to {}",
                env.scenarios.describe(for_scenario)
            );
        }

        // A subgraph only taking some of the edges into account
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&env.graph.0, |e| {
                match e.weight() {
                    Edge::SourceImports => true,
                    Edge::UnitSource((_, s)) => {
                        !env.scenarios.never_matches(s & for_scenario)
                    }
                    _ => false,
                }
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
                                    Edge::UnitSource((_, s)) => {
                                        if env
                                            .scenarios
                                            .never_matches(s & for_scenario)
                                        {
                                            None
                                        } else {
                                            match &env.graph.0[e.target()] {
                                                Node::Source(path) => {
                                                    Some(path.clone())
                                                }
                                                _ => None,
                                            }
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
                        .filter(|e| match e.weight() {
                            Edge::UnitSource((_, s)) => {
                                !env.scenarios.never_matches(s & for_scenario)
                            }
                            _ => false,
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
