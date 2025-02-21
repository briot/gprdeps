use crate::{
    environment::Environment, errors::Error, graph::NodeIndex,
    settings::Settings, sourcefile::SourceFile,
};
use petgraph::{algo::condensation, graph::Graph, Directed, Direction};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::rc::Rc;

pub struct ActionSourceUnused {
    pub unused: Vec<(PathBuf, PathBuf)>,
    pub ignore: Vec<PathBuf>,
    pub recurse: bool,
}

// A unit graph is a subset of the full dependency graph, which only includes
// some of the Unit nodes and their dependencies.  Each node's weight is a
// reference to the full dependency graph.
struct UnitNodeIndex(NodeIndex); //  node in unit graph
type UnitGraph = Graph<NodeIndex, u8, Directed, u32>;

// A condensed unit graph is similar to a unit graph, but all strongly connected
// components (aka with dependency cycles) are grouped into single nodes.
// The weights are a vec of the unit graph's node weights, i.e. they are vecs
// of the full dependency graph nodes.
// For instance:
//    pkg.io_any      depends on pkg which is the parent package
//    pkg.io_any.v1   depends on pkg.io_any for the same reason
//    pkg.io_any      depends on pkg.io_any.v1 for dispatching
// If we do not look at strongly connected components, the above is a cycle that
// means none of these packages will ever be reported as unused.
struct CondensedNodeIndex(NodeIndex); // node in condensed graph
type CondensedGraph = Graph<Vec<NodeIndex>, u8, Directed, u32>;

impl ActionSourceUnused {
    /// Report all source files that are never imported.
    /// Ignore those units that are "main" units for a project.
    /// Ignore files in specific directories (typically, third-party libraries)
    pub fn perform(
        &self,
        env: &Environment,
        settings: &Settings,
    ) -> Result<(), Error> {
        let expected = self.parse_unused_files()?;
        let expected_nodes: HashSet<NodeIndex> = env
            .files
            .values()
            .filter_map(|file| {
                let sm = file.borrow();
                if expected.contains(&sm.path) {
                    sm.unit_node
                } else {
                    None
                }
            })
            .collect();
        let ada_unit_nodes: HashSet<NodeIndex> = env
            .files
            .values()
            .filter_map(|file| {
                let sm = file.borrow();
                if sm.lang != "ada" || sm.unit_node.is_none() {
                    None
                } else {
                    Some(sm.unit_node.unwrap())
                }
            })
            .collect();
        let keepers: HashSet<NodeIndex> = self
            .keepers(env)
            .filter_map(|file| file.borrow().unit_node)
            .collect();
        let unit_graph = self.build_unit_graph(env, &ada_unit_nodes);
        let condensed: CondensedGraph = condensation(unit_graph, true);
        let unused_nodes =
            self.find_unused(condensed, &keepers, &expected_nodes);
        let paths = env.file_paths_from_units(unused_nodes.iter().cloned());

        settings.print_files(
            "\nFiles in unused.txt but not on disk",
            expected.iter().filter(|p| !p.is_file()).collect(),
        );
        settings.print_files(
            "\nUnused Ada files (not in unused.txt)",
            paths.difference(&expected).collect(),
        );
        settings.print_files(
            "\nUsed Ada files but in unused.txt",
            expected.difference(&paths).collect(),
        );

        Ok(())
    }

    /// Parse the "unused.txt" files that lists files that we know are unused.
    fn parse_unused_files(&self) -> Result<HashSet<PathBuf>, Error> {
        let mut unused = HashSet::new();
        for (filename, root) in &self.unused {
            unused.extend(
                io::BufReader::new(File::open(filename)?)
                    .lines()
                    .map_while(Result::ok)
                    .filter(|line|
                        matches!(line.chars().next(), Some(c) if c != '#'))
                    .map(|line| root.join(line))
            );
        }
        Ok(unused)
    }

    /// Compute the list of files we should never report as unused.
    /// This includes main units, library interfaces, as well as files in
    /// specific directories (e.g. third party libraries)
    fn keepers<'a>(
        &'a self,
        env: &'a Environment,
    ) -> impl Iterator<Item = &'a Rc<RefCell<SourceFile>>> {
        env.files.values().filter(|file| {
            let sm = file.borrow();
            sm.is_ever_main
                || sm.is_library_interface
                || self.ignore.iter().any(|ign| sm.path.starts_with(ign))
        })
    }

    /// Build a subset of the dependency graph which only includes the Unit
    /// nodes.
    fn build_unit_graph(
        &self,
        env: &Environment,
        unit_nodes: &HashSet<NodeIndex>,
    ) -> UnitGraph {
        let mut unit_graph = UnitGraph::new();
        let map: HashMap<NodeIndex, UnitNodeIndex> = unit_nodes
            .iter()
            .map(|u| (*u, UnitNodeIndex(unit_graph.add_node(*u))))
            .collect();
        for (parent, child) in env.iter_unit_deps(unit_nodes.iter().cloned()) {
            if let Some(parent_u) = map.get(&parent) {
                unit_graph.add_edge(parent_u.0, map[&child].0, 0);
            }
        }
        unit_graph
    }

    /// Find unused nodes in a condensed graph.
    /// Typically, the node's weights in the condensed graph will be node
    /// indices in the full dependency graph (N).
    /// This is done recursively: we first find unused root nodes (withouth
    /// incoming edges), then remove those, and keep searching recursively.
    ///
    /// None of the nodes in keepers will be removed.
    fn find_unused<N, E>(
        &self,
        mut graph: Graph<Vec<N>, E, Directed, u32>,
        keepers: &HashSet<N>,
        expected_unused: &HashSet<N>,
    ) -> Vec<N>
    where
        N: Clone + Eq + PartialEq + std::hash::Hash,
    {
        let mut unused_nodes: Vec<N> = Vec::new();
        loop {
            let roots: Vec<CondensedNodeIndex> = graph
                .node_indices()
                .filter(|n| {
                    graph
                        .edges_directed(*n, Direction::Incoming)
                        .next()
                        .is_none()

                    // We must look at the weight(), since the indices in
                    // condensed change when we remove nodes.
                    && !graph.node_weight(*n).unwrap().iter().any(
                        |from_unit_graph| keepers.contains(from_unit_graph),
                    )
                })
                .map(CondensedNodeIndex)
                .collect();

            let mut to_remove: Vec<CondensedNodeIndex> = Vec::new();
            for n in roots.into_iter() {
                let mut must_remove = true;
                for orig in graph.node_weight(n.0).unwrap().iter() {
                    unused_nodes.push(orig.clone());
                    if !self.recurse && expected_unused.contains(orig) {
                        must_remove = false;
                    }
                }
                if must_remove {
                    to_remove.push(n);
                }
            }

            if to_remove.is_empty() {
                return unused_nodes;
            }

            // This changes node indices
            for n in &to_remove {
                graph.remove_node(n.0);
            }
        }
    }
}
