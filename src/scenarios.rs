/// Project data can vary based on the values of one or more variables.
/// These variables (named "scenario variables") are typed (so can only take
/// a specific set of values), and can be tested in case statements.
/// When we parse project files, we evaluate all scenarios simultaneously.
use crate::errors::Error;
use crate::perscenario::PerScenario;
use crate::rawexpr::{StringOrOthers, WhenClause};
use crate::scenario_variables::ScenarioVariable;
use std::collections::{HashMap, HashSet};
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

#[derive(Clone)]
pub struct WhenClauseScenario {
    // The scenario associated with this set of values.  This involves a
    // single variable.
    pub scenario: Scenario,

    // The scenario associated with all other values of the variable
    // For instance:
    //     type E1_Type is ("a", "b", "c", "d");
    //     E1 : E1_Type := external ("E1");
    //     case E1 is
    //        when "a" | "b" =>
    //
    // Then scenario is E1="a"|"b", and  self.negate_scenario is E1="c"|"d".
    // This is None when scenario covers all possible values.
    pub negate_scenario: Option<Scenario>,
}

impl ::core::fmt::Debug for WhenClauseScenario {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "clause:{:?}", self.scenario)
    }
}

impl WhenClauseScenario {
    pub fn new(
        scenars: &mut AllScenarios,
        varname: Ustr,
        mask: u64,
        full_mask: u64,
    ) -> Self {
        let mut details = ScenarioDetails::default();
        details.vars.insert(varname, mask);

        let negate = !mask & full_mask;
        let mut neg_details = ScenarioDetails::default();
        neg_details.vars.insert(varname, negate);

        WhenClauseScenario {
            scenario: scenars.create_or_reuse(details),
            negate_scenario: Some(scenars.create_or_reuse(neg_details)),
        }
    }
}

/// Describes nested when clauses
#[derive(Clone, Debug)]
pub struct WhenContext {
    pub clauses: Vec<WhenClauseScenario>,
    pub scenario: Scenario,
}

impl WhenContext {
    pub fn new() -> Self {
        WhenContext {
            clauses: Vec::new(),
            scenario: Scenario::default(),
        }
    }

    pub fn push(
        &self,
        scenars: &mut AllScenarios,
        clause: WhenClauseScenario,
    ) -> Option<Self> {
        match scenars.intersection(self.scenario, clause.scenario) {
            None => None,
            Some(s) => {
                let mut context2 = self.clone();
                context2.clauses.push(clause);
                context2.scenario = s;
                Some(context2)
            }
        }
    }
}

/// Describes the set of scenario variables covered by a scenario.  For each
/// known scenario variables, we either have:
///    * no entry in vars: all values of the variables are valid
///    * a bitmask that indicates which values are allowed in this scenario.
#[derive(Default, PartialEq, Clone)]
struct ScenarioDetails {
    vars: UstrMap<u64>, // Variable name => bitmak of valid values
}

