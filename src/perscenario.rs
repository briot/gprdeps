use crate::allscenarios::AllScenarios;
use crate::scenarios::Scenario;
use std::collections::HashMap;
use ustr::Ustr;

/// A lot of expressions and variables in projects will have a value that
/// differs depending on the scenario.
/// The set of values must cover the whole space of scenarios (and the functions
/// in this package ensure this is the case).  It is possible for multiple
/// scenarios to overlap.  With all methods below, this should still result in
/// consistent values for a given scenario.
#[derive(Clone, Debug, PartialEq)]
pub struct PerScenario<T> {
    values: HashMap<Scenario, T>,
}

impl<T> PerScenario<T> {
    /// Create a new value, with a default value valid for all scenarios
    pub fn new(default_val: T) -> Self {
        let mut m = HashMap::new();
        m.insert(Scenario::default(), default_val);
        PerScenario { values: m }
    }

    /// Iterate over all possible values
    pub fn iter(&self) -> impl Iterator<Item = (&Scenario, &T)> {
        self.values.iter()
    }

    /// Transform the value into another value with the same scenarios
    pub fn map<U, F>(&self, mut transform: F) -> PerScenario<U>
    where
        F: FnMut(&T) -> U,
    {
        PerScenario {
            values: self
                .values
                .iter()
                .map(|(scenario, orig)| (*scenario, transform(orig)))
                .collect(),
        }
    }

    /// Display the value of a variable on two columns:
    ///     <indent>scenario1 value1<eol>
    ///     <indent>scenar2   value2<eol>
    pub fn two_columns(
        &self,
        scenarios: &AllScenarios,
        indent: &str,
        eol: &str,
        fmt: fn(&T) -> String,
    ) -> String {
        let mut col1 = Vec::new();
        for scenario in self.values.keys() {
            col1.push(scenarios.describe(*scenario));
        }
        let max = col1.iter().map(String::len).max().unwrap_or(0);
        let mut lines = self
            .values
            .iter()
            .enumerate()
            .map(|(idx, (_, val))| {
                format!(
                    "{}{:width$} {}",
                    indent,
                    col1[idx],
                    fmt(val),
                    width = max
                )
            })
            .collect::<Vec<_>>();
        lines.sort();
        lines.join(eol)
    }
}

impl PerScenario<Ustr> {
    /// Create a new hashmap from a scenario variable.
    /// All possible values of the scenario variable must be provided.
    /// Given
    ///     type On_Off is ("on", "off");
    ///     E1 : On_Off := external ("e1");
    /// the returned value is
    ///     {"e1=on": "on",  "e1=off": "off"}
    pub fn new_with_variable(
        full_mask: Scenario,
        values: &[(Ustr, Scenario)],
    ) -> Self {
        let base = Scenario::default().0 & !full_mask.0;
        let mut m = HashMap::new();
        for (u, s) in values {
            m.insert(Scenario(s.0 | base), *u);
        }
        PerScenario { values: m }
    }
}

impl<T> PerScenario<T>
where
    T: ::core::fmt::Debug,
{
    #[cfg(test)]
    pub fn format(&self, scenars: &AllScenarios) -> String {
        let mut items = self.values.iter().collect::<Vec<_>>();
        items.sort_by_key(|(s, _)| *s);

        let mut res = String::new();
        res.push('{');
        for (s, v) in items {
            res.push_str(&scenars.describe(*s));
            res.push(':');
            res.push_str(&format!("{:?}", v));
            res.push_str(", ");
        }
        res.push('}');
        res
    }
}

impl<T> PerScenario<T>
where
    T: Clone,
{
    /// Merge two values (and modifies self in place).
    /// The context represents a (nested) case statement, for instance:
    ///     case E1 is
    ///        when "a" | "b" =>
    ///           ...
    ///
    /// Values outside of this context are left unchanged.
    pub fn merge<U, F>(
        &mut self,
        right: &mut PerScenario<U>,
        context: Scenario,
        scenars: &mut AllScenarios,
        merge: F,
    ) where
        F: Fn(&mut T, &U),
    {
        for (s2, v2) in &right.values {
            self.merge_one(context, scenars, &merge, *s2, v2);
        }
    }

    /// Similar to merge(), but for a single scenario.
    pub fn merge_one<U, F>(
        &mut self,
        context: Scenario,
        scenars: &mut AllScenarios,
        merge: F,
        scenario: Scenario,
        value: &U,
    ) where
        F: Fn(&mut T, &U),
    {
        let s = context & scenario;
        if !scenars.never_matches(s) {
            let mut res = HashMap::new();
            std::mem::swap(&mut self.values, &mut res);

            for (s1, mut v1) in res.into_iter() {
                let ns = s1 & s;
                if !scenars.never_matches(ns) {
                    for negated in scenars.negate(s) {
                        let s1_neg = s1 & negated;
                        if !scenars.never_matches(s1_neg) {
                            self.values.insert(s1_neg, v1.clone());
                        }
                    }
                    merge(&mut v1, value);
                    self.values.insert(ns, v1);
                } else {
                    self.values.insert(s1, v1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::allscenarios::tests::{create_single, try_add_variable};
    use crate::allscenarios::AllScenarios;
    use crate::errors::Error;
    use crate::perscenario::PerScenario;
    use crate::scenarios::Scenario;

    #[test]
    fn test_per_scenario() -> Result<(), Error> {
        let mut scenars = AllScenarios::default();
        try_add_variable(&mut scenars, "E1", &["a", "b", "c", "d"]);
        try_add_variable(&mut scenars, "E2", &["e", "f"]);

        let zero = PerScenario::<u8>::new(0);
        assert_eq!(zero.format(&scenars), "{E1=*,E2=*:0, }",);

        // Splitting on s0 context has no effect.
        // Case of doing   V := 1  at the top level.
        let mut one = PerScenario::<u8>::new(1);
        let mut v = zero.clone();
        v.merge(&mut one, Scenario::default(), &mut scenars, |old, new| {
            *old = *new
        });
        assert_eq!(v.format(&scenars), "{E1=*,E2=*:1, }",);

        // Now assume we are inside a case statement.
        //    case E1 is
        //       when a|b => V := 2;
        let ctx = create_single(&mut scenars, "E1", &["a", "b"]);

        // First version: we merge one specific scenario:
        let mut v2 = v.clone();
        v2.merge_one(
            ctx,
            &mut scenars,
            |old, new| *old = *new,
            Scenario::default(),
            &2,
        );
        assert_eq!(v2.format(&scenars), "{E1=a|b,E2=*:2, E1=c|d,E2=*:1, }",);

        // Second version: we merge another PerScenario value
        let mut v2 = v.clone();
        let mut two = PerScenario::<u8>::new(2);
        v2.merge(&mut two, ctx, &mut scenars, |old, new| *old = *new);
        assert_eq!(v2.format(&scenars), "{E1=a|b,E2=*:2, E1=c|d,E2=*:1, }",);

        // Now use the above in another case statement.
        //    L := ("a");
        //    case E2 is
        //       when e  => L := L & V;
        // Note that the result has multiple overlapping scenarios when E2=f
        // for instance, but they all result in the same value for a given
        // scenario.
        let ctx = create_single(&mut scenars, "E2", &["e"]);
        let mut v3 = PerScenario::new(vec![]);
        v3.merge(&mut v2, ctx, &mut scenars, |old, new| old.push(*new));
        assert_eq!(
            v3.format(&scenars),
            "{E1=a|b,E2=e:[2], E1=c|d,E2=e:[1], E1=c|d,E2=f:[], E1=*,E2=f:[], }",
        );

        Ok(())
    }
}
