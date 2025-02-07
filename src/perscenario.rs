use crate::scenarios::{AllScenarios, Scenario, WhenContext};
use std::collections::HashMap;
use ustr::Ustr;

/// A lot of expressions and variables in projects will have a value that
/// differs depending on the scenario.
#[derive(Clone, Debug, PartialEq)]
pub struct PerScenario<T> {
    pub values: HashMap<Scenario, T>,
}

impl<T> PerScenario<T> {
    /// Create a new value, with a default value valid for all scenarios
    pub fn new(default_val: T) -> Self {
        let mut m = HashMap::new();
        m.insert(Scenario::default(), default_val);
        PerScenario { values: m }
    }

    /// Create a new hashmap, with a single value.
    pub fn new_with_scenario(val: T, scenario: Scenario) -> Self {
        let mut m = HashMap::new();
        m.insert(scenario, val);
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
    /// Create a new hahmap from a scenario variable
    pub fn new_with_variable(values: &[(Ustr, Scenario)]) -> Self {
        let mut m = HashMap::new();
        for (u, s) in values {
            m.insert(*s, *u);
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
        items.sort_by(|v1, v2| (v1.0).cmp(v2.0));

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
    /// Split a hashmap to isolate scenarios.
    /// We are about to modify a hashmap but only for one scenario.  To prepare
    /// for this, we split items in the hashmap whenever they intersect with the
    /// scenario.
    /// For instance, if we start with the following variables:
    ///     s1: Scenario = {"E1": "on"}
    ///     s2: Scenario = {"E1": "off"}
    ///     s3: Scenario = {"E2": "on" }
    ///     s4: Scenario = {"E2": "off" }
    ///     expression = { s1: "a", s2&s3: "b", s2&s4: "c" }
    /// And then split the expression for s3, we get
    ///     { s1&s3: "a", s1&!s3:"a", s2&s3&s3:"b", s2&s3&!s3:"b",
    ///       s2&s4&s3:"c", s2&s4&!s3:"c" }
    /// which is simplified to:
    ///     { s1&s3:"a", s1&s4:"a", s2&s3:"b", s2&s4:"c"}
    /// This is exactly equivalent to the initial expression, though it does
    /// have more cases.
    /// This function handles the case where the scenario we split on has more
    /// than one variable.
    fn split(&mut self, context: &WhenContext, scenars: &mut AllScenarios) {
        let mut active: Option<Vec<Scenario>> = None;

        for c in &context.clauses {
            let mut res = HashMap::new();
            let mut new_active = Vec::new();
            for (scenario, v) in self.values.iter_mut() {
                if active.as_ref().map_or(true, |l| l.contains(scenario)) {
                    if let Some(s) = scenars.intersection(*scenario, c.scenario)
                    {
                        new_active.push(s);
                        res.insert(s, v.clone());
                    }
                    if let Some(n) = c.negate_scenario {
                        if let Some(s) = scenars.intersection(*scenario, n) {
                            res.insert(s, v.clone());
                        }
                    }
                } else {
                    res.insert(*scenario, v.clone());
                }
            }
            active = Some(new_active);
            self.values = res;
        }
    }

    /// Apply a delta to the hashmap.
    /// This delta only has values for the current context (e.g. case
    /// statements).
    /// For instance, given:
    ///     case E is
    ///        when "on" =>   V := V1 & V2;
    /// then delta would be the value of V1 & V2 and will only include a
    /// value for the context "E=on".  But if V already had values for other
    /// scenarios they should be preserved.
    /// Note also that self might for instance contain lists, but receive a
    /// delta containing strings (this is the "&" operator).
    pub fn update<U, F>(
        &mut self,
        context: &WhenContext,
        delta: PerScenario<U>,
        scenars: &mut AllScenarios,
        convert: F,
    ) where
        F: Fn(U) -> T,
    {
        self.split(context, scenars);
        self.values.retain(|s, _v| {
            scenars.intersection(*s, context.scenario).is_none()
        });
        for (k, v) in delta.values {
            self.values.insert(k, convert(v));
        }
    }

    /// Merge two hashmaps (modified self in place)
    /// Only the scenario of the context is impacted, all other scenarios
    /// preserve their old values.
    pub fn merge<U, F>(
        &mut self,
        right: &mut PerScenario<U>,
        context: &WhenContext,
        scenars: &mut AllScenarios,
        merge: F,
    ) where
        F: Fn(&mut T, &U),
        U: Clone,
    {
        self.split(context, scenars);
        right.split(context, scenars);

        for (s2, v2) in &right.values {
            self.merge_one(context, scenars, &merge, *s2, v2);
        }
    }

    pub fn merge_one<U, F>(
        &mut self,
        _context: &WhenContext,
        scenars: &mut AllScenarios,
        merge: F,
        scenario: Scenario,
        value: &U,
    ) where
        F: Fn(&mut T, &U),
        U: Clone,
    {
        for (s1, v1) in self.values.iter_mut() {
            if scenars.intersection(*s1, scenario).is_some() {
                merge(v1, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::Error;
    use crate::perscenario::PerScenario;
    use crate::scenarios::tests::try_add_variable;
    use crate::scenarios::{AllScenarios, WhenClauseScenario, WhenContext};
    use ustr::Ustr;

    #[test]
    fn test_per_scenario() -> Result<(), Error> {
        let mut scenars = AllScenarios::default();
        try_add_variable(&mut scenars, "E1", &["a", "b", "c", "d"])?;
        try_add_variable(&mut scenars, "E2", &["e", "f"])?;

        let context = WhenContext::new();

        // Splitting on s0 context has no effect
        let zero = PerScenario::<u8>::new(0);
        let mut empty = zero.clone();
        empty.split(&context, &mut scenars);
        assert_eq!(empty, zero);

        // Splitting at the toplevel (empty context), also has no effect
        let mut oneval = PerScenario::<u8>::new(1);
        let old = oneval.clone();
        oneval.split(&context, &mut scenars);
        assert_eq!(oneval, old);

        // Now splitting on a variable
        let when =
            WhenClauseScenario::new(&mut scenars, Ustr::from("E1"), 3, 31);
        let context2 = context.push(&mut scenars, when).unwrap();
        let mut oneval = PerScenario::<u8>::new(1);
        oneval.split(&context2, &mut scenars);
        assert_eq!(oneval.format(&scenars), "{E1=a|b:1, E1=c|d:1, }",);

        // Splitting on an independent variable
        let when =
            WhenClauseScenario::new(&mut scenars, Ustr::from("E2"), 1, 3);
        let context3 = context.push(&mut scenars, when).unwrap();
        oneval.split(&context3, &mut scenars);
        assert_eq!(
            oneval.format(&scenars),
            "{E1=a|b,E2=e:1, E1=a|b,E2=f:1, E1=c|d,E2=e:1, E1=c|d,E2=f:1, }",
        );

        Ok(())
    }
}
