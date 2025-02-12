use ustr::Ustr;

#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: Ustr,
    valid: Vec<Ustr>,

    // bitmask set to 1 for all entries in valid
    full_mask: u64,
}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted, so that we can easily compare two
    /// such lists in the future.
    pub fn new(name: Ustr, valid: &[Ustr]) -> Self {
        ScenarioVariable {
            name,
            valid: valid.into(),
            full_mask: 2_u64.pow(valid.len() as u32) - 1,
        }
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &[Ustr]) -> bool {
        self.valid == valid
    }

    /// Show the list of valid values (sorted alphabetically)
    pub fn list_valid(&self) -> &[Ustr] {
        &self.valid
    }

    /// The name of the variable
    pub fn name(&self) -> &Ustr {
        &self.name
    }

    /// The mask for one specific value of the variable
    pub fn mask(&self, value: &Ustr) -> u64 {
        for (idx, val) in self.valid.iter().enumerate() {
            if val == value {
                return 2_u64.pow(idx as u32);
            }
        }
        0
    }

    pub fn full_mask(&self) -> u64 {
        self.full_mask
    }
}

impl std::hash::Hash for ScenarioVariable {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.name.hash(state);
    }
}

impl std::fmt::Display for ScenarioVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/**
 * So that we can have a set of ScenarioVariables, and lookup by name
 */
impl std::borrow::Borrow<Ustr> for ScenarioVariable {
    fn borrow(&self) -> &Ustr {
        &self.name
    }
}
