/// Project data can vary based on the values of one or more variables.
/// These variables (named "scenario variables") are typed (so can only take
/// a specific set of values), and can be tested in case statements.
/// When we parse project files, we evaluate all scenarios simultaneously.
use crate::errors::Error;
use crate::perscenario::PerScenario;
use crate::rawexpr::WhenClause;
use crate::scenario_variables::ScenarioVariable;
use crate::simplename::StringOrOthers;
use crate::values::ExprValue;
use std::collections::HashSet;
use ustr::{Ustr, UstrMap};

/// Keeps the current state of a case statement.
/// This involves keeping track of what "when" clauses have been seen, so we
/// can flag when we have duplicates or missing choices.
#[derive(Debug, Clone)]
pub struct CaseStmtScenario {
    var: Ustr,
    // Name of the environment variable
    // ??? Could be directly a &ScenarioVariable
    full_mask: u64,
    // A mask that covers all possible values for the variable
    remaining: u64,
    // The bitmask that lists all values of the variable not yet covered by
    // a WhenClause.
}

/// Describes the set of scenario variables covered by a scenario.  For each
/// known scenario variables, we either have:
///    * no entry in vars: all values of the variables are valid
///    * a bitmask that indicates which values are allowed in this scenario.
#[derive(Default, PartialEq, Clone)]
struct ScenarioDetails {
    vars: UstrMap<u64>, // Variable name => bitmak of valid values
                        //    hash: u64,
}

/// A pointer to a specific scenario.
/// The default is a scenario that allows all values for all variables
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Scenario(usize);

impl std::fmt::Display for Scenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s{}", self.0)
    }
}

/// The collection of all variants of scenarios needed to analyze the project
/// tree.  Each scenario is unique.
pub struct AllScenarios {
    variables: HashSet<ScenarioVariable>,
    scenarios: Vec<ScenarioDetails>, // indexed by Scenario
}

impl std::fmt::Debug for AllScenarios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (s, _) in self.scenarios.iter().enumerate() {
            write!(f, "{}=({}) ", s, self.describe(Scenario(s)))?
        }
        Ok(())
    }
}

impl Default for AllScenarios {
    fn default() -> Self {
        let mut s = Self {
            variables: Default::default(),
            scenarios: Default::default(),
        };
        s.scenarios.push(ScenarioDetails::default()); //  Scenario::Default
        s
    }
}

impl AllScenarios {
    /// Check if we already have a similar scenario, or create a new one
    fn create_or_reuse(&mut self, details: ScenarioDetails) -> Scenario {
        for (idx, candidate) in self.scenarios.iter().enumerate() {
            if *candidate == details {
                return Scenario(idx);
            }
        }
        self.scenarios.push(details);
        Scenario(self.scenarios.len() - 1)
    }

    /// Create a scenario that involves a single variable, and a set of
    /// valid values for it.
    pub fn create_single(&mut self, varname: Ustr, mask: u64) -> Scenario {
        let mut details = ScenarioDetails::default();
        details.vars.insert(varname, mask);
        self.create_or_reuse(details)
    }

    /// Negate the scenario: it returns a series of possibly overlapping
    /// scenarios so that they do not overlap scenario, yet the whole set
    /// covers the full space.
    /// For instance, if we have three variables E1 (with values a,b,c,d),
    /// E2 (with values e and f) and E3 (with values g and h), and we give
    /// a scenario "E1=a|b,E2=e", this will return
    ///     not(a|b and e) = not(a|b) or not(e) = (E1=c|d) or (E2=f)
    pub fn negate(&mut self, scenario: Scenario) -> Vec<Scenario> {
        let mut all_negate = vec![];
        for (varname, mask) in &self.scenarios[scenario.0].vars {
            let var = self.variables.get(varname).unwrap();
            let m = !mask & var.full_mask();
            if m != 0 {
                all_negate.push((*varname, m));
            }
        }
        all_negate
            .into_iter()
            .map(|(varname, mask)| self.create_single(varname, mask))
            .collect()
    }

