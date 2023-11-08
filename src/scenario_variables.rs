use std::collections::HashSet;

#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: String,
    valid: HashSet<String>, // always sorted
}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted.
    pub fn new(name: &str, valid: &HashSet<String>) -> Self {
        ScenarioVariable {
            name: name.to_owned(),
            valid: valid.to_owned(),
        }
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &HashSet<String>) -> bool {
        *valid == self.valid
    }

    /// Show the list of valid values (sorted alphabetically)
    pub fn list_valid(&self) -> &HashSet<String> {
        &self.valid
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