/// A pointer to a specific scenario.
/// The default is a scenario that allows all values for all variables
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Scenario(pub(crate) usize);

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
        let s = Scenario(self.scenarios.len() - 1);
        println!("MANU new scenario {} = {}", s, self.describe(s));
        s
    }

    /// Create a htable with one entry per valid value of the variable.
    /// For instance:
    ///     type T is ("on", "off");
    ///     V : T := external ("name")
    /// And you could this function for "name", we get the following as output
    ///     {"name=on": "on", "name=off": "off"}
    /// This is used so that one can then use "V" in an expression in the
    /// project for instance.
    /// The output is compatible with an ExprValue::Str
    pub fn expr_from_variable(&mut self, varname: Ustr) -> PerScenario<Ustr> {
        let mut map = HashMap::new();
        let values = {
            let var = self.variables.get(&varname).expect("Unknown variable");
            var.list_valid().iter().copied().clone().collect::<Vec<_>>()
        };

        for (idx, v) in values.iter().enumerate() {
            let mut details = ScenarioDetails::default();
            details.vars.insert(varname, 2_u64.pow(idx as u32));
            let scenario = self.create_or_reuse(details);
            map.insert(scenario, *v);
        }
        PerScenario::new_with_map(map)
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
        println!("MANU intersection {} and {}", s1, s2);
        if s1 == s2 {
            println!("MANU    => same {}", s1);
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
        let result = self.create_or_reuse(d);
        println!("MANU    => {}", result);
        Some(result)
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
        let scenar = variable_values
            .iter()
            .next()
            .expect("Must have at least one possible value")
            .0;
        let details = &self.scenarios[scenar.0];
        let varname = details.vars.iter().next().unwrap().0;
        assert_eq!(details.vars.len(), 1);
        let typedef = self.variables.get(varname).expect("Unknown variable");
        CaseStmtScenario {
            var: *varname,
            full_mask: typedef.full_mask(),
            remaining: typedef.full_mask(),
        }
    }

    /// Combine a new when clause with the current case statement.
    /// Returns None if the when clause can never be active (e.g. we have a
    /// "when others" that doesn't match anything)
    pub fn process_when_clause(
        &mut self,
        case_stmt: &mut CaseStmtScenario,
        when: &WhenClause,
    ) -> Option<WhenClauseScenario> {
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

        // Special case: if a WhenClause covers all possible cases, we simply
        // return the default scenario, to avoid building a scenario which in
        // effect is a duplicate
        if mask == case_stmt.full_mask {
            Some(WhenClauseScenario {
                scenario: Scenario::default(),
                negate_scenario: None,
            })
        } else if mask == 0 {
            None
        } else {
            Some(WhenClauseScenario::new(
                self,
                case_stmt.var,
                mask,
                case_stmt.full_mask,
            ))
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
    ) -> Result<(), Error> {
        match self.variables.get(&name) {
            None => {
                self.variables.insert(ScenarioVariable::new(name, valid));
                Ok(())
            }
            Some(oldvar) => {
                if oldvar.has_same_valid(valid) {
                    Ok(())
                } else {
                    Err(Error::ScenarioTwice(name))
                }
            }
        }
    }

    pub fn describe(&self, scenario: Scenario) -> String {
        let details = &self.scenarios[scenario.0];
        let mut varnames = details.vars.keys().collect::<Vec<_>>();

        if varnames.is_empty() {
            "".to_string()
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
                                Some(v.as_str())
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
    use crate::scenario_variables::tests::build_var;
    use crate::scenarios::{AllScenarios, Scenario};
    use ustr::Ustr;

    /// Restrict the scenario to a subset of values for the given variables.
    /// This either returns an existing matching scenario, or a new one.
    /// Returns None if the result never matches a valid combination of
    /// scenarios.
    pub fn split(
        scenarios: &mut AllScenarios,
        scenario: Scenario,
        variable: &str,
        values: &[&str],
    ) -> Option<Scenario> {
        // Prepare the new details
        let mut tmp = scenarios.scenarios[scenario.0].clone();
        let varname = Ustr::from(variable);
        let var = scenarios.variables.get(&varname).unwrap();
        let bitmask = build_var(var, values);
        match tmp.vars.get_mut(&varname) {
            None => {
                tmp.vars.insert(varname, bitmask);
            }
            Some(v) => {
                *v &= bitmask;
                if (*v & var.full_mask()) == 0 {
                    return None;
                }
            }
        }

        Some(scenarios.create_or_reuse(tmp))
    }

    pub fn try_add_variable(
        scenarios: &mut AllScenarios,
        name: &str,
        valid: &[&str],
    ) -> Result<(), Error> {
        scenarios.try_add_variable(
            Ustr::from(name),
            &valid.iter().map(|s| Ustr::from(s)).collect::<Vec<_>>(),
        )
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
        assert_eq!(scenarios.describe(s0).to_string(), "");

        //  case Mode is
        //     when "debug" => ...
        let s2 = split(&mut scenarios, s0, "MODE", &["debug"]);
        assert_eq!(s2, Some(Scenario(1)));
        assert_eq!(scenarios.describe(s2.unwrap()), "MODE=debug");

        //  when others  => for Source_Dirs use ("src1", "src3");
        //     case Check is
        let s3 = split(&mut scenarios, s0, "MODE", &["optimize", "lto"]);
        assert_eq!(s3, Some(Scenario(2)));
        assert_eq!(scenarios.describe(s3.unwrap()), "MODE=lto|optimize");

        let same = split(&mut scenarios, s0, "MODE", &["optimize", "lto"]);
        assert_eq!(same, Some(Scenario(2)));

        let s4 = split(&mut scenarios, s3.unwrap(), "CHECK", &["most"]);
        assert_eq!(s4, Some(Scenario(3)));
        assert_eq!(
            scenarios.describe(s4.unwrap()),
            "CHECK=most,MODE=lto|optimize"
        );

        let s5 = split(&mut scenarios, s3.unwrap(), "CHECK", &["none", "some"]);
        assert_eq!(s5, Some(Scenario(4)));
        assert_eq!(
            scenarios.describe(s5.unwrap()),
            "CHECK=none|some,MODE=lto|optimize"
        );

        //   case Check is
        //      when "none" => for Excluded_Source_Files use ("a.ads");
        let s6 = split(&mut scenarios, s0, "CHECK", &["none"]);
        assert_eq!(s6, Some(Scenario(5)));
        assert_eq!(scenarios.describe(s6.unwrap()), "CHECK=none");

        //      when others => null;
        let s7 = split(&mut scenarios, s0, "CHECK", &["some", "most"]);
        assert_eq!(s7, Some(Scenario(6)));
        assert_eq!(scenarios.describe(s7.unwrap()), "CHECK=most|some");

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
        let s1 = split(&mut scenarios, s0, "MODE", &["debug"]).unwrap();
        let res = scenarios.intersection(s0, s1);
        assert_eq!(res, Some(s1));
        let res = scenarios.intersection(s1, s0); // reverse order
        assert_eq!(res, Some(s1));

        // s1=MODE=debug
        // s2=MODE=debug,CHECK=some
        //    => s2
        let s2 = split(&mut scenarios, s1, "CHECK", &["some"]).unwrap();
        let res = scenarios.intersection(s1, s2);
        assert_eq!(res, Some(s2));
        let res = scenarios.intersection(s2, s1); // reverse order
        assert_eq!(res, Some(s2));

        // s2=MODE=debug,CHECK=some
        // s3=CHECK=none|some
        //    => s2=MODE=debug,CHECK=some
        let s3 = split(&mut scenarios, s0, "CHECK", &["none", "some"]).unwrap();
        let res = scenarios.intersection(s2, s3);
        assert_eq!(res, Some(s2));
        let res = scenarios.intersection(s3, s2); // reverse order
        assert_eq!(res, Some(s2));

        // s4=MODE=debug|optimize,CHECK=some
        // s5=MODE=lto|optimize,CHECK=some|most
        //    =>  s6=MODE=optimize,CHECK=some
        let s4_step1 =
            split(&mut scenarios, s0, "MODE", &["debug", "optimize"]).unwrap();
        let s4 = split(&mut scenarios, s4_step1, "CHECK", &["some"]).unwrap();
        let s5_step1 =
            split(&mut scenarios, s0, "MODE", &["lto", "optimize"]).unwrap();
        let s5 = split(&mut scenarios, s5_step1, "CHECK", &["some", "most"])
            .unwrap();
        let s6_step1 =
            split(&mut scenarios, s0, "MODE", &["optimize"]).unwrap();
        let s6 = split(&mut scenarios, s6_step1, "CHECK", &["some"]).unwrap();
        let res = scenarios.intersection(s4, s5);
        assert_eq!(res, Some(s6));
        let res = scenarios.intersection(s5, s4); // reverse order
        assert_eq!(res, Some(s6));

        Ok(())
    }
}
