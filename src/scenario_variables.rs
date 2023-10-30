#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: String,
    valid: Vec<String>,   // always sorted
}

impl ScenarioVariable {

    /// Create a new scenario variable and its list of valid values
    pub fn new(name: &str, valid: Vec<&str>) -> Self {
        let mut vs: Vec<String> = valid.iter().map(|s| s.to_string()).collect();
        vs.sort();
        ScenarioVariable {
           name: name.to_owned(),
           valid: vs,
        }
    }

    /// Check whether this variable has the exact same set of valid values.
    pub fn has_same_valid(&self, valid: &[&str]) -> bool {
        let mut v = valid.to_vec();
        v.sort();
        v == self.valid
    }

    /// Show the list of valid values
    pub fn list_valid(&self) -> String {
        self.valid.join(", ")
}
}

impl std::hash::Hash for ScenarioVariable {
    fn hash<H>(&self, state: &mut H)
       where H: std::hash::Hasher
    {
        self.name.hash(state)
    }
}

impl std::fmt::Display for ScenarioVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}


