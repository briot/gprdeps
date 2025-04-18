use crate::{perscenario::PerScenario, scenarios::Scenario, values::ExprValue};
use itertools::join;
use ustr::Ustr;

pub struct ScenarioVariable {
    name: Ustr,
    valid: Vec<(Ustr, Scenario)>,

    // bitmask set to 1 for all entries in valid, and all other variables
    // unset.  This is used for case statements to resolve "others".
    full_mask: Scenario,

    // The value used for an "external()" function for this variable.  The
    // value has one Str value per valid value for the scenario, for instance:
    //     type T is ("on", "off"0;
    //     E1 : T := external("e1")
    // then the value for E1 is
    //     {"E1=on": "on", "E1=off": "off"}
    value: ExprValue,

    /// The default value. (i.e. the second argument for `external()`).  Note
    /// that if there is any external that doesn't declare a default, we
    /// assume there is no default.
    default: Option<Ustr>,
}

impl PartialEq for ScenarioVariable {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for ScenarioVariable {}

impl ScenarioVariable {
    /// Create a new scenario variable and its list of valid values.
    /// The list of values must be sorted, so that we can easily compare two
    /// such lists in the future.
    pub fn new(
        name: Ustr,
        valid: Vec<(Ustr, Scenario)>,
        full_mask: Scenario,
        default: Option<Ustr>,
    ) -> Self {
        let value =
            ExprValue::Str(PerScenario::new_with_variable(full_mask, &valid));
        ScenarioVariable {
            name,
            valid,
            full_mask,
            value,
            default,
        }
    }

    /// Unset the default for the variable.
    pub fn unset_default(&mut self) {
        self.default = None;
    }

    pub fn value(&self) -> &ExprValue {
        &self.value
    }

    /// Check whether this variable has the exact same set of valid values.
    /// The list of values must be sorted.
    pub fn has_same_valid(&self, valid: &[Ustr]) -> bool {
        matches!(
            valid.iter().cmp(self.valid.iter().map(|(val, _)| val)),
            std::cmp::Ordering::Equal,
        )
    }

    /// The number of valid values for this variable
    pub fn count_valid(&self) -> usize {
        self.valid.len()
    }

    /// Describe the mask using the actual values (for debug purposes)
    pub fn describe(&self, scenario: Scenario) -> String {
        if (scenario & self.full_mask) == self.full_mask {
            String::new()
        } else {
            format!(
                "{}={}",
                self.name,
                join(
                    // ??? Creates useless temporary string
                    self.valid.iter().filter_map(|(name, mask)| {
                        if scenario & mask != Scenario::empty() {
                            Some(name.as_str())
                        } else {
                            None
                        }
                    }),
                    "|",
                )
            )
        }
    }

    /// The name of the variable
    pub fn name(&self) -> &Ustr {
        &self.name
    }

    /// The mask for one specific value of the variable
    pub fn mask(&self, value: &Ustr) -> Scenario {
        match self.valid.iter().find(|(val, _)| val == value) {
            None => Scenario::empty(),
            Some(item) => item.1,
        }
    }

    pub fn full_mask(&self) -> Scenario {
        self.full_mask
    }

    /// Display the variable, its list of values, and the default
    pub fn describe_var(&self) -> String {
        format!(
            "{}: ({})",
            self.name,
            join(
                self.valid.iter().map(|v| match self.default {
                    Some(d) if d == v.0 => format!("*{}*", v.0),
                    _ => v.0.to_string(),
                }),
                ", "
            ),
        )
    }
}

impl std::fmt::Display for ScenarioVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
