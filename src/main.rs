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

use crate::environment::{Environment, GPRIndex};
use crate::gpr::GPR;

pub fn parse_all(path: &Path) -> Result<Environment, Box<dyn std::error::Error>> {
    let mut env = Environment::default();

    // Prepare the indexes for the GPR files, so that we can later have the list
    // of dependencies

    for gpr in crate::findfile::FileFind::new(path) {
        env.map.insert(gpr.to_path_buf(), GPRIndex(env.gprs.len()));
        env.gprs.push(None);
    }

    // Then parse the GPR files

    for (path, idx) in &env.map {
        let file = files::File::new(path)?;
        let mut lex = lexer::Lexer::new(&file);
        let scan = scanner::Scanner::new(&mut lex);
        let rawgpr = scan.parse()?;
        let gprfile = GPR::new(&env, rawgpr);
        env.gprs[idx.0] = Some(gprfile);
    }

    println!("Parsed {} gpr files", env.map.len());

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
