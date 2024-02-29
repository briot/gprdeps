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
mod units;
mod values;

use crate::environment::Environment;
use crate::errors::Error;
use std::path::Path;

fn main() -> Result<(), Error> {
    let mut env = Environment::default();

    env.add_implicit_project(Path::new(
        "/home/briot/dbc/deepblue/External/Ada_Run_Time/adalib.gpr",
    ));

    if let Err(e) = env.parse_all(Path::new("/home/briot/dbc/deepblue")) {
        println!("ERROR: {}", e);
    }

    let path = "/home/briot/dbc/deepblue/\
        General/Networking/private/servers-sockets.adb";
    env.show_direct_dependencies(Path::new(path))?;
    env.show_indirect_dependencies(Path::new(path))?;

    // TODO:
    // should simplify edges to merge scenarios when possible.  Currently,
    // this merging is done in get_specs(), but it would be better to have
    // it directly in the graph instead.  See scenario in get_specs()

    // TODO:
    // should also parse standard runtime, otherwise we have no filenames
    // for it.

    //    let pool = threadpool::ThreadPool::new(1);
    //    for gpr in list_of_gpr {
    //        let gpr = gpr.clone();
    //        pool.execute(move || {
    //            let _ = parse_gpr_file(&gpr);
    //        });
    //    }
    //    pool.join();

    Ok(())
}
