use std::path::Path;

pub mod environment;
pub mod errors;
pub mod expressions;
pub mod files;
pub mod findfile;
pub mod gpr;
pub mod lexer;
pub mod rawexpr;
pub mod rawgpr;
pub mod scanner;
pub mod scenario_variables;
pub mod scenarios;
pub mod tokens;

use crate::environment::{Environment, Node, Edge};
use crate::gpr::GPR;

pub fn parse_all(
    path: &Path,
) -> Result<Environment, Box<dyn std::error::Error>> {
    let mut env = Environment::default();
    let mut rawgprs = Vec::new();

    // Parse the GPR files, but do not analyze them yet.

    for gpr in crate::findfile::FileFind::new(path) {
        let path = gpr.to_path_buf();
        let file = files::File::new(&path)?;
        let mut lex = lexer::Lexer::new(&file);
        let scan = scanner::Scanner::new(&mut lex);

        // Parse the raw gpr file.  This still depends on lex
        let rawgpr = scan.parse()?;

        // Prepare the final GPR file, without analyzing for now
        let gprfile = GPR::new(&rawgpr);

        // Insert into the graph, so that we know the index for this file
        let idx = env.graph.add_node(Node::Project(gprfile));
        rawgprs.push((idx, rawgpr));
        env.gprmap.insert(gpr.to_path_buf(), idx);
    }

    // Process the dependencies between projects.
    // We'll need to parse them in the correct order so that files included by
    // others are already parsed when we reference their attributes.

    let mut edges = Vec::new();
    for (idx, rawgpr) in &rawgprs {
        let node = &mut env.graph[*idx];
        match node {
            Node::Project(ref mut p) => {
                p.resolve_deps(&env.gprmap, rawgpr);
                for dep in &p.imported {
                    edges.push((*idx, *dep)); // , Edge::Imports));
                }
            },
            _ => panic!("Project node found"),
        }
    }
    for (from, to) in edges {
       env.graph.add_edge(from, to, Edge::Imports);
    }

    // Then parse the GPR files.
    for rawgpr in &rawgprs {
    }

    println!("Parsed {} gpr files", env.gprmap.len());

    //    let pool = threadpool::ThreadPool::new(1);
    //    for gpr in list_of_gpr {
    //        let gpr = gpr.clone();
    //        pool.execute(move || {
    //            let _ = parse_gpr_file(&gpr);
    //        });
    //    }
    //    pool.join();
    Ok(env)
}

fn main() {
    if let Err(e) = parse_all(Path::new("/home/briot/dbc/deepblue")) {
        println!("ERROR: {}", e);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_tree() {
    }

}
