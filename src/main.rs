mod action_unused;
mod ada_lexer;
mod ada_scanner;
mod allscenarios;
mod base_lexer;
mod cli;
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
mod naming;
mod packagename;
mod perscenario;
mod qnames;
mod qualifiedname;
mod rawexpr;
mod rawgpr;
mod scenario_variables;
mod scenarios;
mod settings;
mod simplename;
mod sourcefile;
mod tokens;
mod values;

use crate::cli::{parse_cli, Action};
use crate::environment::Environment;
use crate::errors::Error;

fn main() -> Result<(), Error> {
    // Set RUST_LOG=debug to get the logs
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (settings, action) = parse_cli()?;
    let mut env = Environment::default();
    env.parse_all(&settings)?;

    match action {
        Action::Stats => {
            env.print_stats();
        }
        Action::Dependencies { direct_only, path } => {
            if direct_only {
                env.show_direct_dependencies(&path)?;
            } else {
                env.show_indirect_dependencies(&path)?;
            }
        }
        Action::SourceUnused(act) => {
            act.perform(&env, &settings)?;
        }
        Action::GprShow {
            gprpath,
            print_vars,
        } => {
            let gpr =
                env.get_gpr(&gprpath).expect("Project not found in graph");
            gpr.print_details(&env.scenarios, print_vars);
        }
    }

    // TODO: should simplify edges to merge scenarios when possible.  Currently,
    //    this merging is done in get_specs(), but it would be better to have it
    //    directly in the graph instead.  See scenario in get_specs()
    // TODO: support for GPR_PROJECT_PATH
    //
    // BUG: multiple units with the same name in the project tree would not
    //    work (likely the graph is correct, but since we resolve dependencies
    //    based on unit name, and not file name, we would not know which of the
    //    two units we depend on).

    // BUG: We should not be able to resolve system files, unless we use
    // --runtime. As if we were looking up dependencies from projects we do not
    // import.

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
