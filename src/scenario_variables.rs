use ustr::Ustr;

#[derive(Eq, PartialEq)]
pub struct ScenarioVariable {
    name: Ustr,
    valid: Vec<(Ustr, u64)>,

    // bitmask set to 1 for all entries in valid, and all other variables
    // unset.  This is used for case statements to resolve "others".
    full_mask: u64,
}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted, so that we can easily compare two
    /// such lists in the future.
    pub fn new(name: Ustr, valid: Vec<(Ustr, u64)>) -> Self {
        let full_mask = 2_u64.pow(valid.len() as u32) - 1;
        ScenarioVariable {
            name,
            valid,
            full_mask,
        }
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &[(Ustr, u64)]) -> bool {
        self.valid == valid
    }

    /// Show the list of valid values (sorted alphabetically)
    pub fn list_valid(&self) -> &[(Ustr, u64)] {
        &self.valid
    }

    /// The name of the variable
    pub fn name(&self) -> &Ustr {
        &self.name
    }

    /// The mask for one specific value of the variable
    pub fn mask(&self, value: &Ustr) -> u64 {
        for (val, mask) in self.valid.iter() {
            if val == value {
                return *mask;
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
