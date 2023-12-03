use ustr::{Ustr, UstrSet};

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
    use ustr::{Ustr, UstrSet};

    /// Mostly intended for tests: builds a set of strings
    pub fn build_set(values: &[&str]) -> UstrSet {
        let mut s = UstrSet::default();
        s.extend(values.iter().map(|v| Ustr::from(v)));
        s
    }
}