    /// Splits a scenario along one specific variable.
    /// This is used for when-clauses.  For instance, assume we have the
    /// following
    ///     type T is ("a", "b", "c", "d");
    ///     E1 : T := external("E1");
    ///     E2 : T := external("E2");
    ///
    /// And s1 is the scenario for "E1=a|b" (for instance it is used for the
    /// value of a variable).
    ///
    /// When the project file contains
    ///    case E1 is
    ///       when "a" =>
    /// Then we split s1 long "E1=a".  Which means we compute
    ///     s1 and "E1=a"     =>  E1=a
    ///     s1 and not "E1=a" =>  E1=b
    ///
    /// If the project file contains
    ///    case E1 is
    ///        when "a" | "b" | "c" =>
    /// Then we split and get
    ///     s1 and "E1=a|b|c"     => E1=a|b
    ///     s1 and not "E1=a|b|c" => empty
    ///
    /// If we have s2="E2=a|b", and we split along "E1=a" then we get:
    ///     s2 and "E1=a"      => E1=a,    E2=a|b
    ///     s2 and not "E1="   => E1=b|c|d E=a|b
    pub fn intersection(
        &mut self,
        s1: Scenario,
        s2: Scenario,
    ) -> Option<Scenario> {
        if s1 == s2 {
            return Some(s1);
        }
        let d1 = &self.scenarios[s1.0];
        let d2 = &self.scenarios[s2.0];

        let mut d = ScenarioDetails::default();
        for var in &self.variables {
            let n = var.name();
            match (d1.vars.get(n), d2.vars.get(n)) {
                (None, None) => {
                    // both scenario allow all values
                }
                (None, Some(&v2)) => {
                    d.vars.insert(*n, v2);
                }
                (Some(&v1), None) => {
                    d.vars.insert(*n, v1);
                }
                (Some(&v1), Some(&v2)) => {
                    //  We can end up with an empty set of possible
                    //  values.  In this case, the intersection is
                    //  empty.
                    let v = v1 & v2;
                    if (v & var.full_mask()) == 0 {
                        return None;
                    }
                    d.vars.insert(*n, v1 & v2);
                }
            }
        }
        Some(self.create_or_reuse(d))
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
        let scenar_var = variable_values
            .iter()
            .next()
            .expect("Must have at least one possible value");
        let details = &self.scenarios[scenar_var.0 .0];

        match details.vars.len() {
            // First case: value depends on a single typed scenario variable
            1 => {
                let varname = details.vars.iter().next().unwrap().0;
                let typedef =
                    self.variables.get(varname).expect("Unknown variable");
                CaseStmtScenario {
                    var: *varname,
                    full_mask: typedef.full_mask(),
                    remaining: typedef.full_mask(),
                }
            }

            // Second case: a constant value (e.g. Prj'Target)
            0 => CaseStmtScenario {
                var: *scenar_var.1,
                full_mask: 0,
                remaining: 0,
            },

            _ => {
                panic!(
                "Variable used in case statement should have a simple value \
                 that depends on exactly one scenario variable, but got {:?}",
                self.describe(*scenar_var.0),
            );
            }
        }
    }

    /// Combine a new when clause with the current case statement.
    /// Returns None if the when clause can never be active (e.g. we have a
    /// "when others" that doesn't match anything)
    pub fn process_when_clause(
        &mut self,
        context: Scenario,
        case_stmt: &mut CaseStmtScenario,
        when: &WhenClause,
    ) -> Option<Scenario> {
        // If the expression in Case was a constant, check whether this
        // WhenClause would be used.  In this case, case_stmt.var is the
        // actual value of the expression.

        if case_stmt.full_mask == 0 {
            let mut found = false;
            for val in &when.values {
                match val {
                    StringOrOthers::Str(value_in_when) => {
                        if *value_in_when == case_stmt.var {
                            found = true;
                            break;
                        }
                    }
                    StringOrOthers::Others => {
                        found = true;
                        break;
                    }
                }
            }

            if found {
                Some(context)
            } else {
                None
            }
        } else {
            let mut mask = 0_u64;
            let var = self.variables.get(&case_stmt.var).unwrap();

            for val in &when.values {
                match val {
                    StringOrOthers::Str(value_in_when) => {
                        let m = var.mask(value_in_when);
                        mask |= m;
                        case_stmt.remaining &= !m;
                    }
                    StringOrOthers::Others => {
                        mask = case_stmt.remaining;
                        case_stmt.remaining = 0;
                    }
                }
            }

            // Special case: if a WhenClause covers all possible cases, we
            // simply return the default scenario, to avoid building a scenario
            // which in effect is a duplicate

            if mask == 0 {
                None
            } else if mask == case_stmt.full_mask {
                Some(context)
            } else {
                // Scenario just for the WhenClause
                let when_scenario = self.create_single(case_stmt.var, mask);

                // Merged with the context
                self.intersection(context, when_scenario)
            }
        }
    }

