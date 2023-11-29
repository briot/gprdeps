use ustr::{Ustr, UstrSet};

/// Used in handling `case` statements.  This represents the remaining valid
/// values for a variable, so that we can compute `others`.
pub struct VariableContext {
    remaining: UstrSet,
}

impl VariableContext {
    /// Called when seeing a `when` clause, to remove the corresponding state
    /// from the set of remaining values.
    pub fn when(&mut self, value: Ustr) {
        self.remaining.remove(&value);
    }

    /// Return the list of remaining unmatch values, corresponding to a
    /// `when others` clause
    pub fn when_others(&self) -> &UstrSet {
        &self.remaining
    }
}

#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: Ustr,
    valid: UstrSet,
}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted.
    pub fn new(name: Ustr, valid: UstrSet) -> Self {
        ScenarioVariable { name, valid }
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &UstrSet) -> bool {
        if valid.len() != self.valid.len() {
            return false;
        }
        for v in valid {
            if !self.valid.contains(v) {
                return false;
            }
        }
        true
    }

    /// Show the list of valid values (sorted alphabetically)
    pub fn list_valid(&self) -> &UstrSet {
        &self.valid
    }

    /// Create a new context for a `case` statement
    pub fn start_case_stmt(&self) -> VariableContext {
        VariableContext {
            remaining: self.valid.clone(),
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
pub mod tests {
    use crate::scenario_variables::ScenarioVariable;
    use ustr::{Ustr, UstrSet};

    /// Mostly intended for tests: builds a set of strings
    pub fn build_set(values: &[&str]) -> UstrSet {
        let mut s = UstrSet::default();
        s.extend(values.iter().map(|v| Ustr::from(v)));
        s
    }

    #[test]
    fn when_clauses() {
        let v = ScenarioVariable::new(
            Ustr::from("MODE"),
            build_set(&["v1", "v2", "v3"]),
        );

        assert_eq!(*v.list_valid(), build_set(&["v3", "v1", "v2"]));
        assert!(!v.has_same_valid(&build_set(&["v1"])));

        let mut c = v.start_case_stmt();
        assert_eq!(*c.when_others(), build_set(&["v3", "v1", "v2"]));
        c.when(Ustr::from("v2"));
        assert_eq!(*c.when_others(), build_set(&["v3", "v1"]));
        c.when(Ustr::from("v1"));
        c.when(Ustr::from("v3"));
        assert_eq!(*c.when_others(), build_set(&[]));
    }
}
