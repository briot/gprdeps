/// Project data can vary based on the values of one or more variables.
/// These variables (named "scenario variables") are typed (so can only take
/// a specific set of values), and can be tested in case statements.
/// When we parse project files, we evaluate all scenarios simultaneously.
use crate::perscenario::PerScenario;
use crate::rawexpr::WhenClause;
use crate::scenario_variables::ScenarioVariable;
use crate::scenarios::{Scenario, ScenarioFactory};
use crate::simplename::StringOrOthers;
use itertools::join;
use ustr::{Ustr, UstrMap};

/// Keeps the current state of a case statement.
/// This involves keeping track of what "when" clauses have been seen, so we
/// can flag when we have duplicates or missing choices.
#[derive(Debug, Clone)]
pub struct CaseStmtScenario {
    var: Ustr,
    // Name of the environment variable
    // ??? Could be directly a &ScenarioVariable
    full_mask: Scenario,
    // A mask that covers all possible values for the variable
    remaining: Scenario,
    // The bitmask that lists all values of the variable not yet covered by
    // a WhenClause.
}

/// The collection of all variants of scenarios needed to analyze the project
/// tree.  Each scenario is unique.
#[derive(Default)]
pub struct AllScenarios {
    variables: UstrMap<ScenarioVariable>,
    factory: ScenarioFactory,
}

impl AllScenarios {
    /// True if this scenario is not applicable (cannot occur in practice).
    /// This is the case if for at least one of the variables the mask is 0.
    pub fn never_matches(&self, scenario: Scenario) -> bool {
        self.variables
            .values()
            .any(|var| (scenario & var.full_mask()) == Scenario::empty())
    }