    /// Declares a new scenario variables and the list of all values it can
    /// accept.  If the variable is already declared, check that we are
    /// declaring the same set of values.
    /// The list of values must be sorted.
    pub fn try_add_variable(
        &mut self,
        name: Ustr,
        valid: &[Ustr],
    ) -> Result<ExprValue, Error> {
        let values: Vec<(Ustr, u64)> = valid
            .iter()
            .enumerate()
            .map(|(idx, v)| (*v, 2_u64.pow(idx as u32)))
            .collect();

        let val: Vec<(Ustr, Scenario)> = values
            .iter()
            .map(|(str, mask)| (*str, self.create_single(name, *mask)))
            .collect();
        let expr = ExprValue::Str(PerScenario::new_with_variable(&val));

        if let Some(v) = self.variables.get(&name) {
            if v.has_same_valid(&values) {
                Ok(expr)
            } else {
                Err(Error::ScenarioTwice(name))
            }
        } else {
            let var = ScenarioVariable::new(name, values.clone());
            self.variables.insert(var);
            Ok(expr)
        }
    }

    /// Print statistics about scenario variables
    pub fn print_stats(&self) {
        println!("Scenario vars:{:-7}", self.variables.len());
        println!("Scenarios:    {:-7}", self.scenarios.len());
        let total_valid: usize =
            self.variables.iter().map(|v| v.list_valid().len()).sum();
        println!("    values:   {:-7}", total_valid);
    }

