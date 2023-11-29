pub mod directory;
pub mod environment;
pub mod errors;
pub mod files;
pub mod findfile;
pub mod gpr;
pub mod graph;
pub mod lexer;
pub mod rawexpr;
pub mod rawgpr;
pub mod scanner;
pub mod scenario_variables;
pub mod scenarios;
pub mod settings;
pub mod tokens;
pub mod values;

use crate::environment::Environment;
use std::path::Path;

fn main() {
    let mut env = Environment::default();
    if let Err(e) = env.parse_all(Path::new("/home/briot/dbc/deepblue")) {
        println!("ERROR: {}", e);
    }
}
