mod ada_lexer;
mod ada_scanner;
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
mod packagename;
mod perscenario;
mod rawexpr;
mod rawgpr;
mod scenario_variables;
mod scenarios;
mod settings;
mod simplename;
mod sourcefile;
mod tokens;
mod units;
mod values;

use crate::cli::{parse_cli, Action};
use crate::environment::Environment;
use crate::errors::Error;

fn main() -> Result<(), Error> {
    let (settings, action) = parse_cli()?;
    let mut env = Environment::default();

    match action {
        Action::Stats => {
            env.parse_all(&settings.root, &settings, true)?;
            env.print_stats();
        }
        Action::Dependencies { direct_only, path } => {
            env.parse_all(&settings.root, &settings, true)?;
            if direct_only {
                env.show_direct_dependencies(&path)?;
            } else {
                env.show_indirect_dependencies(&path)?;
            }
        }
        Action::GprShow {
            gprpath,
            print_vars,
        } => {
            env.parse_all(&settings.root, &settings, false)?;
            let gpr =
                env.get_gpr(&gprpath).expect("Project not found in graph");
            gpr.print_details(&env.scenarios, print_vars);
        }
    }

    // TODO: should simplify edges to merge scenarios when possible.  Currently,
    //    this merging is done in get_specs(), but it would be better to have it
    //    directly in the graph instead.  See scenario in get_specs()
    // TODO: support for --root as a gpr project, and only load its deps
    // TODO: support for GPR_PROJECT_PATH
    // TODO: instead of trimming projects, just don't insert attributes
    //    This fails on general.gpr, because a variable references an attribute
    //    that we are not keeping.
    // TODO: improve performance for scenario: should use a bitmask to known
    //    which values are valid, then we can use "&", "|" and "!" to intersect
    //    multiple scenarios.

    // BUG:
    // scenarios for valgrind unit are wrong.  We get
    //    checking=off,tasking=on    and checking=on,tasking=off
    // when it should be for all scenarios
    // Similar for task_initialization:
    //    checking=off   /  tasking=off  / checking=on,tasking=on

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
