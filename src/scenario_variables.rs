use std::collections::HashSet;

/// Mostly intended for tests: builds a set of strings
pub fn build_set(values: &[&str]) -> HashSet<String> {
    let mut s = HashSet::new();
    for v in values {
        s.insert(v.to_string());
    }
    s
}

/// Used in handling `case` statements.  This represents the remaining valid
/// values for a variable, so that we can compute `others`.
pub struct VariableContext<'a> {
    remaining: HashSet<&'a str>,
}

impl<'a> VariableContext<'a> {
    /// Called when seeing a `when` clause, to remove the corresponding state
    /// from the set of remaining values.
    pub fn when(&mut self, value: &str) {
        self.remaining.remove(value);
    }

    /// Return the list of remaining unmatch values, corresponding to a
    /// `when others` clause
    pub fn when_others(&self) -> &HashSet<&'a str> {
        &self.remaining
    }
}

#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: String,
    valid: HashSet<String>,
}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted.
    pub fn new(name: &str, valid: &[&str]) -> Self {
        ScenarioVariable {
            name: name.to_owned(),
            valid: valid
                .iter()
                .map(|s| s.to_string())
                .collect()
        }
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &[&str]) -> bool {
        if valid.len() != self.valid.len() {
            return false;
        }
        for v in valid {
            if !self.valid.contains(*v) {
                return false;
            }
        }
        true
    }

    /// Show the list of valid values (sorted alphabetically)
    pub fn list_valid(&self) -> &HashSet<String> {
        &self.valid
    }

    /// Create a new context for a `case` statement
    pub fn start_case_stmt(&self) -> VariableContext {
        VariableContext {
            remaining: self.valid.iter().map(|s| s.as_str()).collect(),
        }
    }
}

impl std::hash::Hash for ScenarioVariable {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.name.hash(state)
    }
}

impl std::fmt::Display for ScenarioVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use crate::scenario_variables::{build_set, ScenarioVariable};
    use std::collections::HashSet;

    fn build_str_set<'a>(values: &[&'a str]) -> HashSet<&'a str> {
        values.iter().copied().collect::<HashSet<&str>>()
    }

    #[test]
    fn when_clauses() {
        let v = ScenarioVariable::new("MODE", &["v1", "v2", "v3"]);

        assert_eq!(*v.list_valid(), build_set(&["v3", "v1", "v2"]));
        assert!(!v.has_same_valid(&["v1"]));

        let mut c = v.start_case_stmt();
        assert_eq!(*c.when_others(), build_str_set(&["v3", "v1", "v2"]));
        c.when("v2");
        assert_eq!(*c.when_others(), build_str_set(&["v3", "v1"]));
        c.when("v1");
        c.when("v3");
        assert_eq!(*c.when_others(), build_str_set(&[]));
    }
}