    pub fn describe(&self, scenario: Scenario) -> String {
        let details = &self.scenarios[scenario.0];
        let mut varnames = details.vars.keys().collect::<Vec<_>>();

        if varnames.is_empty() {
            "*".to_string()
        } else {
            varnames.sort();
            varnames
                .iter()
                .map(|n| {
                    let var = self.variables.get(*n).unwrap();
                    let d = details.vars[n];

                    // The list of valid values is sorted, so the output will
                    // automatically be sorted.
                    let values = var
                        .list_valid()
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, v)| {
                            if d & 2_u64.pow(idx as u32) != 0 {
                                Some(v.0.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    format!("{}={}", n, values.join("|"))
                })
                .collect::<Vec<_>>()
                .join(",")
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::errors::Error;
    use crate::scenarios::{AllScenarios, Scenario};
    use ustr::Ustr;

    pub fn try_add_variable(
        scenarios: &mut AllScenarios,
        name: &str,
        valid: &[&str],
    ) -> Result<(), Error> {
        scenarios.try_add_variable(
            Ustr::from(name),
            &valid.iter().map(|s| Ustr::from(s)).collect::<Vec<_>>(),
        )?;
        Ok(())
    }

    #[test]
    fn create_scenario() -> Result<(), Error> {
        let mut scenarios = AllScenarios::default();
        try_add_variable(
            &mut scenarios,
            "MODE",
            &["debug", "lto", "optimize"],
        )?;
        try_add_variable(&mut scenarios, "CHECK", &["most", "none", "some"])?;

        let s0 = Scenario::default();
        assert_eq!(scenarios.describe(s0).to_string(), "*");

        //  case Mode is
        //     when "debug" => ...
        let s2 = scenarios.create_single(Ustr::from("MODE"), 1);
        assert_eq!(s2, Scenario(1));
        assert_eq!(scenarios.describe(s2), "MODE=debug");

        //  when others  => for Source_Dirs use ("src1", "src3");
        //     case Check is
        let s3 = scenarios.create_single(Ustr::from("MODE"), 6);
        assert_eq!(s3, Scenario(7));
        assert_eq!(scenarios.describe(s3), "MODE=lto|optimize");

        let same = scenarios.create_single(Ustr::from("MODE"), 6);
        assert_eq!(same, Scenario(7));

        let check_most = scenarios.create_single(Ustr::from("CHECK"), 1);
        let s4 = scenarios.intersection(s3, check_most);
        assert_eq!(s4, Some(Scenario(8)));
        assert_eq!(
            scenarios.describe(s4.unwrap()),
            "CHECK=most,MODE=lto|optimize"
        );

        let check_none_some = scenarios.create_single(Ustr::from("CHECK"), 6);
        let s5 = scenarios.intersection(s3, check_none_some);
        assert_eq!(s5, Some(Scenario(10)));
        assert_eq!(
            scenarios.describe(s5.unwrap()),
            "CHECK=none|some,MODE=lto|optimize"
        );

        //   case Check is
        //      when "none" => for Excluded_Source_Files use ("a.ads");
        let check_none = scenarios.create_single(Ustr::from("CHECK"), 2);
        assert_eq!(check_none, Scenario(5));
        assert_eq!(scenarios.describe(check_none), "CHECK=none");

        //      when others => null;
        let s7 = scenarios.create_single(Ustr::from("CHECK"), 5);
        assert_eq!(s7, Scenario(11));
        assert_eq!(scenarios.describe(s7), "CHECK=most|some");

        Ok(())
    }

    #[test]
    fn test_intersection() -> Result<(), Error> {
        let mut scenarios = AllScenarios::default();
        try_add_variable(
            &mut scenarios,
            "MODE",
            &["debug", "lto", "optimize"],
        )?;
        try_add_variable(&mut scenarios, "CHECK", &["most", "none", "some"])?;
        let s0 = Scenario::default();

        // s0=everything
        // s1=MODE=debug
        //    => s1
        let s1 = scenarios.create_single(Ustr::from("MODE"), 1);
        let res = scenarios.intersection(s0, s1);
        assert_eq!(res, Some(s1));
        let res = scenarios.intersection(s1, s0); // reverse order
        assert_eq!(res, Some(s1));

        // s1=MODE=debug
        // s2=MODE=debug,CHECK=some
        //    => s2
        let check_some = scenarios.create_single(Ustr::from("CHECK"), 4);
        let s2 = scenarios.intersection(s1, check_some).unwrap();
        let res = scenarios.intersection(s1, s2);
        assert_eq!(res, Some(s2));
        let res = scenarios.intersection(s2, s1); // reverse order
        assert_eq!(res, Some(s2));

        // s2=MODE=debug,CHECK=some
        // s3=CHECK=none|some
        //    => s2=MODE=debug,CHECK=some
        let check_none_some = scenarios.create_single(Ustr::from("CHECK"), 6);
        let res = scenarios.intersection(s2, check_none_some);
        assert_eq!(res, Some(s2));
        let res = scenarios.intersection(check_none_some, s2); // reverse order
        assert_eq!(res, Some(s2));

        // s4=MODE=debug|optimize,CHECK=some
        // s5=MODE=lto|optimize,CHECK=some|most
        //    =>  s6=MODE=optimize,CHECK=some
        let mode_debug_opt = scenarios.create_single(Ustr::from("MODE"), 5);
        let s4 = scenarios.intersection(mode_debug_opt, check_some).unwrap();
        let mode_lto_opt = scenarios.create_single(Ustr::from("MODE"), 6);
        let check_some_most = scenarios.create_single(Ustr::from("CHECK"), 5);
        let s5 = scenarios
            .intersection(mode_lto_opt, check_some_most)
            .unwrap();
        let mode_opt = scenarios.create_single(Ustr::from("MODE"), 4);
        let s6 = scenarios.intersection(mode_opt, check_some).unwrap();
        let res = scenarios.intersection(s4, s5);
        assert_eq!(res, Some(s6));
        let res = scenarios.intersection(s5, s4); // reverse order
        assert_eq!(res, Some(s6));

        Ok(())
    }
}
