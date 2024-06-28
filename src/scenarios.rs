/// Project data can be varied based on the values of one or more variables.
/// These variables (named "scenario variables") are typed (so can only take
/// a specific set of values), and can be tested in case statements.
/// When we parse project files, we evaluate all scenarios simultaneously.
/// For instance, if we have
///     project A is
///         type Mode_Type  is ("debug", "optimize", "lto");
///         type Check_Type is ("none", "some", "most");
///         Mode  : Mode_Type := external ("MODE");
///         Check : Check_Type := external ("CHECK");
///         case Mode is
///            when "debug" => for Source_Dirs use ("src1/", "src2/");
///            when others  => for Source_Dirs use ("src1", "src3");
///               case Check is
///                  when "most"  => for Source_Dirs use Source_Dirs & "src4";
///                  when others  => null;
///               end case;
///         end case;
///
///         for Excluded_Source_Files use ();  --  implicit in general
///         case Check is
///            when "none" => for Excluded_Source_Files use ("a.ads");
///            when others => null;
///         end case;
///     end A;
///
/// Then internally we create multiple scenarios:
///     s0         => ()
///     s1         => (mode=debug)
///     s2         => (mode=optimize|lto)
///     s3         => (mode=optimize|lto, check=most)
///     s4         => (check=none)
///     s5 = s1|s2 => () = s0                       # for "src1"
///     s6 = s0-s4 => (check=some|most)             # for excluded_source_files
///     s7 = s1*s6 => (mode=debug,check=some|most)  # for source files, later
///     s8 = s1*s4 => (mode=debug,check=none)       # for source files, later
///     s9 = s2*s6 => (mode=optimize|lto,check=some|most)
///     s10= s2*s4 => (mode=optimize|lto,check=none)
///     s11= s3*s6 => (mode=optimize|lto,check=most)
/// And the attributes of the project are parsed as:
///     source_dirs = (s0, "src1"), (s1, "src2"), (s2, "src3"), (s3, "src4")
///     excluded_source_files = (s6, ) (s4, "a.ads")
///
/// If we parse another project, we will create additional scenarios.  Scenarios
/// can overlap (for instance s3 is fully included in s2), but are not
/// duplicated, for efficiency reasons.
///
/// The second stage of processing for projects is to find the list of source
/// files.  For this, we check the files in all directories:
///     s0  src1 => a.ads, b.ads
///     s1  src2 => b.adb
///     s2  src3 => b.adb
///     s3  src4 => c.ads
/// We need to intersect those with the excluded source files attribute, and
/// create additional scenarios:
///     s0*s6=s6    => src1 - ()        => src1/a.ads, src1/b.ads
///     s0*s4=s4    => src1 - ("a.ads") => src1/b.ads
///     s1*s6=s7    => src2 - ()        => src2/b.adb
///     s1*s4=s8    => src2 - ("a.ads") => src2/b.adb
///     s2*s6=s9    => src3 - ()        => src3/b.adb
///     s2*s4=s10   => src3 - ("a.ads") => src3/b.adb
///     s3*s6=s11   => src4 - ()        => src4/c.ads
///     s3*s4=()    => ()
///
/// Now, for instance to find the full list of source files in the scenario
///     s20 => (mode=optimize,check=none)
/// we need to intersect that scenario with each of the ones used in the list of
/// source files, and keep non-empty ones:
///     s20*s6  = empty
///     s20*s4  = not empty    => src1/b.ads
///     s20*s7  = empty
///     s20*s8  = empty
///     s20*s9  = empty
///     s20*s10 = not empty    => src3/b.adb
///     s20*s11 = empty
///
/// Likewise, when we later want to resolve file dependencies (e.g. we have
/// a project B that imports A, and one of its files d.ads depends on
/// b.adb).  We thus take the intersection of each scenario where d.ads exists
/// (say s0 to simplify) which each scenario needed for A's source_files
/// attribute, to know which b.adb gets used.
///     s0*s7  = s7  => src2/b.adb
///     s0*s8  = s8  => src2/b.adb
///     s0*s9  = s9  => src3/b.adb
///     s0*s10 = s10 => src3/b.adb
/// There are duplicates here, so we can group things to reduce the size.
///     s7|s8  = (mode=debug,check=some|most) | (mode=debug,check=none)
///            = (mode=debug) = s1     => src2/b.adb
///     s9|s10 = (mode=opt|lto,check=some|most) | (mode=opt|lto,check=none)
///            = (mode=opt|lto) = s2   => src3/b.adb
use crate::errors::Error;
use crate::scenario_variables::ScenarioVariable;
use std::collections::{HashMap, HashSet};
use ustr::{Ustr, UstrMap};

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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
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

    /// Create a htable with one entry per valid value of the variable.
    /// For instance:
    ///     type T is ("on", "off");
    ///     V : T := external ("name")
    /// And you could this function for "name", we get the following as output
    ///     {"name=on": "on", "name=off": "off"}
    /// This is used so that one can then use "V" in an expression in the
    /// project for instance.
    pub fn expr_from_variable(
        &mut self,
        varname: Ustr,
    ) -> HashMap<Scenario, Ustr> {
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
        map
    }

    /// Finds the intersection of two scenarios
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
            match d1.vars.get(var.name()) {
                None => {
                    match d2.vars.get(n) {
                        None => {
                            // both scenario allow all values
                        }
                        Some(v2) => {
                            d.vars.insert(*n, *v2);
                        }
                    }
                }
                Some(v1) => {
                    match d2.vars.get(n) {
                        None => {
                            d.vars.insert(*n, *v1);
                        }
                        Some(v2) => {
                            let v = *v1 & *v2;

                            //  We can end up with an empty set of possible
                            //  values.  In this case, the intersection is
                            //  empty.
                            if (v & var.full_mask()) == 0 {
                                return None;
                            }

                            d.vars.insert(*n, *v1 & *v2);
                        }
                    }
                }
            }
        }
        Some(self.create_or_reuse(d))
    }

    /// Union of two scenarios
    /// Used when a value (e.g. one of the source directories) is present in
    /// multiple scenarios.  If possible it returns a new (larger) scenario
    /// where the variable applies.
    ///
    ///     [mode=debug,    check=on]
    ///     [mode=optimize, check=on]
    ///        => [mode=debug|optimize, check=on]
    ///
    ///     [mode=debug, check=on]
    ///     [mode=lto,   check=off]
    ///        => no merging, they differ on more than one variable
    ///
    ///     [mode=debug, check=on]
    ///     [mode=debug]      valid for all values of check
    ///        => [mode=debug]
    ///

    pub fn union(&mut self, s1: Scenario, s2: Scenario) -> Option<Scenario> {
        let mut diffcount = 0;
        let mut d1 = self.scenarios[s1.0].clone();
        let d2 = &self.scenarios[s2.0];
        let mut to_remove: Option<Ustr> = None;

        for (name, value) in &mut d1.vars {
            match d2.vars.get(name) {
                None => {
                    if diffcount > 0 {
                        return None;
                    }

                    // `name` is not found in s2, so this behaves as if it
                    // accepted all possible values of the variable.  To
                    // represent this, we remove the entry for the variable.
                    diffcount += 1;
                    to_remove = Some(*name);
                }
                Some(value2) if *value != *value2 => {
                    if diffcount > 0 {
                        return None;
                    }

                    *value |= *value2;
                    diffcount += 1;

                    // If a variable now has all possible values in the
                    // scenario, we simply remove it
                    // (e.g. MODE=debug|optimize|lto is the same as not checking
                    // MODE at all).
                    let var = self.variables.get(name).unwrap();
                    if (*value & var.full_mask()) == var.full_mask() {
                        to_remove = Some(*name);
                    }
                }
                Some(_) => {}
            }
        }

        for name in d2.vars.keys() {
            if !d1.vars.contains_key(name) {
                if diffcount > 0 {
                    return None;
                }
                diffcount += 1;
                d1.vars.remove(name);
            }
        }

        if let Some(n) = to_remove {
            d1.vars.remove(&n);
        }

        Some(self.create_or_reuse(d1))
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
    fn test_union() -> Result<(), Error> {
        let mut scenarios = AllScenarios::default();
        try_add_variable(
            &mut scenarios,
            "MODE",
            &["debug", "lto", "optimize"],
        )?;
        try_add_variable(&mut scenarios, "CHECK", &["most", "none", "some"])?;
        let s0 = Scenario::default();

        //  s3=[mode=debug,    check=some]
        //  s5=[mode=optimize, check=some]
        //     => s6=[mode=debug|optimize, check=some]
        let s2 = split(&mut scenarios, s0, "MODE", &["debug"]).unwrap();
        let s3 = split(&mut scenarios, s2, "CHECK", &["some"]).unwrap();
        let s4 = split(&mut scenarios, s0, "MODE", &["optimize"]).unwrap();
        let s5 = split(&mut scenarios, s4, "CHECK", &["some"]).unwrap();
        let s6 = scenarios.union(s3, s5);
        assert_eq!(s6, Some(Scenario(5)));
        assert_eq!(
            scenarios.describe(s6.unwrap()),
            "CHECK=some,MODE=debug|optimize"
        );

        let s6 = scenarios.union(s5, s3); //  reverse order
        assert_eq!(s6, Some(Scenario(5)));
        assert_eq!(
            scenarios.describe(s6.unwrap()),
            "CHECK=some,MODE=debug|optimize"
        );

        //  s3=[mode=debug, check=some]
        //  s8=[mode=lto,   check=most]
        //     => no merging, they differ on more than one variable
        let s6 = split(&mut scenarios, s0, "MODE", &["lto"]).unwrap();
        let s7 = split(&mut scenarios, s6, "CHECK", &["most"]).unwrap();
        let res = scenarios.union(s2, s7);
        assert!(res.is_none());
        let res = scenarios.union(s7, s2); // reverse order
        assert!(res.is_none());

        //  s3=[mode=debug, check=some]
        //  s2=[mode=debug]      valid for all values of check
        //     => s2=[mode=debug]
        let res = scenarios.union(s3, s2);
        assert_eq!(res, Some(Scenario(1)));
        assert_eq!(scenarios.describe(res.unwrap()), "MODE=debug");

        let res = scenarios.union(s2, s3); //  reverse order
        assert_eq!(res, Some(Scenario(1)));
        assert_eq!(scenarios.describe(res.unwrap()), "MODE=debug");

        // Merging same value multiple times has no impact
        let s2 = split(&mut scenarios, s0, "MODE", &["debug"]).unwrap();
        let s3 = split(&mut scenarios, s0, "MODE", &["optimize"]).unwrap();
        let s4 = scenarios.union(s2, s3).unwrap();
        let res = scenarios.union(s4, s2).unwrap();
        assert_eq!(scenarios.describe(res), "MODE=debug|optimize");

        // Merging all possible values for a variable should remote it from
        // the union altogether.
        let s2 = split(&mut scenarios, s0, "MODE", &["debug"]).unwrap();
        let s3 = split(&mut scenarios, s0, "MODE", &["optimize"]).unwrap();
        let s4 = split(&mut scenarios, s0, "MODE", &["lto"]).unwrap();
        let s5 = scenarios.union(s2, s3).unwrap();
        let res = scenarios.union(s5, s4).unwrap();
        assert_eq!(scenarios.describe(res), "");
        assert_eq!(res, s0);

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
