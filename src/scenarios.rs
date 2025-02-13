//! All scenario variables are stored as a bitmask.
//! It includes all variables (new ones could be discovered later, after a
//! specific scenario has been created).
//! For instance, if the first variable has 3 possible values, the second
//! variable has 2 possible values, and so on, scenarios will be a bitmask
//! like:
//!     [0 1 1][0 1][0 0 ....]

type Mask = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Scenario(Mask);

impl Default for Scenario {
    /// The default value is a scenario that applies to all values for all
    /// variables.
    fn default() -> Self {
        Scenario(Mask::MAX)
    }
}

impl std::fmt::Display for Scenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s{}", self.0)
    }
}

impl Scenario {
    /// A scenario that never applies
    pub fn empty() -> Self {
        Scenario(0)
    }

    /// True if the scenario never applies for any of the variables.
    /// Note that there are cases where a scenario might not apply because one
    /// of the variable has no matching value, but this won't be detected.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

pub struct ScenarioFactory {
    next_mask: Mask,
}

impl Default for ScenarioFactory {
    fn default() -> Self {
        ScenarioFactory { next_mask: 1 }
    }
}

impl ScenarioFactory {
    pub fn get_next(&mut self) -> Scenario {
        let s = Scenario(self.next_mask);
        self.next_mask *= 2;
        s
    }
}

/// Compute the intersection of two scenarios, i.e. all combinations where both
/// apply.
impl ::core::ops::BitAnd<Scenario> for Scenario {
    type Output = Scenario;

    fn bitand(self, rhs: Scenario) -> Self::Output {
        Scenario(self.0 & rhs.0)
    }
}

impl ::core::ops::BitAnd<&Scenario> for Scenario {
    type Output = Scenario;

    fn bitand(self, rhs: &Scenario) -> Self::Output {
        Scenario(self.0 & rhs.0)
    }
}

impl ::core::ops::BitOr<Scenario> for Scenario {
    type Output = Scenario;

    fn bitor(self, rhs: Scenario) -> Self::Output {
        Scenario(self.0 | rhs.0)
    }
}

impl ::core::ops::BitOr<Scenario> for &Scenario {
    type Output = Scenario;

    fn bitor(self, rhs: Scenario) -> Self::Output {
        Scenario(self.0 | rhs.0)
    }
}

impl ::core::ops::Not for Scenario {
    type Output = Scenario;

    fn not(self) -> Self::Output {
        Scenario(!self.0)
    }
}