    /// Negate the scenario: it returns a series of possibly overlapping
    /// scenarios so that they do not overlap scenario, yet the whole set
    /// covers the full space.
    /// For instance, if we have three variables E1 (with values a,b,c,d),
    /// E2 (with values e and f) and E3 (with values g and h), and we give
    /// a scenario "E1=a|b,E2=e,E3=*"
    /// Then scenarios are represented as
    ///    b 0000 00 00    (E1, E2, E3)
    /// The full masks for the variables are
    ///    E1:  1111 00 00
    ///    E2:  0000 11 00
    ///    E3:  0000 00 11
    /// So our specific input scenario is
    ///    b 1100 10 11
    /// So the negation is
    ///    not(a|b and e) = not(a|b) or not(e) = (E1=c|d) or (E2=f)
    ///    b 0011 11 11    # E1=c|d, E2=*, E3=*
    ///    b 1111 01 11    # E1=*,   E2=f, E3=*
    pub fn negate(
        &self,
        scenario: Scenario,
    ) -> impl std::iter::Iterator<Item = Scenario> + '_ {
        let mask = scenario;
        self.variables.values().filter_map(move |var| {
            let negate_var_only = !mask & var.full_mask();
            let all_other_vars = !var.full_mask();
            if !negate_var_only.is_empty() {
                Some(all_other_vars | negate_var_only)
            } else {
                None
            }
        })
    }

    /// Prepares the handling of a Case Statement in a project file.
    /// From
    ///     V : Type := external ("VAR");
    ///     case V is
    /// we need to find the declaration of V, which will be an
    ///     values::ExprValur::Str(per_scenario)
    /// where per_scenario is itself a hashmap mapping scenarios to the
    /// corresponding value of V.  Since we start with a simple external
    /// variable, each scenario will only reference a single scenario variable
    /// ("VAR" in this example).
    pub fn prepare_case_stmt(
        &self,
        variable_values: &PerScenario<Ustr>,
    ) -> CaseStmtScenario {
        let scenar_and_varname = variable_values
            .iter()
            .next()
            .expect("Must have at least one possible value");

        // Check that the value depends on exactly one scenario variable.
        let mut mask = Scenario::default();
        for (s, _) in variable_values.iter() {
            mask = mask & *s;
        }

        // Special case: the value is in fact independent of scenarios,
        // for instance Prj'Target
        if mask == Scenario::default() {
            return CaseStmtScenario {
                var: *scenar_and_varname.1,
                full_mask: Scenario::empty(),
                remaining: Scenario::empty(),
            };
        }

        for v in self.variables.values() {
            if (mask | v.full_mask()) == Scenario::default() {
                return CaseStmtScenario {
                    var: *v.name(),
                    full_mask: v.full_mask(),
                    remaining: v.full_mask(),
                };
            }
        }

        panic!(
            "Variable used in case statement should have a simple \
             value that depends on exactly one scenario variable, \
             but got {:?}",
            self.describe(*scenar_and_varname.0),
        );
    }

    /// Combine a new when clause with the current case statement.
    /// Returns None if the when clause can never be active (e.g. we have a
    /// "when others" that doesn't match anything)
    pub fn process_when_clause(
        &mut self,
        context: Scenario,
        case_stmt: &mut CaseStmtScenario,
        when: &WhenClause,
    ) -> Scenario {
        // If the expression in Case was a constant, check whether this
        // WhenClause would be used.  In this case, case_stmt.var is the
        // actual value of the expression.
        if case_stmt.full_mask.is_empty() {
            for val in &when.values {
                match val {
                    StringOrOthers::Str(value_in_when) => {
                        if *value_in_when == case_stmt.var {
                            return context;
                        }
                    }
                    StringOrOthers::Others => {
                        return context;
                    }
                }
            }
            Scenario::empty()
        } else {
            let mut mask = Scenario::empty();
            let var = self.variables.get(&case_stmt.var).unwrap();
            let all_other_vars = !var.full_mask();
            for val in &when.values {
                match val {
                    StringOrOthers::Str(value_in_when) => {
                        let m = var.mask(value_in_when);
                        mask = mask | m;
                        case_stmt.remaining = case_stmt.remaining & !m;
                    }
                    StringOrOthers::Others => {
                        mask = case_stmt.remaining;
                        case_stmt.remaining = Scenario::empty();
                    }
                }
            }
            all_other_vars | (context & mask)
        }
    }

    /// Declares a new scenario variables and the list of all values it can
    /// accept.  If the variable is already declared, check that we are
    /// declaring the same set of values.
    /// The list of values must be sorted.
    pub fn try_add_variable<'a>(
        &'a mut self,
        name: Ustr,
        valid: &[Ustr],
    ) -> &'a ScenarioVariable {
        self.variables
            .entry(name)
            .and_modify(|v| {
                if !v.has_same_valid(valid) {
                    panic!(
                        "Scenario variable {} defined multiple times with \
                         different types {:?} and {}",
                        name,
                        valid,
                        v.describe(Scenario::default()),
                    );
                }
            })
            .or_insert_with(|| {
                let mut full_mask = Scenario::empty();
                let values: Vec<(Ustr, Scenario)> = valid
                    .iter()
                    .map(|v| {
                        let s = self.factory.get_next();
                        let res = (*v, s);
                        full_mask = full_mask | s;
                        res
                    })
                    .collect();

                ScenarioVariable::new(name, values, full_mask)
            })
    }

    /// Print statistics about scenario variables
    pub fn print_stats(&self) {
        println!("Scenario vars:{:-7}", self.variables.len());
        let total_valid: usize =
            self.variables.values().map(|v| v.count_valid()).sum();
        println!("    values:   {:-7}", total_valid);
    }

    pub fn describe(&self, scenario: Scenario) -> String {
        // Sort display, for tests
        let mut vars = self.variables.iter().collect::<Vec<_>>();
        vars.sort_by_key(|(name, _)| *name);
        join(vars.iter().map(|(_, v)| v.describe(scenario)), ",")
    }
}

#[cfg(test)]
pub mod tests {
    use crate::allscenarios::AllScenarios;
    use crate::errors::Error;
    use crate::scenarios::Scenario;
    use ustr::Ustr;

    /// Create a scenario that involves a single variable, and a set of
    /// valid values for it.
    pub fn create_single(
        scenarios: &mut AllScenarios,
        varname: &str,
        values: &[&str],
    ) -> Scenario {
        let var = scenarios.variables.get(&Ustr::from(varname)).unwrap();
        let mut mask = !var.full_mask();
        for v in values {
            mask = mask | var.mask(&Ustr::from(v));
        }
        mask
    }

