mod ada_lexer;
mod ada_scanner;
mod base_lexer;
mod cpp_lexer;
mod cpp_scanner;
mod directory;
mod environment;
mod errors;
mod files;
mod findfile;
mod gpr;
mod gpr_scanner;
mod graph;
mod rawexpr;
mod rawgpr;
mod scenario_variables;
mod scenarios;
mod settings;
mod sourcefile;
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
