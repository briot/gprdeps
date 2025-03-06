use crate::{
    environment::Environment,
    errors::Error,
    settings::Settings,
};
use std::collections::HashSet;

pub struct ActionStats {
}

impl ActionStats {

    pub fn new() -> Self {
         ActionStats {}
    }

    pub fn perform(
        &self,
        env: &Environment,
        _settings: &Settings,
    ) -> Result<(), Error> {
        env.scenarios.print_stats();

        // Display the total number of scenarios that result in different
        // values for variables.  This is however not very useful, since
        // some scenarios are more general than others.  So perhaps we
        // should only count scenarios for which no variable is "any value".
        let mut used = HashSet::new();
        env.find_used_scenarios(&mut used);
        println!("Distinct scenarios: {}", used.len());

        println!("\nGraph nodes:  {:-7}", env.graph.node_count());
        println!("   Projects:     = {:-6}", env.gprs.len());
        println!("   Units:        + {:-6}", env.units.len());
        println!("   Source files: + {:-6}", env.files.len());
        println!("Graph edges:  {:-7}", env.graph.edge_count());
        Ok(())
    }
}
