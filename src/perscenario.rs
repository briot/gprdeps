use crate::allscenarios::AllScenarios;
use crate::scenarios::Scenario;
use std::collections::HashMap;
use ustr::Ustr;

#[cfg(test)]
use std::fmt::Write;

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

impl<T> Default for PerScenario<T>
where
    T: Default,
{
    fn default() -> Self {
        PerScenario::new(T::default())
    }
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

    /// Update self.
    /// The context represents a (nested) case statement, for instance:
    ///     case E1 is
    ///        when "a" | "b" =>
    ///           ...
    /// Values outside of this context are left unchanged.
    pub fn update<U, F>(
        &mut self,
        right: &PerScenario<U>,
        context: Scenario,
        scenars: &mut AllScenarios,
        merge: F,
    ) where
        T: Clone,
        F: Fn(&mut T, &U),
    {
        let mut to_replace = Vec::new();

        // ??? Order of iteration may vary, resulting in test output changes
        for (s2, v2) in &right.values {
            let s = context & s2;
            to_replace.extend(
                self.values
                    .keys()
                    .filter(|s1| !scenars.never_matches(*s1 & s)),
            );
            for s1 in &to_replace {
                if let Some(mut v1) = self.values.remove(s1) {
                    for negated in scenars.negate(s) {
                        let s1_neg = s1 & negated;
                        if !scenars.never_matches(s1_neg) {
                            self.values.insert(s1_neg, v1.clone());
                        }
                    }
                    merge(&mut v1, v2);
                    self.values.insert(s1 & s, v1);
                }
            }
            to_replace.clear();
        }
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
        let base = Scenario::default() & !full_mask;
        let mut m = HashMap::new();
        for (u, s) in values {
            m.insert(s | base, *u);
        }
        PerScenario { values: m }
    }
}

#[cfg(test)]
impl<T> PerScenario<T>
where
    T: ::core::fmt::Debug,
{
    pub fn format(&self, scenars: &AllScenarios) -> String {
        let mut items = self.values.iter().collect::<Vec<_>>();
        items.sort_by_key(|(s, _)| *s);

        let mut res = String::new();
        res.push('{');
        for (s, v) in items {
            res.push_str(&scenars.describe(*s));
            res.push(':');
            let _ = write!(res, "{:?}", v); // ignore errors in tests
            res.push_str(", ");
        }
        res.push('}');
        res
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
        assert_eq!(zero.format(&scenars), "{*:0, }",);

        // Splitting on s0 context has no effect.
        // Case of doing   V := 1  at the top level.
        let one = PerScenario::<u8>::new(1);
        let mut v = zero.clone();
        v.update(&one, Scenario::default(), &mut scenars, |old, new| {
            *old = *new
        });
        assert_eq!(v.format(&scenars), "{*:1, }",);

        // Now assume we are inside a case statement.
        //    case E1 is
        //       when a|b => V := 2;
        let ctx = create_single(&mut scenars, "E1", &["a", "b"]);
        let mut v2 = v.clone();
        let two = PerScenario::<u8>::new(2);
        v2.update(&two, ctx, &mut scenars, |old, new| *old = *new);
        assert_eq!(v2.format(&scenars), "{E1=a|b:2, E1=c|d:1, }",);

        // Now use the above in another case statement.
        //    L := ("a");
        //    case E2 is
        //       when e  => L := L & V;
        // Note that the result has multiple overlapping scenarios when E2=f
        // for instance, but they all result in the same value for a given
        // scenario.
        // The actual output may vary depending how Rust iterates a hash map
        // in merge().
        let ctx = create_single(&mut scenars, "E2", &["e"]);
        let mut v3 = PerScenario::new(vec![]);
        v3.update(&v2, ctx, &mut scenars, |old, new| old.push(*new));
        let out = v3.format(&scenars);
        let expect1 =
            "{E1=a|b,E2=e:[2], E1=c|d,E2=e:[1], E1=c|d,E2=f:[], E2=f:[], }";
        let expect2 =
            "{E1=a|b,E2=e:[2], E1=c|d,E2=e:[1], E1=a|b,E2=f:[], E2=f:[], }";
        if out != expect1 && out != expect2 {
            assert_eq!(out, expect1);
        }

        Ok(())
    }
}