    pub fn try_add_variable(
        scenarios: &mut AllScenarios,
        name: &str,
        valid: &[&str],
    ) {
        scenarios.try_add_variable(
            Ustr::from(name),
            &valid.iter().map(|s| Ustr::from(s)).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn create_scenario() -> Result<(), Error> {
        let mut scenarios = AllScenarios::default();
        try_add_variable(&mut scenarios, "MODE", &["debug", "lto", "optimize"]);

        let s0 = Scenario::default();
        assert_eq!(scenarios.describe(s0), "MODE=*");

        //  case Mode is
        //     when "debug" => ...
        let s2 = create_single(&mut scenarios, "MODE", &["debug"]);
        assert_eq!(scenarios.describe(s2), "MODE=debug");

        //  when others  => for Source_Dirs use ("src1", "src3");
        //     case Check is
        let s3 = create_single(&mut scenarios, "MODE", &["lto", "optimize"]);
        assert_eq!(scenarios.describe(s3), "MODE=lto|optimize");

        let same = create_single(&mut scenarios, "MODE", &["lto", "optimize"]);
        assert_eq!(same, s3);

        // Create a second variable later
        try_add_variable(&mut scenarios, "CHECK", &["most", "none", "some"]);

        let check_most = create_single(&mut scenarios, "CHECK", &["most"]);
        let s4 = s3 & check_most;
        assert_eq!(scenarios.describe(s4), "CHECK=most,MODE=lto|optimize");

        let check_none_some =
            create_single(&mut scenarios, "CHECK", &["none", "some"]);
        let s5 = s3 & check_none_some;
        assert_eq!(scenarios.describe(s5), "CHECK=none|some,MODE=lto|optimize");

        //   case Check is
        //      when "none" => for Excluded_Source_Files use ("a.ads");
        let check_none = create_single(&mut scenarios, "CHECK", &["none"]);
        assert_eq!(scenarios.describe(check_none), "CHECK=none,MODE=*");

        //      when others => null;
        let s7 = create_single(&mut scenarios, "CHECK", &["most", "none"]);
        assert_eq!(scenarios.describe(s7), "CHECK=most|none,MODE=*");

        Ok(())
    }

    #[test]
    fn test_intersection() -> Result<(), Error> {
        let mut scenarios = AllScenarios::default();
        try_add_variable(&mut scenarios, "MODE", &["debug", "lto", "optimize"]);
        try_add_variable(&mut scenarios, "CHECK", &["most", "none", "some"]);
        let s0 = Scenario::default();

        // s0=everything
        // s1=MODE=debug
        //    => s1
        let s1 = create_single(&mut scenarios, "MODE", &["debug"]);
        let res = s0 & s1;
        assert_eq!(res, s1);
        let res = s1 & s0;
        assert_eq!(res, s1);

        // s1=MODE=debug
        // s2=MODE=debug,CHECK=some
        //    => s2
        let check_some = create_single(&mut scenarios, "CHECK", &["some"]);
        let s2 = s1 & check_some;
        let res = s1 & s2;
        assert_eq!(res, s2);
        let res = s2 & s1; // reverse order
        assert_eq!(res, s2);

        // s2=MODE=debug,CHECK=some
        // s3=CHECK=none|some
        //    => s2=MODE=debug,CHECK=some
        let check_none_some =
            create_single(&mut scenarios, "CHECK", &["none", "some"]);
        let res = s2 & check_none_some;
        assert_eq!(res, s2);
        let res = check_none_some & s2; // reverse order
        assert_eq!(res, s2);

        // s4=MODE=debug|optimize,CHECK=some
        // s5=MODE=lto|optimize,CHECK=some|most
        //    =>  s6=MODE=optimize,CHECK=some
        let mode_debug_opt =
            create_single(&mut scenarios, "MODE", &["debug", "optimize"]);
        let s4 = mode_debug_opt & check_some;
        let mode_lto_opt =
            create_single(&mut scenarios, "MODE", &["lto", "optimize"]);
        let check_some_most =
            create_single(&mut scenarios, "CHECK", &["some", "most"]);
        let s5 = mode_lto_opt & check_some_most;
        let mode_opt = create_single(&mut scenarios, "MODE", &["optimize"]);
        let s6 = mode_opt & check_some;
        let res = s4 & s5;
        assert_eq!(res, s6);
        let res = s5 & s4; // reverse order
        assert_eq!(res, s6);

        Ok(())
    }
}
