mod directory;
mod environment;
mod errors;
mod files;
mod findfile;
mod gpr;
mod graph;
mod lexer;
mod rawexpr;
mod rawgpr;
mod scanner;
mod scenario_variables;
mod scenarios;
mod settings;
// mod sourcefile;
mod tokens;
mod values;

use crate::environment::Environment;
use std::path::Path;

fn main() {
    let mut env = Environment::default();
    if let Err(e) = env.parse_all(Path::new("/home/briot/dbc/deepblue")) {
        println!("ERROR: {}", e);
    }
}
