pub struct VarValue {
    name: String,
    value: Option<String>, // If None, all values allowed
}

/// Describes the context in which the parsing occurs.
/// Initially, for instance, an attribute definition is valid for all
/// combinations of scenario variables.
/// Then when we see a case-statement, we create multiple new scenario sets.
/// For instance:
///   project P is
///       // ScenarioSet =>   []   // no restriction
///       case Mode is
///          when "Debug" =>
///             // ScenarioSet => [{"Mode"="Debug"}]
///             case Optimize is
///                 when "off" =>  [{"Mode"="Debug"}, {"Optimize"="off"}]
///             ...
///      end case;
pub struct Scenario {
    vars: Vec<VarValue>,
}

impl Scenario {}

pub struct Expression<'a, T> {
    value: Vec<(&'a Scenario, T)>,
}

impl<'a, T> Expression<'a, T> {
    pub fn get(&self, scenario: &Scenario) -> Option<T> {
        None
    }
}
